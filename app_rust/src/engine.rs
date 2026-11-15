use std::path::Path;
use std::sync::Arc;
use regex::Regex;
use tokenizers::Tokenizer;
use ort::environment::Environment;
use ort::session::Session;

pub const PUNCT_CHARS: [&str; 5] = ["", ",", ".", "?", "!"];

pub const LOWERCASE_AFTER_COMMA: &[&str] = &[
    "я", "мы", "ты", "вы", "он", "она", "оно", "они",
    "мой", "моя", "мое", "мои", "твой", "твоя", "твое", "твои",
    "наш", "наша", "наше", "наши", "ваш", "ваша", "ваше", "ваши",
    "свой", "своя", "свое", "свои", "его", "ее", "их",
    "этот", "эта", "это", "эти", "тот", "та", "то", "те",
    "такой", "такая", "такое", "такие", "весь", "вся", "все", "всего", "всех",
    "каждый", "каждая", "каждое", "каждые", "сам", "сама", "само", "сами",
    "самый", "самая", "самое", "самые", "любой", "любая", "любое", "любые",
    "другой", "другая", "другое", "другие", "иной",
    "кто", "что", "какой", "какая", "какое", "какие", "чей", "чья", "чье", "чьи",
    "который", "которая", "которое", "которые", "которого", "которой", "которых", "которому", "которым",
    "сколько", "столько", "несколько",
    "никто", "ничто", "никакой", "ничей",
    "и", "а", "но", "да", "или", "либо", "то", "чтобы", "чтоб", "как", "когда", "если",
    "хотя", "будто", "словно", "точно", "потому", "поэтому", "причем", "притом",
    "же", "ли", "бы", "не", "ни", "ведь", "вот", "даже", "лишь", "только",
    "однако", "конечно", "обязательно", "вообще", "наверное", "видимо", "кстати",
    "действительно", "сейчас", "потом", "уже", "еще", "всегда", "никогда", "где", "куда", "откуда",
    "так", "также", "тоже", "зато", "оттого", "затем", "пока", "едва",
];

pub const PROPER_NOUNS: &[&str] = &[
    // Мужские имена и популярные формы
    "эльдар", "эльдара", "эльдару", "эльдаром",
    "александр", "александра", "александру", "саша", "саня", "шура",
    "алексей", "алексея", "алексею", "леша", "леха",
    "дмитрий", "дмитрия", "дмитрию", "дима", "димон",
    "максим", "максима", "максиму", "макс",
    "иван", "ивана", "ивану", "ваня",
    "артем", "артема", "артему", "тема",
    "сергей", "сергея", "сергею", "сережа", "серега",
    "андрей", "андрея", "андрею", "андрюха",
    "николай", "николая", "николаю", "коля",
    "михаил", "михаила", "михаилу", "миша",
    "евгений", "евгения", "евгению", "женя",
    "владимир", "владимира", "владимиру", "вова", "володя",
    "владислав", "владислава", "влад",
    "вячеслав", "вячеслава", "слава",
    "станислав", "станислава", "стас",
    "ярослав", "ярослава", "ярик",
    "святослав", "ростислав", "мирослав",
    "денис", "дениса", "денису", "ден",
    "павел", "павла", "павлу", "паша",
    "роман", "романа", "роману", "рома",
    "илья", "ильи", "илье", "илюха",
    "кирилл", "кирилла", "кириллу", "киря",
    "никита", "никиты", "никите",
    "даниил", "даниила", "данила", "данил", "даня",
    "тимур", "тимура", "тимуру",
    "руслан", "руслана", "руслану",
    "игорь", "игоря", "игорю",
    "олег", "олега", "олегу",
    "виктор", "виктора", "виктору", "витя",
    "юрий", "юрия", "юрию", "юра",
    "антон", "антона", "антону", "тоха",
    "константин", "константина", "костя",
    "георгий", "георгия", "гоша", "жора",
    "борис", "бориса", "боря",
    "леонид", "леонида", "леня",
    "василий", "василия", "вася",
    "анатолий", "анатолия", "толя",
    "валерий", "валерия", "валера",
    "григорий", "григория", "гриша",
    "матвей", "матвея", "мот",
    "тимофей", "тимофея", "тима",
    "глеб", "глеба", "арсений", "арсения", "семен", "семена",
    "федор", "федора", "федя", "степан", "степана", "степа",
    "марк", "марка", "лев", "льва", "макар", "захар", "егор", "егора",
    "вадим", "вадима", "виталий", "виталия", "артур", "артура",
    "марат", "марата", "ринат", "рината", "дамир", "дамира", "карим",
    // Женские имена и популярные формы
    "анна", "анны", "анне", "аня", "анюта",
    "мария", "марии", "маша", "маруся",
    "елена", "елены", "лена",
    "ольга", "ольги", "оля",
    "наталья", "натальи", "наташа",
    "екатерина", "екатерины", "катя", "катюша",
    "татьяна", "таani", "татьяны", "таня",
    "ирина", "ирины", "ира",
    "светлана", "светланы", "света",
    "юлия", "юлии", "юля",
    "анастасия", "анастасии", "настя",
    "дарья", "дарьи", "даша",
    "алина", "алины", "полина", "полины", "поля",
    "ксения", "ксении", "ксюша",
    "виктория", "виктории", "вика",
    "марина", "марины", "кристина", "кристины",
    "людмила", "людмилы", "люда", "галина", "галины", "галя",
    "валерия", "валерии", "лера", "вероника", "вероники", "ника",
    "диана", "дианы", "софия", "софии", "софья", "соня",
    "алиса", "алисы", "ева", "евы", "милана", "миланы",
    "алла", "инна", "нина", "вера", "надежда", "любовь", "оксана",
    "яana", "яна", "яны", "карина", "карины", "альбина", "элина", "камилла",
    // Города, страны и сервисы
    "москва", "москвы", "москве", "москву", "москвой",
    "санкт-петербург", "санкт-петербурга", "санкт-петербурге", "петербург", "питер", "питере", "спб",
    "новосибирск", "новосибирске", "екатеринбург", "екатеринбурге",
    "казань", "казани", "сочи", "краснодар", "краснодаре",
    "самара", "самаре", "уфа", "уфе", "омск", "омске", "челябинск", "ростов", "воронеж", "пермь", "волгоград",
    "россия", "россии", "россию", "россией", "рф",
    "сша", "китай", "китае", "турция", "турции", "германия", "германии", "франция", "европа", "европе",
    "телеграм", "телеграме", "ватсап", "ватсапе", "ютуб", "ютубе", "гугл", "яндекс", "авито", "озон", "вайлдберриз",
];

pub const VOCATIVE_TRIGGERS: &[&str] = &[
    "привет", "здравствуй", "здравствуйте", "спасибо", "благодарю", "пока", "прощай", "алло", "эй",
    "уважаемый", "уважаемая", "дорогой", "дорогая", "милый", "милая", "слушай", "слушайте", "смотри", "смотрите",
];

pub struct PunctEngine {
    tokenizer: Tokenizer,
    _env: Arc<Environment>,
    session: Session,
    word_re: Regex,
}

impl PunctEngine {
    pub fn from_memory(model_bytes: &[u8], tokenizer_bytes: &[u8], dll_path: Option<impl AsRef<Path>>) -> Result<Self, Box<dyn std::error::Error>> {
        let tokenizer = Tokenizer::from_bytes(tokenizer_bytes)
            .map_err(|e| format!("Failed to load tokenizer: {e}"))?;

        let env_builder = match dll_path {
            Some(p) => ort::init_from(p.as_ref().to_str().unwrap())?,
            None => ort::init(),
        };
        let env = Arc::new(env_builder.with_name("PunctEngine").build()?);

        let session = Session::builder(&env)?
            .with_intra_threads(1)?
            .commit_from_memory(model_bytes)?;

        let word_re = Regex::new(r"\d+(?:[.,]\d+)+|\w+(?:-\w+)*")?;

        Ok(Self {
            tokenizer,
            _env: env,
            session,
            word_re,
        })
    }

    pub fn new(model_path: impl AsRef<Path>, tokenizer_path: impl AsRef<Path>, dll_path: Option<impl AsRef<Path>>) -> Result<Self, Box<dyn std::error::Error>> {
        let tokenizer = Tokenizer::from_file(tokenizer_path.as_ref())
            .map_err(|e| format!("Failed to load tokenizer: {e}"))?;

        let env_builder = match dll_path {
            Some(p) => ort::init_from(p.as_ref().to_str().unwrap())?,
            None => ort::init(),
        };
        let env = Arc::new(env_builder.with_name("PunctEngine").build()?);

        let session = Session::builder(&env)?
            .with_intra_threads(1)?
            .commit_from_file(model_path.as_ref())?;

        let word_re = Regex::new(r"\d+(?:[.,]\d+)+|\w+(?:-\w+)*")?;

        Ok(Self {
            tokenizer,
            _env: env,
            session,
            word_re,
        })
    }

    pub fn normalize_word(word: &str) -> String {
        word.to_lowercase().replace('ё', "е")
    }

    pub fn restore(&mut self, text: &str, casual_mode: bool) -> Result<String, Box<dyn std::error::Error>> {
        let matches: Vec<regex::Match> = self.word_re.find_iter(text).collect();
        if matches.is_empty() {
            return Ok(text.to_string());
        }
        let words: Vec<&str> = matches.iter().map(|m| m.as_str()).collect();

        let normalized_words: Vec<String> = words.iter().map(|w| Self::normalize_word(w)).collect();
        let word_slices: Vec<&str> = normalized_words.iter().map(|s| s.as_str()).collect();

        // Tokenize with word-level alignment
        let encoding = self.tokenizer.encode(&word_slices[..], true)
            .map_err(|e| format!("Tokenizer error: {e}"))?;

        let input_ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention_mask: Vec<i64> = encoding.get_attention_mask().iter().map(|&m| m as i64).collect();
        let seq_len = input_ids.len();

        let input_ids_tensor = ort::value::Tensor::from_array(([1, seq_len], input_ids.into_boxed_slice()))?;
        let mask_tensor = ort::value::Tensor::from_array(([1, seq_len], attention_mask.into_boxed_slice()))?;

        let outputs = self.session.run(ort::inputs![
            "input_ids" => input_ids_tensor,
            "attention_mask" => mask_tensor,
        ])?;

        let (punct_shape, punct_data) = outputs["punct_logits"].try_extract_tensor::<f32>()?;
        let (case_shape, case_data) = outputs["case_logits"].try_extract_tensor::<f32>()?;

        let num_punct_classes = punct_shape[2] as usize;
        let num_case_classes = case_shape[2] as usize;

        let mut punct_labels = vec![0usize; words.len()];
        let mut case_labels = vec![0usize; words.len()];
        let mut prev_word_idx: Option<usize> = None;

        for (token_idx, word_idx_opt) in encoding.get_word_ids().iter().enumerate() {
            if let Some(word_idx) = word_idx_opt {
                let wid = *word_idx as usize;
                if Some(wid) != prev_word_idx && wid < words.len() {
                    // Argmax for punctuation
                    let p_offset = token_idx * num_punct_classes;
                    let p_slice = &punct_data[p_offset..p_offset + num_punct_classes];
                    let best_p = p_slice.iter()
                        .enumerate()
                        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    punct_labels[wid] = best_p;

                    #[cfg(debug_assertions)]
                    println!("Word: {:10} | Logits: O:{:.2} ,:{:.2} .:{:.2} ?:{:.2} !:{:.2}",
                        words[wid], p_slice[0], p_slice[1], p_slice[2], p_slice[3], p_slice[4]);

                    // Argmax for case
                    let c_offset = token_idx * num_case_classes;
                    let c_slice = &case_data[c_offset..c_offset + num_case_classes];
                    let best_c = c_slice.iter()
                        .enumerate()
                        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    case_labels[wid] = best_c;
                }
                prev_word_idx = Some(wid);
            }
        }

        // Grammatical post-processing: sentence boundary capitalization, proper nouns & comma casing
        if !case_labels.is_empty() && case_labels[0] == 0 {
            case_labels[0] = 1;
        }
        for i in 0..words.len() {
            let norm = normalized_words[i].as_str();

            // Сохраняем аббревиатуры, написанные КАПСОМ в оригинале (от 2 букв: США, РФ, API, ООО)
            let char_count = words[i].chars().count();
            let orig_all_caps = char_count >= 2
                && words[i].chars().any(|c| c.is_alphabetic())
                && words[i].chars().all(|c| !c.is_alphabetic() || c.is_uppercase());
            let is_known_acronym = matches!(
                norm,
                "рф" | "сша" | "спб" | "ссср" | "мгу" | "ооо" | "зао" | "оао" | "ип"
                    | "ндс" | "мвд" | "фсб" | "жкх" | "ржд" | "ввп" | "снг" | "оон"
                    | "нато" | "ес" | "цб" | "сми"
            );

            if orig_all_caps || is_known_acronym {
                case_labels[i] = 2;
            } else if case_labels[i] == 2 {
                // Защита от ложного КАПСА нейросети на обычных коротких словах (например "жди" -> "ЖДИ")
                case_labels[i] = if i == 0 { 1 } else { 0 };
            }

            if PROPER_NOUNS.contains(&norm) {
                if case_labels[i] == 0 {
                    case_labels[i] = 1;
                }
                // Обособление обращения запятыми: "привет эльдар как дела" -> "Привет, Эльдар, как дела?"
                if i > 0 && VOCATIVE_TRIGGERS.contains(&normalized_words[i - 1].as_str()) {
                    if punct_labels[i - 1] == 0 {
                        punct_labels[i - 1] = 1;
                    }
                    if i + 1 < words.len() && punct_labels[i] == 0 {
                        punct_labels[i] = 1;
                    }
                }
            }

            if i > 0 {
                let prev_p = punct_labels[i - 1];
                // 2: '.', 3: '?', 4: '!' -> Начало нового предложения (всегда заглавная)
                if (prev_p == 2 || prev_p == 3 || prev_p == 4) && case_labels[i] == 0 {
                    case_labels[i] = 1;
                } else if prev_p == 1 && case_labels[i] == 1 {
                    // 1: ',' -> Слово после запятой. Если это местоимение, союз или обычное слово — строчная!
                    if LOWERCASE_AFTER_COMMA.contains(&norm) && !PROPER_NOUNS.contains(&norm) {
                        case_labels[i] = 0;
                    }
                }
            }
        }

        let result = Self::reconstruct_text(text, &matches, &mut punct_labels, &case_labels, casual_mode);
        Ok(result)
    }

    pub fn reconstruct_text(
        text: &str,
        matches: &[regex::Match],
        punct_labels: &mut [usize],
        case_labels: &[usize],
        casual_mode: bool,
    ) -> String {
        let words_len = matches.len();
        if words_len == 0 {
            return text.to_string();
        }

        // Перенос запятой перед открывающей скобкой на закрывающую скобку:
        // В русской пунктуации запятая ставится ПОСЛЕ скобки, а не перед '('
        let mut pending_paren_comma = false;
        for i in 0..words_len {
            let next_start = if i + 1 < words_len { matches[i + 1].start() } else { text.len() };
            let gap = &text[matches[i].end()..next_start];
            if gap.contains('(') && punct_labels[i] == 1 {
                punct_labels[i] = 0;
                pending_paren_comma = true;
            }
            if gap.contains(')') && pending_paren_comma {
                if punct_labels[i] == 0 {
                    punct_labels[i] = 1;
                }
                pending_paren_comma = false;
            }
        }

        let mut result = String::with_capacity(text.len() + 16);

        // 1. Сохраняем открывающие символы в самом начале строки (кавычки, скобки, эмодзи)
        let prefix = &text[..matches[0].start()];
        for ch in prefix.chars() {
            if !matches!(ch, ',' | '.' | '?' | '!') {
                result.push(ch);
            }
        }

        // 2. Собираем слова и промежутки с сохранением спецсимволов
        for i in 0..words_len {
            let word = matches[i].as_str();
            let next_start = if i + 1 < words_len { matches[i + 1].start() } else { text.len() };
            let gap = &text[matches[i].end()..next_start];

            let mut p = punct_labels[i].min(PUNCT_CHARS.len() - 1);
            // Если пользователь сам явно поставил '?' или '!' в исходнике, сохраняем эту интонацию
            if gap.contains('?') && p <= 2 {
                p = 3;
            } else if gap.contains('!') && p <= 2 {
                p = 4;
            }

            let c = case_labels[i];
            let formatted_word = match c {
                2 => word.to_uppercase(),
                1 => {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => {
                            first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                        }
                    }
                }
                _ => word.to_lowercase(),
            };
            result.push_str(&formatted_word);

            // Разбираем gap на примыкающие суффиксы (%, $, ), ") и остальные символы
            let mut chars_iter = gap.chars().peekable();
            while let Some(&ch) = chars_iter.peek() {
                if matches!(ch, '%' | '$' | '€' | '₽' | '°' | ')' | ']' | '}' | '"' | '»') {
                    result.push(ch);
                    chars_iter.next();
                } else {
                    break;
                }
            }

            result.push_str(PUNCT_CHARS[p]);

            let rest_gap: String = chars_iter
                .filter(|&ch| !matches!(ch, ',' | '.' | '?' | '!'))
                .collect();

            if i + 1 < words_len {
                if rest_gap.contains('\n') {
                    for ch in rest_gap.chars() {
                        if !ch.is_whitespace() || ch == '\n' || ch == '\r' {
                            result.push(ch);
                        }
                    }
                } else {
                    let extra: String = rest_gap.chars().filter(|c| !c.is_whitespace()).collect();
                    if extra.is_empty() {
                        result.push(' ');
                    } else {
                        result.push(' ');
                        result.push_str(&extra);
                        // Если спецсимвол — не открывающая скобка/кавычка, ставим пробел перед следующим словом
                        if let Some(last_extra) = extra.chars().last() {
                            if !matches!(last_extra, '(' | '[' | '{' | '«' | '"' | '/' | '@' | '#') {
                                result.push(' ');
                            }
                        }
                    }
                }
            } else {
                // После самого последнего слова сохраняем хвост (закрывающие скобки, смайлики и т.д.)
                let extra_trimmed = rest_gap.trim();
                if !extra_trimmed.is_empty() {
                    result.push(' ');
                    result.push_str(extra_trimmed);
                }
            }
        }

        // Casual Mode (Режим мессенджера/чата):
        // Убираем только завершающую точку в самом конце фразы/сообщения,
        // сохраняя вопросительные/восклицательные знаки и внутренние точки между предложениями.
        if casual_mode {
            if let Some(pos) = result.rfind('.') {
                if result[pos + 1..].chars().all(|c| !c.is_alphanumeric() && c != '?' && c != '!') {
                    let mut s = result[..pos].to_string();
                    s.push_str(&result[pos + 1..]);
                    result = s;
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preserve_brackets_emojis_and_currency() {
        let word_re = Regex::new(r"\d+(?:[.,]\d+)+|\w+(?:-\w+)*").unwrap();
        let text = "привет (я скоро буду) жди 😊";
        let matches: Vec<regex::Match> = word_re.find_iter(text).collect();
        let mut punct = vec![1, 0, 0, 0, 2]; // запятая перед скобкой должна перенестись за скобку, точка в конце
        let case = vec![1, 0, 0, 0, 0];

        let res_casual = PunctEngine::reconstruct_text(text, &matches, &mut punct.clone(), &case, true);
        assert_eq!(res_casual, "Привет (я скоро буду), жди 😊");

        let res_formal = PunctEngine::reconstruct_text(text, &matches, &mut punct, &case, false);
        assert_eq!(res_formal, "Привет (я скоро буду), жди. 😊");
    }

    #[test]
    fn test_preserve_currency_and_percent() {
        let word_re = Regex::new(r"\d+(?:[.,]\d+)+|\w+(?:-\w+)*").unwrap();
        let text = "курс 100$ за штуку а со скидкой 20% дешевле";
        let matches: Vec<regex::Match> = word_re.find_iter(text).collect();
        let mut punct = vec![0, 0, 0, 1, 0, 0, 0, 0, 2];
        let case = vec![1, 0, 0, 0, 0, 0, 0, 0, 0];

        let res = PunctEngine::reconstruct_text(text, &matches, &mut punct, &case, true);
        assert_eq!(res, "Курс 100$ за штуку, а со скидкой 20% дешевле");
    }
}
