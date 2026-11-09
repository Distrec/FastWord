use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const RU_WORDS: &str = include_str!("../words/ru.txt");
pub const EN_WORDS: &str = include_str!("../words/en.txt");
pub const RU_PHRASES: &str = include_str!("../words/phrases_ru.txt");
pub const EN_PHRASES: &str = include_str!("../words/phrases_en.txt");
pub const PHRASES_PER_TEST: usize = 4;
pub const WORDS_PER_TEST: usize = 25;
pub const ENDLESS_AHEAD: usize = 200;
pub const ENDLESS_LINES: usize = 3;
pub const ENDLESS_PAUSE: Duration = Duration::from_secs(3);

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

pub const PAIRS_RU: [Lesson; 8] = [
    Lesson { title: "ва ол", new: "ваол" },
    Lesson { title: "фы дж", new: "фыдж" },
    Lesson { title: "ми ть", new: "мить" },
    Lesson { title: "еп нр", new: "епнр" },
    Lesson { title: "ук гш", new: "укгш" },
    Lesson { title: "чс бю", new: "чсбю" },
    Lesson { title: "йц щз", new: "йцщз" },
    Lesson { title: "я эхъ", new: "яэхъ" },
];

pub const PAIRS_EN: [Lesson; 8] = [
    Lesson { title: "df jk", new: "dfjk" },
    Lesson { title: "as l;", new: "asl;" },
    Lesson { title: "vb nm", new: "vbnm" },
    Lesson { title: "tg yh", new: "tgyh" },
    Lesson { title: "er ui", new: "erui" },
    Lesson { title: "qw op", new: "qwop" },
    Lesson { title: "xc , .", new: "xc,." },
    Lesson { title: "z ! ? /", new: "z!?/" },
];

pub const PAIR_COUNT: usize = 8;

pub const LESSON_COUNT: usize = 7;
pub const ENDLESS: usize = LESSON_COUNT;
pub const MODE_COUNT: usize = LESSON_COUNT + 1;

pub const ENDLESS_LESSON: Lesson = Lesson { title: "Бесконечный режим", new: "" };

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Mix {
    Words,
    All,
}

impl Mix {
    pub fn toggle(self) -> Self {
        match self {
            Mix::Words => Mix::All,
            Mix::All => Mix::Words,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Mix::Words => "Слова",
            Mix::All => "Всё подряд",
        }
    }
}

pub const MIN_WORD: usize = 3;

pub fn is_endless(mode: usize) -> bool {
    mode == ENDLESS
}

pub fn cycle_lesson(lesson: usize, forward: bool) -> usize {
    if forward {
        (lesson + 1) % MODE_COUNT
    } else {
        (lesson + MODE_COUNT - 1) % MODE_COUNT
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

    pub fn pairs(self) -> &'static [Lesson; PAIR_COUNT] {
        match self {
            Lang::Ru => &PAIRS_RU,
            Lang::En => &PAIRS_EN,
        }
    }

    pub fn lesson(self, mode: usize) -> &'static Lesson {
        self.lessons().get(mode).unwrap_or(&ENDLESS_LESSON)
    }

    pub fn allowed(self, lesson: usize) -> Option<Vec<char>> {
        if self.lesson(lesson).new.is_empty() {
            return None;
        }
        Some(self.lessons()[..=lesson].iter().flat_map(|l| l.new.chars()).collect())
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Best {
    pub cpm: u32,
    pub secs: u32,
    pub acc: u32,
}

fn state_file(name: &str) -> Option<std::path::PathBuf> {
    Some(state_path()?.with_file_name(name))
}

fn write_file(path: Option<std::path::PathBuf>, text: String) {
    if let Some(p) = path {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, text);
    }
}

fn lang_tag(lang: Lang) -> &'static str {
    if lang == Lang::En { "en" } else { "ru" }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Lesson(usize),
    Phrases(bool),
    Mixed(bool),
}

impl Kind {
    pub fn endless(self) -> bool {
        matches!(self, Kind::Phrases(true) | Kind::Mixed(true))
    }
}

pub fn load_gui_state() -> (Lang, Kind) {
    let text = state_file("gui").and_then(|p| std::fs::read_to_string(p).ok());
    let Some(text) = text else {
        return (load_state().0, Kind::Lesson(0));
    };
    let mut parts = text.split_whitespace();
    let lang = if parts.next() == Some("en") { Lang::En } else { Lang::Ru };
    let tag = parts.next();
    let endless = parts.next() == Some("inf");
    let kind = match tag {
        Some("phrases") => Kind::Phrases(endless),
        Some("mixed") => Kind::Mixed(endless),
        s => Kind::Lesson(s.and_then(|s| s.parse::<usize>().ok()).filter(|&n| n < PAIR_COUNT).unwrap_or(0)),
    };
    (lang, kind)
}

pub fn save_gui_state(lang: Lang, kind: Kind) {
    let kind = match kind {
        Kind::Lesson(i) => i.to_string(),
        Kind::Phrases(e) => format!("phrases {}", if e { "inf" } else { "fin" }),
        Kind::Mixed(e) => format!("mixed {}", if e { "inf" } else { "fin" }),
    };
    write_file(state_file("gui"), format!("{} {kind}\n", lang_tag(lang)));
}

impl Lang {
    pub fn phrases(self) -> Vec<&'static str> {
        let text = match self {
            Lang::Ru => RU_PHRASES,
            Lang::En => EN_PHRASES,
        };
        text.lines().map(str::trim).filter(|l| !l.is_empty()).collect()
    }
}

pub fn make_blend(lang: Lang, rng: &mut Rng) -> Vec<char> {
    let phrases = lang.phrases();
    let words: Vec<&str> = lang.words().split_whitespace().filter(|w| w.chars().count() >= MIN_WORD).collect();
    let mut parts: Vec<String> = Vec::new();
    let mut len = 0;
    let mut words_next = rng.below(2) == 0;
    while len < 160 {
        let part = if words_next {
            let n = 2 + rng.below(3);
            (0..n).map(|_| words[rng.below(words.len())]).collect::<Vec<_>>().join(" ")
        } else {
            phrases[rng.below(phrases.len())].to_string()
        };
        words_next = !words_next || rng.below(3) == 0;
        len += part.chars().count() + 1;
        parts.push(part);
    }
    parts.join(" ").chars().collect()
}

pub fn make_phrases(lang: Lang, rng: &mut Rng) -> Vec<char> {
    let all = lang.phrases();
    let mut picked: Vec<&str> = Vec::with_capacity(PHRASES_PER_TEST);
    while picked.len() < PHRASES_PER_TEST {
        let p = all[rng.below(all.len())];
        if !picked.contains(&p) {
            picked.push(p);
        }
    }
    picked.join(" ").chars().collect()
}

fn read_results() -> Vec<(String, usize, Best)> {
    let text = state_file("results").and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    text.lines()
        .filter_map(|line| {
            let v: Vec<&str> = line.split_whitespace().collect();
            let n = |i: usize| v.get(i)?.parse::<u32>().ok();
            Some((v.first()?.to_string(), n(1)? as usize, Best { cpm: n(2)?, secs: n(3)?, acc: n(4)? }))
        })
        .collect()
}

pub fn load_results(lang: Lang) -> Vec<Option<Best>> {
    let mut out = vec![None; PAIR_COUNT];
    for (tag, i, b) in read_results() {
        if tag == lang_tag(lang) && i < PAIR_COUNT {
            out[i] = Some(b);
        }
    }
    out
}

pub fn save_result(lang: Lang, lesson: usize, best: Best) -> bool {
    let mut all = read_results();
    let tag = lang_tag(lang);
    match all.iter_mut().find(|(t, i, _)| t == tag && *i == lesson) {
        Some((_, _, b)) if b.cpm >= best.cpm => return false,
        Some((_, _, b)) => *b = best,
        None => all.push((tag.to_string(), lesson, best)),
    }
    let text: String = all.iter().map(|(t, i, b)| format!("{t} {i} {} {} {}\n", b.cpm, b.secs, b.acc)).collect();
    write_file(state_file("results"), text);
    true
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
        .filter(|&n| n < MODE_COUNT)
        .unwrap_or(LESSON_COUNT - 1);
    (lang, lesson)
}

pub fn load_mix() -> Mix {
    let text = state_path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    if text.split_whitespace().nth(2) == Some("all") { Mix::All } else { Mix::Words }
}

fn write_state(lang: Lang, lesson: usize, mix: Option<Mix>) {
    if let Some(p) = state_path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let mix = mix.unwrap_or_else(load_mix);
        let tag = if lang == Lang::En { "en" } else { "ru" };
        let mix_tag = if mix == Mix::All { "all" } else { "words" };
        let _ = std::fs::write(p, format!("{tag} {lesson} {mix_tag}\n"));
    }
}

pub fn save_state(lang: Lang, lesson: usize) {
    write_state(lang, lesson, None);
}

pub fn save_state_mix(lang: Lang, lesson: usize, mix: Mix) {
    write_state(lang, lesson, Some(mix));
}

pub fn locate(lang: Lang, c: char) -> Option<(usize, usize)> {
    let c = match (lang, c) {
        (Lang::En, '?') => '/',
        (Lang::Ru, ',') => '.',
        _ => c.to_lowercase().next()?,
    };
    lang.rows()
        .iter()
        .enumerate()
        .find_map(|(r, keys)| keys.chars().position(|k| k == c).map(|col| (r, col)))
}

pub fn finger_of(lang: Lang, c: char) -> Option<Finger> {
    match c {
        ' ' => Some(Finger::Thumb),
        'ё' | 'Ё' | '`' | '~' | '!' => Some(Finger::LPinky),
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
    let new: Vec<char> = lang.lesson(lesson).new.chars().collect();
    make_letters(lang, &new, lang.allowed(lesson), rng)
}

pub fn make_letters(lang: Lang, new: &[char], allowed: Option<Vec<char>>, rng: &mut Rng) -> Vec<char> {
    let dict: Vec<&str> = lang.words().split_whitespace().collect();
    let real: Vec<&str> = match &allowed {
        None => dict.clone(),
        Some(a) => dict.iter().copied().filter(|w| w.chars().all(|c| a.contains(&c))).collect(),
    };

    let mut picked: Vec<String> = Vec::with_capacity(WORDS_PER_TEST);
    while picked.len() < WORDS_PER_TEST {
        let w = match &allowed {
            Some(a) if rng.below(16) >= real.len().min(8) => pseudo_word(new, a, rng),
            _ => real[rng.below(real.len())].to_string(),
        };
        if picked.last() != Some(&w) {
            picked.push(w);
        }
    }
    picked.join(" ").chars().collect()
}

pub fn make_endless(lang: Lang, mix: Mix, rng: &mut Rng) -> Vec<char> {
    let dict: Vec<&str> = lang.words().split_whitespace().collect();
    let letters: Vec<char> = lang.lessons().iter().flat_map(|l| l.new.chars()).collect();
    let mut picked: Vec<String> = Vec::with_capacity(WORDS_PER_TEST);
    while picked.len() < WORDS_PER_TEST {
        let w = match mix {
            Mix::All if rng.below(3) == 0 => pseudo_word(&letters, &letters, rng),
            Mix::All => dict[rng.below(dict.len())].to_string(),
            Mix::Words => {
                let w = dict[rng.below(dict.len())];
                if w.chars().count() < MIN_WORD {
                    continue;
                }
                w.to_string()
            }
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
    pub endless: bool,
    pub last_key: Option<Instant>,
    pub idle: Duration,
    pub mix: Option<Mix>,
    pub phrases: bool,
    pub blend: bool,
}

impl Test {
    pub fn for_mode(lang: Lang, mode: usize, rng: &mut Rng) -> Self {
        let mut t = Test::new(make_text(lang, mode, rng));
        t.endless = is_endless(mode);
        t
    }

    pub fn for_kind(lang: Lang, kind: Kind, rng: &mut Rng) -> Self {
        match kind {
            Kind::Lesson(i) => Test::for_pair(lang, i, rng),
            Kind::Phrases(false) => {
                let mut t = Test::new(make_phrases(lang, rng));
                t.phrases = true;
                t
            }
            Kind::Phrases(true) => {
                let mut t = Test::new(make_blend(lang, rng));
                t.endless = true;
                t.blend = true;
                t
            }
            Kind::Mixed(endless) => {
                let mut t = Test::new(make_endless(lang, Mix::All, rng));
                t.endless = endless;
                t.mix = Some(Mix::All);
                t
            }
        }
    }

    pub fn for_pair(lang: Lang, lesson: usize, rng: &mut Rng) -> Self {
        let letters: Vec<char> = lang.pairs()[lesson].new.chars().collect();
        Test::new(make_letters(lang, &letters, Some(letters.clone()), rng))
    }

    pub fn for_mode_mix(lang: Lang, mode: usize, mix: Mix, rng: &mut Rng) -> Self {
        if !is_endless(mode) {
            return Test::for_mode(lang, mode, rng);
        }
        let mut t = Test::new(make_endless(lang, mix, rng));
        t.endless = true;
        t.mix = Some(mix);
        t
    }

    pub fn top_up(&mut self, lang: Lang, rng: &mut Rng) {
        if self.endless && self.target.len() - self.typed.len().min(self.target.len()) < ENDLESS_AHEAD {
            self.target.push(' ');
            let more = match self.mix {
                _ if self.phrases => make_phrases(lang, rng),
                _ if self.blend => make_blend(lang, rng),
                Some(mix) => make_endless(lang, mix, rng),
                None => make_text(lang, ENDLESS, rng),
            };
            self.target.extend(more);
        }
    }

    pub fn paused(&self) -> bool {
        self.last_key.map_or(true, |k| k.elapsed() >= ENDLESS_PAUSE)
    }

    pub fn new(target: Vec<char>) -> Self {
        Test {
            target,
            typed: Vec::new(),
            started: None,
            finished: None,
            keystrokes: 0,
            mistakes: 0,
            wrong_layout: false,
            endless: false,
            last_key: None,
            idle: Duration::ZERO,
            mix: None,
            phrases: false,
            blend: false,
        }
    }

    pub fn done(&self) -> bool {
        self.finished.is_some()
    }

    pub fn type_char(&mut self, c: char, lang: Lang) {
        if self.done() || self.typed.len() >= self.target.len() {
            return;
        }
        let now = Instant::now();
        self.started.get_or_insert(now);
        if let Some(gap) = self.last_key.map(|k| now - k) {
            if self.endless && gap > ENDLESS_PAUSE {
                self.idle += gap;
            }
        }
        self.last_key = Some(now);
        self.keystrokes += 1;
        let ok = c == self.target[self.typed.len()];
        if !ok {
            self.mistakes += 1;
        }
        self.wrong_layout = !ok && lang.is_foreign(c);
        self.typed.push(c);
        if !self.endless && self.typed.len() == self.target.len() {
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
        let Some(s) = self.started else { return Duration::ZERO };
        let end = match (self.endless, self.last_key) {
            (true, Some(k)) if self.paused() => k,
            _ => self.finished.unwrap_or_else(Instant::now),
        };
        (end - s).saturating_sub(self.idle)
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
    fn endless_never_finishes() {
        let mut rng = Rng(7);
        let mut t = Test::for_mode(Lang::Ru, ENDLESS, &mut rng);
        assert!(t.endless);
        for _ in 0..2000 {
            let c = t.target[t.typed.len()];
            t.type_char(c, Lang::Ru);
            t.top_up(Lang::Ru, &mut rng);
            assert!(t.target.len() - t.typed.len() >= ENDLESS_AHEAD);
        }
        assert!(!t.done());
        assert_eq!(t.correct(), 2000);
        assert_eq!(cycle_lesson(LESSON_COUNT - 1, true), ENDLESS);
        assert_eq!(cycle_lesson(ENDLESS, true), 0);
        assert_eq!(Lang::En.allowed(ENDLESS), None);
    }

    #[test]
    fn endless_mix() {
        let mut rng = Rng(7);
        for lang in [Lang::Ru, Lang::En] {
            let mut t = Test::for_mode_mix(lang, ENDLESS, Mix::Words, &mut rng);
            for _ in 0..2000 {
                t.type_char('x', lang);
                t.top_up(lang, &mut rng);
            }
            let text: String = t.target.iter().collect();
            let dict: Vec<&str> = lang.words().split_whitespace().collect();
            for w in text.split(' ') {
                assert!(w.chars().count() >= MIN_WORD && dict.contains(&w), "{w}");
            }
            let all: String = make_endless(lang, Mix::All, &mut rng).into_iter().collect();
            assert!(all.split(' ').all(|w| !w.is_empty() && w.chars().all(|c| finger_of(lang, c).is_some())));
        }
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
    fn pairs_cover_alphabet_once() {
        for (lang, alphabet) in [
            (Lang::Ru, "абвгдежзийклмнопрстуфхцчшщъыьэюя"),
            (Lang::En, "abcdefghijklmnopqrstuvwxyz"),
        ] {
            let mut all: Vec<char> = lang.pairs().iter().flat_map(|l| l.new.chars()).filter(|c| c.is_alphabetic()).collect();
            all.sort();
            let mut expected: Vec<char> = alphabet.chars().collect();
            expected.sort();
            assert_eq!(all, expected, "{lang:?}");
            for l in lang.pairs() {
                assert_eq!(l.title.replace(' ', ""), l.new);
            }
        }
    }

    #[test]
    fn pair_text_uses_only_learned_letters() {
        let mut rng = Rng(777);
        for lang in [Lang::Ru, Lang::En] {
            for lesson in 0..PAIR_COUNT {
                let allowed: Vec<char> = lang.pairs()[lesson].new.chars().collect();
                let text = Test::for_pair(lang, lesson, &mut rng).target;
                assert_eq!(text.iter().filter(|&&c| c == ' ').count(), WORDS_PER_TEST - 1);
                assert!(text.iter().all(|c| *c == ' ' || allowed.contains(c)), "{lang:?} {lesson}");
                for c in &allowed {
                    assert!(finger_of(lang, *c).is_some(), "{lang:?}: {c}");
                }
            }
        }
    }

    #[test]
    fn results_keep_best() {
        let dir = std::env::temp_dir().join(format!("fastword-test-{}", std::process::id()));
        std::env::set_var("XDG_STATE_HOME", &dir);
        let b = |cpm| Best { cpm, secs: 30, acc: 97 };
        assert!(save_result(Lang::Ru, 2, b(120)));
        assert!(!save_result(Lang::Ru, 2, b(90)));
        assert!(save_result(Lang::Ru, 2, b(150)));
        assert!(save_result(Lang::En, 2, b(40)));
        let ru = load_results(Lang::Ru);
        assert_eq!(ru[2], Some(b(150)));
        assert_eq!(ru[0], None);
        assert_eq!(load_results(Lang::En)[2], Some(b(40)));
        for (lang, kind) in [(Lang::En, Kind::Lesson(0)), (Lang::Ru, Kind::Lesson(7)), (Lang::Ru, Kind::Phrases(false)), (Lang::En, Kind::Mixed(true)), (Lang::Ru, Kind::Phrases(true))] {
            save_gui_state(lang, kind);
            assert_eq!(load_gui_state(), (lang, kind));
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn phrases_are_typeable() {
        for lang in [Lang::Ru, Lang::En] {
            assert!(lang.phrases().len() >= PHRASES_PER_TEST);
            for p in lang.phrases() {
                for c in p.chars() {
                    assert!(finger_of(lang, c).is_some(), "{lang:?}: «{c}» в «{p}»");
                    assert!(!lang.is_foreign(c), "{lang:?}: «{c}» в «{p}»");
                }
            }
            let mut rng = Rng(5);
            let t = Test::for_kind(lang, Kind::Phrases(false), &mut rng);
            assert!(!t.endless && t.target.len() > 40);
            let t = Test::for_kind(lang, Kind::Mixed(false), &mut rng);
            assert!(!t.endless && t.target.iter().filter(|&&c| c == ' ').count() == WORDS_PER_TEST - 1);
            let t = Test::for_kind(lang, Kind::Phrases(true), &mut rng);
            assert!(t.endless && t.blend && t.target.len() >= 160);
            assert!(t.target.iter().all(|c| finger_of(lang, *c).is_some()));
            for kind in [Kind::Phrases(true), Kind::Mixed(true)] {
                let mut t = Test::for_kind(lang, kind, &mut rng);
                assert!(t.endless);
                let start = t.target.len();
                for _ in 0..start {
                    let c = t.target[t.typed.len()];
                    t.type_char(c, lang);
                    t.top_up(lang, &mut rng);
                }
                assert!(!t.done() && t.target.len() > start, "{kind:?}");
                if kind != Kind::Mixed(true) {
                    assert!(t.target.iter().all(|c| finger_of(lang, *c).is_some()));
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
