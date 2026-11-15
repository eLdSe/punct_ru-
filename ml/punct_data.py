"""Russian punctuation + case restoration: dataset preparation (stdlib only).

Any well-punctuated text is a free training example: strip punctuation, lowercase,
and the removed marks become per-word labels for two heads (punctuation, case).

Usage:
    python punct_data.py lenta.csv wiki.jsonl --out data/ --text-field text
"""

import argparse
import csv
import gzip
import json
import random
import re
from collections import Counter
from pathlib import Path

PUNCT_LABELS = ["O", "COMMA", "PERIOD", "QUESTION", "EXCLAM"]
PUNCT_CHARS = ["", ",", ".", "?", "!"]
CASE_LABELS = ["LOWER", "CAP", "UPPER"]
IGNORE = -100  # label for sub-tokens/special tokens, skipped by the loss

# Word = number with inner separator ("3,5", "1.000") or letters/digits joined by hyphens ("кто-то").
# The C# app must split user text with the same pattern.
WORD_RE = re.compile(r"\d+(?:[.,]\d+)+|\w+(?:-\w+)*")
_ALLOWED_RE = re.compile(
    r"[\w\s,.?!-]+"
)  # anything else (: ; — ( ) …) -> sentence skipped
_ABBREV_RE = re.compile(
    r"\.\s*[a-zа-яё]|\b[A-ZА-ЯЁ]\."
)  # "т.е.", "г. москва", initials "А. С."
_QUOTES = str.maketrans("", "", '«»"„“”')
_SENT_SPLIT_RE = re.compile(r"(?<=[.?!])\s+")


def normalize_word(word: str) -> str:
    """Model input form of a word. Must match the C# side exactly."""
    return word.lower().replace("ё", "е")


def _punct_label(gap: str):
    if "-" in gap:  # hyphen used as a dash
        return None
    if "?" in gap:
        return 3
    if "!" in gap:
        return 4
    if "." in gap:
        return 2
    if "," in gap:
        return 1
    return 0


def _case_label(word: str) -> int:
    if len(word) > 1 and word.isupper():
        return 2
    return 1 if word[0].isupper() else 0


def parse_sentence(sentence: str):
    """Return [(word, punct, case), ...] or None if the sentence has unsupported/ambiguous marks."""
    s = sentence.translate(_QUOTES).strip()
    if (
        not s
        or s[-1] not in ".?!"
        or not (s[0].isupper() or s[0].isdigit())
        or "_" in s
        or not _ALLOWED_RE.fullmatch(s)
        or _ABBREV_RE.search(s)
    ):
        return None
    matches = list(WORD_RE.finditer(s))
    if not matches or s[: matches[0].start()].strip():
        return None
    out = []
    for i, m in enumerate(matches):
        gap_end = matches[i + 1].start() if i + 1 < len(matches) else len(s)
        punct = _punct_label(s[m.end() : gap_end])
        if punct is None:
            return None
        out.append((m.group(), punct, _case_label(m.group())))
    return out


def iter_runs(paragraph: str, max_words: int):
    """Yield runs of consecutive valid sentences (a skipped sentence breaks the run)."""
    run = []
    for sent in _SENT_SPLIT_RE.split(paragraph.translate(_QUOTES).strip()):
        parsed = parse_sentence(sent)
        if parsed and len(parsed) <= max_words:
            run.append(parsed)
        else:
            if run:
                yield run
            run = []
    if run:
        yield run


def make_chunks(run, rng: random.Random, max_words: int):
    """Group sentences into chunks of random length (1..max_words words, whole sentences only)."""
    chunk, target = [], rng.randint(1, max_words)
    for sent in run:
        if chunk and len(chunk) + len(sent) > max_words:
            yield chunk
            chunk, target = [], rng.randint(1, max_words)
        chunk.extend(sent)
        if len(chunk) >= target:
            yield chunk
            chunk, target = [], rng.randint(1, max_words)
    if chunk:
        yield chunk


def to_example(chunk) -> dict:
    return {
        "words": [normalize_word(w) for w, _, _ in chunk],
        "punct": [p for _, p, _ in chunk],
        "case": [c for _, _, c in chunk],
    }


def apply_labels(words, punct, case) -> str:
    """Inverse operation: user's words + predicted labels -> text. Reference for the C# side."""
    out = []
    for w, p, c in zip(words, punct, case):
        if c == 2:
            w = w.upper()
        elif c == 1:
            w = w[:1].upper() + w[1:]
        out.append(w + PUNCT_CHARS[p])
    return " ".join(out)


def align_labels(word_ids, labels):
    """Map word-level labels onto tokens: first sub-token gets the label, the rest IGNORE.

    word_ids: output of a HF fast tokenizer `encoding.word_ids()` (None for special tokens).
    """
    out, prev = [], None
    for wid in word_ids:
        out.append(IGNORE if wid is None or wid == prev else labels[wid])
        prev = wid
    return out


def clean_subtitles_line(line: str) -> list[str]:
    """Clean subtitle lines: strip tags, bracketed sounds, dashes, drop broken ellipses."""
    line = re.sub(r"<[^>]+>", "", line)
    line = re.sub(r"\[.*?\]|\(.*?\)", "", line)
    if "..." in line or "…" in line:
        return []
    out = []
    for part in re.split(r"(?:^|\s)[-—]\s+", line):
        part = part.strip()
        if part:
            out.append(part)
    return out


def iter_documents(path: Path, text_field: str, is_subtitles: bool = False):
    if is_subtitles:
        open_fn = (
            (lambda: gzip.open(path, "rt", encoding="utf-8"))
            if path.name.endswith(".gz")
            else (lambda: path.open(encoding="utf-8"))
        )
        with open_fn() as f:
            for line in f:
                for cleaned in clean_subtitles_line(line):
                    yield cleaned
    elif path.suffix == ".csv" or path.name.endswith(".csv.gz"):
        csv.field_size_limit(2**31 - 1)
        open_fn = (
            (lambda: gzip.open(path, "rt", encoding="utf-8", newline=""))
            if path.name.endswith(".gz")
            else (lambda: path.open(encoding="utf-8", newline=""))
        )
        with open_fn() as f:
            for row in csv.DictReader(f):
                yield row.get(text_field) or ""
    elif path.suffix == ".jsonl":
        with path.open(encoding="utf-8") as f:
            for line in f:
                yield json.loads(line).get(text_field) or ""
    else:  # plain text: one document per line
        open_fn = (
            (lambda: gzip.open(path, "rt", encoding="utf-8"))
            if path.name.endswith(".gz")
            else (lambda: path.open(encoding="utf-8"))
        )
        with open_fn() as f:
            yield from f


def build_dataset(
    inputs,
    out_dir: Path,
    text_field="text",
    max_words=64,
    val=0.01,
    test=0.01,
    seed=42,
    max_examples=None,
    is_subtitles: bool = False,
) -> dict:
    """Stream documents -> train/val/test JSONL. Split is per document to avoid leakage."""
    rng = random.Random(seed)
    out_dir.mkdir(parents=True, exist_ok=True)
    files = {
        k: (out_dir / f"{k}.jsonl").open("w", encoding="utf-8")
        for k in ("train", "val", "test")
    }
    counts, punct_stats, case_stats = Counter(), Counter(), Counter()
    try:
        for path in inputs:
            for doc in iter_documents(
                Path(path), text_field, is_subtitles=is_subtitles
            ):
                r = rng.random()
                split = "val" if r < val else "test" if r < val + test else "train"
                for paragraph in doc.splitlines():
                    for run in iter_runs(paragraph, max_words):
                        for chunk in make_chunks(run, rng, max_words):
                            ex = to_example(chunk)
                            files[split].write(
                                json.dumps(ex, ensure_ascii=False) + "\n"
                            )
                            counts[split] += 1
                            punct_stats.update(PUNCT_LABELS[p] for p in ex["punct"])
                            case_stats.update(CASE_LABELS[c] for c in ex["case"])
                            if max_examples and sum(counts.values()) >= max_examples:
                                return {
                                    "examples": dict(counts),
                                    "punct": dict(punct_stats),
                                    "case": dict(case_stats),
                                }
    finally:
        for f in files.values():
            f.close()
    return {
        "examples": dict(counts),
        "punct": dict(punct_stats),
        "case": dict(case_stats),
    }


def main():
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    ap.add_argument(
        "inputs",
        nargs="+",
        help=".csv / .jsonl (uses --text-field) or .txt (one doc per line)",
    )
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--text-field", default="text")
    ap.add_argument("--max-words", type=int, default=64)
    ap.add_argument("--val", type=float, default=0.01)
    ap.add_argument("--test", type=float, default=0.01)
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--max-examples", type=int)
    args = ap.parse_args()
    stats = build_dataset(
        args.inputs,
        args.out,
        args.text_field,
        args.max_words,
        args.val,
        args.test,
        args.seed,
        args.max_examples,
    )
    print(json.dumps(stats, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
