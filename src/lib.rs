use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const RU_WORDS: &str = include_str!("../words/ru.txt");
pub const EN_WORDS: &str = include_str!("../words/en.txt");
pub const WORDS_PER_TEST: usize = 25;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Lang {
    Ru,
    En,
}

impl Lang {
    pub fn toggle(self) -> Self {
        match self {
            Lang::Ru => Lang::En,
            Lang::En => Lang::Ru,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Lang::Ru => "RU",
            Lang::En => "EN",
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            Lang::Ru => RU_WORDS,
            Lang::En => EN_WORDS,
        }
    }

    pub fn is_foreign(self, c: char) -> bool {
        match self {
            Lang::Ru => c.is_ascii_alphabetic(),
            Lang::En => is_cyrillic(c),
        }
    }

    pub fn layout_hint(self) -> &'static str {
        match self {
            Lang::Ru => "Похоже, включена английская раскладка — переключите на русскую",
            Lang::En => "Похоже, включена русская раскладка — переключите на английскую",
        }
    }
}

pub fn is_cyrillic(c: char) -> bool {
    matches!(c, 'а'..='я' | 'А'..='Я' | 'ё' | 'Ё')
}

pub const KEYS_EN: [&str; 3] = ["qwertyuiop[]", "asdfghjkl;'", "zxcvbnm,./"];
pub const KEYS_RU: [&str; 3] = ["йцукенгшщзхъ", "фывапролджэ", "ячсмитьбю."];

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Finger {
    LPinky,
    LRing,
    LMiddle,
    LIndex,
    RIndex,
    RMiddle,
    RRing,
    RPinky,
    Thumb,
}

impl Finger {
    pub fn for_column(col: usize) -> Self {
        match col {
            0 => Finger::LPinky,
            1 => Finger::LRing,
            2 => Finger::LMiddle,
            3 | 4 => Finger::LIndex,
            5 | 6 => Finger::RIndex,
            7 => Finger::RMiddle,
            8 => Finger::RRing,
            _ => Finger::RPinky,
        }
    }

    pub fn group(self) -> usize {
        match self {
            Finger::LPinky | Finger::RPinky => 0,
            Finger::LRing | Finger::RRing => 1,
            Finger::LMiddle | Finger::RMiddle => 2,
            Finger::LIndex | Finger::RIndex => 3,
            Finger::Thumb => 4,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Finger::LPinky => "левый мизинец",
            Finger::LRing => "левый безымянный",
            Finger::LMiddle => "левый средний",
            Finger::LIndex => "левый указательный",
            Finger::RIndex => "правый указательный",
            Finger::RMiddle => "правый средний",
            Finger::RRing => "правый безымянный",
            Finger::RPinky => "правый мизинец",
            Finger::Thumb => "большой палец",
        }
    }

}

pub struct Lesson {
    pub title: &'static str,
    pub new: &'static str,
}

pub const LESSONS_RU: [Lesson; 7] = [
    Lesson { title: "Основной ряд", new: "фываолдж" },
    Lesson { title: "Основной ряд целиком", new: "прэ" },
    Lesson { title: "Верхний ряд", new: "кенг" },
    Lesson { title: "Верхний ряд целиком", new: "йцушщзхъ" },
    Lesson { title: "Нижний ряд", new: "мить" },
    Lesson { title: "Нижний ряд целиком", new: "ячсбю" },
    Lesson { title: "Все ряды — слова", new: "" },
];

pub const LESSONS_EN: [Lesson; 7] = [
    Lesson { title: "Основной ряд", new: "asdfjkl" },
    Lesson { title: "Основной ряд целиком", new: "gh" },
    Lesson { title: "Верхний ряд", new: "erui" },
    Lesson { title: "Верхний ряд целиком", new: "qwtyop" },
    Lesson { title: "Нижний ряд", new: "vbnm" },
    Lesson { title: "Нижний ряд целиком", new: "zxc" },
    Lesson { title: "Все ряды — слова", new: "" },
];

pub const LESSON_COUNT: usize = 7;

pub fn cycle_lesson(lesson: usize, forward: bool) -> usize {
    if forward {
        (lesson + 1) % LESSON_COUNT
    } else {
        (lesson + LESSON_COUNT - 1) % LESSON_COUNT
    }
}

impl Lang {
    pub fn rows(self) -> [&'static str; 3] {
        match self {
            Lang::Ru => KEYS_RU,
            Lang::En => KEYS_EN,
        }
    }

    pub fn lessons(self) -> &'static [Lesson; LESSON_COUNT] {
        match self {
            Lang::Ru => &LESSONS_RU,
            Lang::En => &LESSONS_EN,
        }
    }

    pub fn allowed(self, lesson: usize) -> Option<Vec<char>> {
        let lessons = self.lessons();
        if lessons[lesson].new.is_empty() {
            return None;
        }
        Some(lessons[..=lesson].iter().flat_map(|l| l.new.chars()).collect())
    }
}

pub fn state_path() -> Option<std::path::PathBuf> {
    if cfg!(windows) {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Some(std::path::PathBuf::from(appdata).join("fastword").join("state"));
        }
    }
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".local/state")))?;
    Some(base.join("fastword").join("state"))
}

pub fn load_state() -> (Lang, usize) {
    let text = state_path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    let mut parts = text.split_whitespace();
    let lang = if parts.next() == Some("en") { Lang::En } else { Lang::Ru };
    let lesson = parts
        .next()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&n| n < LESSON_COUNT)
        .unwrap_or(LESSON_COUNT - 1);
    (lang, lesson)
}

pub fn save_state(lang: Lang, lesson: usize) {
    if let Some(p) = state_path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tag = if lang == Lang::En { "en" } else { "ru" };
        let _ = std::fs::write(p, format!("{tag} {lesson}\n"));
    }
}

pub fn locate(lang: Lang, c: char) -> Option<(usize, usize)> {
    let c = c.to_lowercase().next()?;
    lang.rows()
        .iter()
        .enumerate()
        .find_map(|(r, keys)| keys.chars().position(|k| k == c).map(|col| (r, col)))
}

pub fn finger_of(lang: Lang, c: char) -> Option<Finger> {
    match c {
        ' ' => Some(Finger::Thumb),
        'ё' | 'Ё' | '`' | '~' => Some(Finger::LPinky),
        _ => locate(lang, c).map(|(_, col)| Finger::for_column(col)),
    }
}

pub struct Rng(pub u64);

impl Rng {
    pub fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Rng(seed | 1)
    }

    pub fn below(&mut self, n: usize) -> usize {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x % n as u64) as usize
    }
}

pub fn pseudo_word(new: &[char], allowed: &[char], rng: &mut Rng) -> String {
    let len = 2 + rng.below(4);
    (0..len)
        .map(|_| {
            if rng.below(2) == 0 {
                new[rng.below(new.len())]
            } else {
                allowed[rng.below(allowed.len())]
            }
        })
        .collect()
}

pub fn make_text(lang: Lang, lesson: usize, rng: &mut Rng) -> Vec<char> {
    let dict: Vec<&str> = lang.words().split_whitespace().collect();
    let allowed = lang.allowed(lesson);
    let new: Vec<char> = lang.lessons()[lesson].new.chars().collect();
    let real: Vec<&str> = match &allowed {
        None => dict.clone(),
        Some(a) => dict.iter().copied().filter(|w| w.chars().all(|c| a.contains(&c))).collect(),
    };

    let mut picked: Vec<String> = Vec::with_capacity(WORDS_PER_TEST);
    while picked.len() < WORDS_PER_TEST {
        let w = match &allowed {
            Some(a) if rng.below(16) >= real.len().min(8) => pseudo_word(&new, a, rng),
            _ => real[rng.below(real.len())].to_string(),
        };
        if picked.last() != Some(&w) {
            picked.push(w);
        }
    }
    picked.join(" ").chars().collect()
}

pub struct Test {
    pub target: Vec<char>,
    pub typed: Vec<char>,
    pub started: Option<Instant>,
    pub finished: Option<Instant>,
    pub keystrokes: usize,
    pub mistakes: usize,
    pub wrong_layout: bool,
}

impl Test {
    pub fn new(target: Vec<char>) -> Self {
        Test {
            target,
            typed: Vec::new(),
            started: None,
            finished: None,
            keystrokes: 0,
            mistakes: 0,
            wrong_layout: false,
        }
    }

    pub fn done(&self) -> bool {
        self.finished.is_some()
    }

    pub fn type_char(&mut self, c: char, lang: Lang) {
        if self.done() {
            return;
        }
        let now = Instant::now();
        self.started.get_or_insert(now);
        self.keystrokes += 1;
        let ok = c == self.target[self.typed.len()];
        if !ok {
            self.mistakes += 1;
        }
        self.wrong_layout = !ok && lang.is_foreign(c);
        self.typed.push(c);
        if self.typed.len() == self.target.len() {
            self.finished = Some(now);
        }
    }

    pub fn backspace(&mut self) {
        if !self.done() {
            self.typed.pop();
            self.wrong_layout = false;
        }
    }

    pub fn elapsed(&self) -> Duration {
        match self.started {
            None => Duration::ZERO,
            Some(s) => self.finished.unwrap_or_else(Instant::now) - s,
        }
    }

    pub fn correct(&self) -> usize {
        self.typed
            .iter()
            .zip(&self.target)
            .filter(|(a, b)| a == b)
            .count()
    }

    pub fn cpm(&self) -> f64 {
        let secs = self.elapsed().as_secs_f64();
        if secs < 1.0 {
            0.0
        } else {
            self.correct() as f64 * 60.0 / secs
        }
    }

    pub fn accuracy(&self) -> f64 {
        if self.keystrokes == 0 {
            100.0
        } else {
            (self.keystrokes - self.mistakes) as f64 * 100.0 / self.keystrokes as f64
        }
    }
}

pub fn wrap(text: &[char], width: usize) -> Vec<(usize, usize)> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut last_break: Option<usize> = None;
    for i in 0..text.len() {
        if text[i] == ' ' {
            last_break = Some(i + 1);
        }
        if i + 1 - start > width {
            match last_break {
                Some(b) if b > start => {
                    lines.push((start, b));
                    start = b;
                }
                _ => {
                    lines.push((start, i));
                    start = i;
                }
            }
            last_break = None;
        }
    }
    if start < text.len() {
        lines.push((start, text.len()));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn wrap_breaks_on_spaces() {
        let t = chars("один два три четыре");
        let lines = wrap(&t, 8);
        let parts: Vec<String> = lines.iter().map(|&(s, e)| t[s..e].iter().collect()).collect();
        assert_eq!(parts, ["один два ", "три ", "четыре"]);
    }

    #[test]
    fn wrap_splits_long_word() {
        let t = chars("абвгдеж");
        assert_eq!(wrap(&t, 3), [(0, 3), (3, 6), (6, 7)]);
    }

    #[test]
    fn stats_and_finish() {
        let mut t = Test::new(chars("да"));
        t.type_char('l', Lang::Ru);
        assert!(t.wrong_layout);
        t.backspace();
        t.type_char('д', Lang::Ru);
        t.type_char('а', Lang::Ru);
        assert!(t.done());
        assert_eq!(t.correct(), 2);
        assert_eq!(t.keystrokes, 3);
        assert!((t.accuracy() - 200.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn fingers() {
        assert_eq!(finger_of(Lang::Ru, 'о'), Some(Finger::RIndex));
        assert_eq!(finger_of(Lang::Ru, 'а'), Some(Finger::LIndex));
        assert_eq!(finger_of(Lang::Ru, 'ж'), Some(Finger::RPinky));
        assert_eq!(finger_of(Lang::Ru, 'ю'), Some(Finger::RRing));
        assert_eq!(finger_of(Lang::Ru, 'й'), Some(Finger::LPinky));
        assert_eq!(finger_of(Lang::En, 'f'), Some(Finger::LIndex));
        assert_eq!(finger_of(Lang::En, 'b'), Some(Finger::LIndex));
        assert_eq!(finger_of(Lang::En, 'n'), Some(Finger::RIndex));
        assert_eq!(finger_of(Lang::En, ' '), Some(Finger::Thumb));
    }

    #[test]
    fn every_word_letter_has_a_finger() {
        for (lang, words) in [(Lang::Ru, RU_WORDS), (Lang::En, EN_WORDS)] {
            for c in words.chars().filter(|c| !c.is_whitespace()) {
                assert!(finger_of(lang, c).is_some(), "{lang:?}: {c}");
            }
        }
    }

    #[test]
    fn lessons_cover_alphabet_once() {
        for (lang, alphabet) in [
            (Lang::Ru, "абвгдежзийклмнопрстуфхцчшщъыьэюя"),
            (Lang::En, "abcdefghijklmnopqrstuvwxyz"),
        ] {
            let mut all: Vec<char> = lang.lessons().iter().flat_map(|l| l.new.chars()).collect();
            all.sort();
            let mut expected: Vec<char> = alphabet.chars().collect();
            expected.sort();
            assert_eq!(all, expected, "{lang:?}");
        }
    }

    #[test]
    fn lesson_text_uses_only_learned_letters() {
        let mut rng = Rng(12345);
        for lang in [Lang::Ru, Lang::En] {
            for lesson in 0..LESSON_COUNT {
                let text = make_text(lang, lesson, &mut rng);
                assert_eq!(text.iter().filter(|&&c| c == ' ').count(), WORDS_PER_TEST - 1);
                if let Some(allowed) = lang.allowed(lesson) {
                    for c in &text {
                        assert!(*c == ' ' || allowed.contains(c), "{lang:?} урок {lesson}: {c}");
                    }
                }
            }
        }
    }

    #[test]
    fn word_lists_are_clean() {
        for w in RU_WORDS.split_whitespace() {
            assert!(w.chars().all(|c| matches!(c, 'а'..='я')), "ru: {w}");
        }
        for w in EN_WORDS.split_whitespace() {
            assert!(w.chars().all(|c| c.is_ascii_lowercase()), "en: {w}");
        }
    }
}
