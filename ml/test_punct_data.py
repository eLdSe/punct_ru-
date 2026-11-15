import json
import random
import tempfile
import unittest
from pathlib import Path

from punct_data import (IGNORE, align_labels, apply_labels, build_dataset, iter_runs,
                        make_chunks, normalize_word, parse_sentence)

EVAL_FILE = Path(__file__).parent / "eval" / "conversational_ru.txt"


def labels(sentence):
    parsed = parse_sentence(sentence)
    return None if parsed is None else [(w, p, c) for w, p, c in parsed]


class ParseSentence(unittest.TestCase):
    def test_basic_labels(self):
        self.assertEqual(labels("Привет, Саша! Я звонил."),
                         [("Привет", 1, 1), ("Саша", 4, 1), ("Я", 0, 1), ("звонил", 2, 0)])
        self.assertEqual(labels("Я звонил, но ты не ответил."), [
            ("Я", 0, 1), ("звонил", 1, 0), ("но", 0, 0), ("ты", 0, 0), ("не", 0, 0), ("ответил", 2, 0)])

    def test_question_exclam_priority(self):
        self.assertEqual(labels("Ты где?!")[-1][1], 3)
        self.assertEqual(labels("Ура!!!")[-1][1], 4)
        self.assertEqual(labels("Ну и дела...")[-1][1], 2)

    def test_case_labels(self):
        self.assertEqual([c for _, _, c in labels("В США и Москве тепло.")], [1, 2, 0, 1, 0])

    def test_numbers_and_hyphens_are_single_words(self):
        self.assertEqual([w for w, _, _ in labels("Кто-то купил 3,5 кг.")], ["Кто-то", "купил", "3,5", "кг"])

    def test_quotes_are_stripped(self):
        self.assertEqual([w for w, _, _ in labels("Он читал «Войну и мир».")], ["Он", "читал", "Войну", "и", "мир"])

    def test_rejects_unsupported(self):
        for s in ["Москва — столица.", "Итак: всё.", "Он пришёл; она ушла.", "Дом (новый).",
                  "Это, т.е. всё.", "Писал А. С. Пушкин.", "Он пришёл - и ушёл.", "без заглавной.",
                  "Без точки", "Ссылка my_var.", "Он сказал д'Артаньян."]:
            self.assertIsNone(parse_sentence(s), s)


class RunsAndChunks(unittest.TestCase):
    def test_invalid_sentence_breaks_run(self):
        runs = list(iter_runs("Я дома. Москва — столица. Ты где? Иду.", max_words=64))
        self.assertEqual([len(r) for r in runs], [1, 2])

    def test_long_sentence_dropped(self):
        long = "Я " + "очень " * 70 + "устал."
        self.assertEqual(list(iter_runs(long, max_words=64)), [])

    def test_chunks_respect_limit_and_keep_all_words(self):
        run = list(iter_runs("Я дома. Ты где? Мы идём в кино, а потом в кафе. Пока!" * 20, 64))[0]
        chunks = list(make_chunks(run, random.Random(0), max_words=10))
        self.assertTrue(all(len(c) <= 10 for c in chunks))
        self.assertEqual(sum(map(len, chunks)), sum(map(len, run)))


class Subtitles(unittest.TestCase):
    def test_clean_subtitles_line(self):
        from punct_data import clean_subtitles_line
        self.assertEqual(clean_subtitles_line("- Привет! Как дела?"), ["Привет! Как дела?"])
        self.assertEqual(clean_subtitles_line("- Ты где? - Я дома."), ["Ты где?", "Я дома."])
        self.assertEqual(clean_subtitles_line("<i>[Музыка]</i> Я иду к тебе."), ["Я иду к тебе."])
        self.assertEqual(clean_subtitles_line("Я думаю... что это ошибка."), [])


class Labels(unittest.TestCase):
    def test_normalize(self):
        self.assertEqual(normalize_word("Ёлка"), "елка")

    def test_align_labels(self):
        # [CLS] при ##вет са ##ша [SEP]
        self.assertEqual(align_labels([None, 0, 0, 1, 1, None], [1, 4]), [IGNORE, 1, IGNORE, 4, IGNORE, IGNORE])

    def test_roundtrip_on_eval_set(self):
        """Every eval line must be parseable and reconstructable from lowercase words + labels."""
        for line in EVAL_FILE.read_text(encoding="utf-8").splitlines():
            runs = list(iter_runs(line, 64))
            self.assertEqual(len(runs), 1, line)
            words = [w for sent in runs[0] for w, _, _ in sent]
            punct = [p for sent in runs[0] for _, p, _ in sent]
            case = [c for sent in runs[0] for _, _, c in sent]
            self.assertEqual(apply_labels([w.lower() for w in words], punct, case), line)


class BuildDataset(unittest.TestCase):
    def test_end_to_end_csv(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "corpus.csv"
            src.write_text('title,text\nA,"Привет, мир! Как дела?\nВсё хорошо."\nB,"Москва — столица."\n',
                           encoding="utf-8")
            stats = build_dataset([src], Path(tmp) / "out", val=0, test=0)
            rows = [json.loads(l) for l in (Path(tmp) / "out" / "train.jsonl").read_text(encoding="utf-8").splitlines()]
            words = [w for r in rows for w in r["words"]]
            self.assertEqual(words, ["привет", "мир", "как", "дела", "все", "хорошо"])
            self.assertTrue(all(len(r["words"]) == len(r["punct"]) == len(r["case"]) for r in rows))
            self.assertEqual(stats["examples"]["train"], len(rows))


if __name__ == "__main__":
    unittest.main()
