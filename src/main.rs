use std::io::{self, Stdout, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};

const RU_WORDS: &str = include_str!("../words/ru.txt");
const EN_WORDS: &str = include_str!("../words/en.txt");
const WORDS_PER_TEST: usize = 25;
const MAX_WIDTH: usize = 70;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Lang {
    Ru,
    En,
}

impl Lang {
    fn toggle(self) -> Self {
        match self {
            Lang::Ru => Lang::En,
            Lang::En => Lang::Ru,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Lang::Ru => "RU",
            Lang::En => "EN",
        }
    }

    fn words(self) -> &'static str {
        match self {
            Lang::Ru => RU_WORDS,
            Lang::En => EN_WORDS,
        }
    }

    fn is_foreign(self, c: char) -> bool {
        match self {
            Lang::Ru => c.is_ascii_alphabetic(),
            Lang::En => is_cyrillic(c),
        }
    }

    fn layout_hint(self) -> &'static str {
        match self {
            Lang::Ru => "Похоже, включена английская раскладка — переключите на русскую",
            Lang::En => "Похоже, включена русская раскладка — переключите на английскую",
        }
    }
}

fn is_cyrillic(c: char) -> bool {
    matches!(c, 'а'..='я' | 'А'..='Я' | 'ё' | 'Ё')
}

const KEYS_EN: [&str; 3] = ["qwertyuiop[]", "asdfghjkl;'", "zxcvbnm,./"];
const KEYS_RU: [&str; 3] = ["йцукенгшщзхъ", "фывапролджэ", "ячсмитьбю."];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Finger {
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
    fn for_column(col: usize) -> Self {
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

    fn name(self) -> &'static str {
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

    fn color(self) -> Color {
        match self {
            Finger::LPinky | Finger::RPinky => Color::Magenta,
            Finger::LRing | Finger::RRing => Color::Blue,
            Finger::LMiddle | Finger::RMiddle => Color::Green,
            Finger::LIndex | Finger::RIndex => Color::Yellow,
            Finger::Thumb => Color::Cyan,
        }
    }
}

struct Lesson {
    title: &'static str,
    new: &'static str,
}

const LESSONS_RU: [Lesson; 7] = [
    Lesson { title: "Основной ряд", new: "фываолдж" },
    Lesson { title: "Основной ряд целиком", new: "прэ" },
    Lesson { title: "Верхний ряд", new: "кенг" },
    Lesson { title: "Верхний ряд целиком", new: "йцушщзхъ" },
    Lesson { title: "Нижний ряд", new: "мить" },
    Lesson { title: "Нижний ряд целиком", new: "ячсбю" },
    Lesson { title: "Все ряды — слова", new: "" },
];

const LESSONS_EN: [Lesson; 7] = [
    Lesson { title: "Основной ряд", new: "asdfjkl" },
    Lesson { title: "Основной ряд целиком", new: "gh" },
    Lesson { title: "Верхний ряд", new: "erui" },
    Lesson { title: "Верхний ряд целиком", new: "qwtyop" },
    Lesson { title: "Нижний ряд", new: "vbnm" },
    Lesson { title: "Нижний ряд целиком", new: "zxc" },
    Lesson { title: "Все ряды — слова", new: "" },
];

const LESSON_COUNT: usize = 7;

impl Lang {
    fn rows(self) -> [&'static str; 3] {
        match self {
            Lang::Ru => KEYS_RU,
            Lang::En => KEYS_EN,
        }
    }

    fn lessons(self) -> &'static [Lesson; LESSON_COUNT] {
        match self {
            Lang::Ru => &LESSONS_RU,
            Lang::En => &LESSONS_EN,
        }
    }

    fn allowed(self, lesson: usize) -> Option<Vec<char>> {
        let lessons = self.lessons();
        if lessons[lesson].new.is_empty() {
            return None;
        }
        Some(lessons[..=lesson].iter().flat_map(|l| l.new.chars()).collect())
    }
}

fn state_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".local/state")))?;
    Some(base.join("klava").join("state"))
}

fn load_state() -> (Lang, usize) {
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

fn save_state(lang: Lang, lesson: usize) {
    if let Some(p) = state_path() {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tag = if lang == Lang::En { "en" } else { "ru" };
        let _ = std::fs::write(p, format!("{tag} {lesson}\n"));
    }
}

fn locate(lang: Lang, c: char) -> Option<(usize, usize)> {
    let c = c.to_lowercase().next()?;
    lang.rows()
        .iter()
        .enumerate()
        .find_map(|(r, keys)| keys.chars().position(|k| k == c).map(|col| (r, col)))
}

fn finger_of(lang: Lang, c: char) -> Option<Finger> {
    match c {
        ' ' => Some(Finger::Thumb),
        'ё' | 'Ё' | '`' | '~' => Some(Finger::LPinky),
        _ => locate(lang, c).map(|(_, col)| Finger::for_column(col)),
    }
}

fn draw_keyboard(out: &mut Stdout, lang: Lang, lesson: usize, next: Option<char>, cols: usize, row0: usize) -> io::Result<()> {
    const KB_WIDTH: usize = 12 * 3 + 2;
    let left = cols.saturating_sub(KB_WIDTH) / 2;
    let hot = next.and_then(|c| locate(lang, c));
    let allowed = lang.allowed(lesson);
    let new = lang.lessons()[lesson].new;
    for (r, keys) in lang.rows().iter().enumerate() {
        queue!(out, cursor::MoveTo((left + r) as u16, (row0 + r) as u16))?;
        for (col, key) in keys.chars().enumerate() {
            let is_hot = hot == Some((r, col));
            let home_mark = r == 1 && (col == 3 || col == 6);
            let in_lesson = allowed.as_ref().map_or(true, |a| a.contains(&key));
            let color = if in_lesson { Finger::for_column(col).color() } else { Color::DarkGrey };
            queue!(
                out,
                SetAttribute(Attribute::Reset),
                SetForegroundColor(color),
                SetAttribute(if is_hot { Attribute::Reverse } else { Attribute::Dim }),
            )?;
            if is_hot || new.contains(key) {
                queue!(out, SetAttribute(Attribute::NormalIntensity), SetAttribute(Attribute::Bold))?;
            }
            queue!(out, Print(' '))?;
            if home_mark {
                queue!(out, SetAttribute(Attribute::Underlined))?;
            }
            queue!(out, Print(key), SetAttribute(Attribute::NoUnderline), Print(' '))?;
        }
    }
    let space_hot = next == Some(' ');
    queue!(
        out,
        SetAttribute(Attribute::Reset),
        cursor::MoveTo((left + 9) as u16, (row0 + 3) as u16),
        SetForegroundColor(Finger::Thumb.color()),
        SetAttribute(if space_hot { Attribute::Reverse } else { Attribute::Dim }),
        Print(format!("{:^18}", "пробел")),
        SetAttribute(Attribute::Reset),
    )
}

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Rng(seed | 1)
    }

    fn below(&mut self, n: usize) -> usize {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x % n as u64) as usize
    }
}

fn pseudo_word(new: &[char], allowed: &[char], rng: &mut Rng) -> String {
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

fn make_text(lang: Lang, lesson: usize, rng: &mut Rng) -> Vec<char> {
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

struct Test {
    target: Vec<char>,
    typed: Vec<char>,
    started: Option<Instant>,
    finished: Option<Instant>,
    keystrokes: usize,
    mistakes: usize,
    wrong_layout: bool,
}

impl Test {
    fn new(target: Vec<char>) -> Self {
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

    fn done(&self) -> bool {
        self.finished.is_some()
    }

    fn type_char(&mut self, c: char, lang: Lang) {
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

    fn backspace(&mut self) {
        if !self.done() {
            self.typed.pop();
            self.wrong_layout = false;
        }
    }

    fn elapsed(&self) -> Duration {
        match self.started {
            None => Duration::ZERO,
            Some(s) => self.finished.unwrap_or_else(Instant::now) - s,
        }
    }

    fn correct(&self) -> usize {
        self.typed
            .iter()
            .zip(&self.target)
            .filter(|(a, b)| a == b)
            .count()
    }

    fn cpm(&self) -> f64 {
        let secs = self.elapsed().as_secs_f64();
        if secs < 1.0 {
            0.0
        } else {
            self.correct() as f64 * 60.0 / secs
        }
    }

    fn accuracy(&self) -> f64 {
        if self.keystrokes == 0 {
            100.0
        } else {
            (self.keystrokes - self.mistakes) as f64 * 100.0 / self.keystrokes as f64
        }
    }
}

fn wrap(text: &[char], width: usize) -> Vec<(usize, usize)> {
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

fn draw_char(out: &mut Stdout, t: &Test, i: usize) -> io::Result<()> {
    let target = t.target[i];
    if i < t.typed.len() {
        if t.typed[i] == target {
            queue!(out, SetForegroundColor(Color::Reset), Print(target))
        } else {
            let shown = if target == ' ' { '·' } else { target };
            queue!(
                out,
                SetForegroundColor(Color::Red),
                SetAttribute(Attribute::Underlined),
                Print(shown),
                SetAttribute(Attribute::NoUnderline)
            )
        }
    } else if i == t.typed.len() {
        queue!(
            out,
            SetForegroundColor(Color::Reset),
            SetAttribute(Attribute::Reverse),
            Print(target),
            SetAttribute(Attribute::NoReverse)
        )
    } else {
        queue!(out, SetForegroundColor(Color::DarkGrey), Print(target))
    }
}

fn draw(out: &mut Stdout, lang: Lang, lesson: usize, t: &Test) -> io::Result<()> {
    let (cols, rows) = terminal::size()?;
    let (cols, rows) = (cols as usize, rows as usize);
    let width = MAX_WIDTH.min(cols.saturating_sub(4)).max(10);
    let lines = wrap(&t.target, width);
    let top = rows.saturating_sub(lines.len() + 13) / 2;
    let left = (cols.saturating_sub(width) / 2) as u16;
    let at = |row: usize| cursor::MoveTo(left, row as u16);

    let info = &lang.lessons()[lesson];
    let letters: String = info.new.chars().map(|c| format!("{c} ")).collect();
    queue!(
        out,
        terminal::Clear(ClearType::All),
        at(top),
        SetForegroundColor(Color::DarkGrey),
        Print("klava  "),
        SetForegroundColor(Color::Cyan),
        Print(lang.label()),
        SetForegroundColor(Color::Reset),
        Print(format!("  Урок {}/{LESSON_COUNT}: {}", lesson + 1, info.title)),
    )?;
    if !letters.is_empty() {
        queue!(
            out,
            SetForegroundColor(Color::DarkGrey),
            Print(if lesson == 0 { "  · буквы: " } else { "  · новые: " }),
            SetForegroundColor(Color::Reset),
            SetAttribute(Attribute::Bold),
            Print(letters.trim_end()),
            SetAttribute(Attribute::Reset),
        )?;
    }
    queue!(
        out,
        at(top + 1),
        SetForegroundColor(Color::DarkGrey),
        Print("←/→ — урок · ↑ — другие слова · Tab — раскладка · Esc — выход"),
    )?;

    for (n, &(s, e)) in lines.iter().enumerate() {
        queue!(out, at(top + 3 + n))?;
        for i in s..e {
            draw_char(out, t, i)?;
        }
    }

    let status = top + 4 + lines.len();
    let secs = t.elapsed().as_secs_f64();
    if t.done() {
        queue!(
            out,
            at(status),
            SetForegroundColor(Color::Green),
            Print("Готово!  "),
            SetForegroundColor(Color::Reset),
            Print(format!(
                "{:.0} зн/мин ({:.0} сл/мин) · точность {:.0}% · {:.1} с",
                t.cpm(),
                t.cpm() / 5.0,
                t.accuracy(),
                secs
            )),
            at(status + 2),
            SetForegroundColor(Color::DarkGrey),
            Print("Enter — ещё раз"),
        )?;
        if lesson + 1 < LESSON_COUNT {
            if t.accuracy() >= 95.0 {
                queue!(
                    out,
                    SetForegroundColor(Color::Green),
                    Print(" · точность от 95% — можно дальше: → следующий урок"),
                )?;
            } else {
                queue!(out, Print(" · для перехода к следующему уроку цель — точность от 95%"))?;
            }
        }
    } else if t.started.is_none() {
        queue!(
            out,
            at(status),
            SetForegroundColor(Color::DarkGrey),
            Print("Начните печатать — таймер запустится с первой клавиши"),
        )?;
    } else {
        queue!(
            out,
            at(status),
            SetForegroundColor(Color::DarkGrey),
            Print(format!(
                "{:.0} с · {:.0} зн/мин · точность {:.0}%",
                secs.floor(),
                t.cpm(),
                t.accuracy()
            )),
        )?;
    }
    if t.wrong_layout {
        queue!(
            out,
            at(status + 1),
            SetForegroundColor(Color::Yellow),
            Print(lang.layout_hint()),
        )?;
    }

    let next = t.target.get(t.typed.len()).copied();
    if let Some(c) = next {
        if let Some(f) = finger_of(lang, c) {
            let key = if c == ' ' { "Пробел".to_string() } else { format!("Клавиша «{c}»") };
            queue!(
                out,
                at(status + 3),
                SetForegroundColor(Color::DarkGrey),
                Print(format!("{key} — ")),
                SetForegroundColor(f.color()),
                SetAttribute(Attribute::Bold),
                Print(f.name()),
                SetAttribute(Attribute::Reset),
            )?;
        }
    }
    draw_keyboard(out, lang, lesson, next, cols, status + 5)?;

    queue!(out, ResetColor, SetAttribute(Attribute::Reset))?;
    out.flush()
}

struct TermGuard;

impl TermGuard {
    fn enter(out: &mut Stdout) -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(out, terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(TermGuard)
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), cursor::Show, terminal::LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

enum Action {
    Quit,
    Restart,
    ToggleLang,
    Next,
    PrevLesson,
    NextLesson,
    Backspace,
    Type(char),
    None,
}

fn action(k: KeyEvent, done: bool) -> Action {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    let alt = k.modifiers.contains(KeyModifiers::ALT);
    match k.code {
        KeyCode::Esc => Action::Quit,
        KeyCode::Char('c' | 'с') if ctrl => Action::Quit,
        KeyCode::Char('r' | 'к') if ctrl => Action::Restart,
        KeyCode::Tab => Action::ToggleLang,
        KeyCode::Up => Action::Restart,
        KeyCode::Left => Action::PrevLesson,
        KeyCode::Right => Action::NextLesson,
        KeyCode::Enter if done => Action::Next,
        KeyCode::Backspace => Action::Backspace,
        KeyCode::Char(c) if !ctrl && !alt => Action::Type(c),
        _ => Action::None,
    }
}

fn main() -> io::Result<()> {
    let (mut lang, mut lesson) = load_state();
    if let Some(arg) = std::env::args().nth(1) {
        match arg.as_str() {
            "en" => lang = Lang::En,
            "ru" => lang = Lang::Ru,
            _ => {
                eprintln!("Использование: klava [ru|en]");
                std::process::exit(2);
            }
        }
    }

    let mut out = io::stdout();
    let _guard = TermGuard::enter(&mut out)?;
    let mut rng = Rng::new();
    let mut test = Test::new(make_text(lang, lesson, &mut rng));
    let mut dirty = true;

    loop {
        if dirty {
            draw(&mut out, lang, lesson, &test)?;
            dirty = false;
        }
        if !event::poll(Duration::from_millis(250))? {
            dirty = test.started.is_some() && !test.done();
            continue;
        }
        match event::read()? {
            Event::Key(k) if k.kind != KeyEventKind::Release => {
                match action(k, test.done()) {
                    Action::Quit => break,
                    Action::Restart | Action::Next => {
                        test = Test::new(make_text(lang, lesson, &mut rng));
                    }
                    a @ (Action::ToggleLang | Action::PrevLesson | Action::NextLesson) => {
                        match a {
                            Action::ToggleLang => lang = lang.toggle(),
                            Action::PrevLesson => lesson = (lesson + LESSON_COUNT - 1) % LESSON_COUNT,
                            _ => lesson = (lesson + 1) % LESSON_COUNT,
                        }
                        save_state(lang, lesson);
                        test = Test::new(make_text(lang, lesson, &mut rng));
                    }
                    Action::Backspace => test.backspace(),
                    Action::Type(c) => test.type_char(c, lang),
                    Action::None => continue,
                }
                dirty = true;
            }
            Event::Resize(..) => dirty = true,
            _ => {}
        }
    }
    Ok(())
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
