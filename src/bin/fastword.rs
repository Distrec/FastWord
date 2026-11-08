use std::io::{self, Stdout, Write};
use std::time::Duration;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType},
};

use fastword::*;

const MAX_WIDTH: usize = 70;

fn finger_color(f: Finger) -> Color {
    [Color::Magenta, Color::Blue, Color::Green, Color::Yellow, Color::Cyan][f.group()]
}

fn draw_keyboard(out: &mut Stdout, lang: Lang, lesson: usize, next: Option<char>, cols: usize, row0: usize) -> io::Result<()> {
    const KB_WIDTH: usize = 12 * 3 + 2;
    let left = cols.saturating_sub(KB_WIDTH) / 2;
    let hot = next.and_then(|c| locate(lang, c));
    let allowed = lang.allowed(lesson);
    let new = lang.lesson(lesson).new;
    for (r, keys) in lang.rows().iter().enumerate() {
        queue!(out, cursor::MoveTo((left + r) as u16, (row0 + r) as u16))?;
        for (col, key) in keys.chars().enumerate() {
            let is_hot = hot == Some((r, col));
            let home_mark = r == 1 && (col == 3 || col == 6);
            let in_lesson = allowed.as_ref().map_or(true, |a| a.contains(&key));
            let color = if in_lesson { finger_color(Finger::for_column(col)) } else { Color::DarkGrey };
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
        SetForegroundColor(finger_color(Finger::Thumb)),
        SetAttribute(if space_hot { Attribute::Reverse } else { Attribute::Dim }),
        Print(format!("{:^18}", "пробел")),
        SetAttribute(Attribute::Reset),
    )
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
    let mut lines = wrap(&t.target, width);
    if t.endless {
        let cur = lines.iter().position(|&(_, e)| t.typed.len() < e).unwrap_or(0);
        lines = lines.into_iter().skip(cur.saturating_sub(1)).take(ENDLESS_LINES).collect();
    }
    let top = rows.saturating_sub(lines.len() + 13) / 2;
    let left = (cols.saturating_sub(width) / 2) as u16;
    let at = |row: usize| cursor::MoveTo(left, row as u16);

    let info = lang.lesson(lesson);
    let title = if is_endless(lesson) {
        format!("  ∞ {}", info.title)
    } else {
        format!("  Урок {}/{LESSON_COUNT}: {}", lesson + 1, info.title)
    };
    let letters: String = info.new.chars().map(|c| format!("{c} ")).collect();
    queue!(
        out,
        terminal::Clear(ClearType::All),
        at(top),
        SetForegroundColor(Color::DarkGrey),
        Print("FastWord  "),
        SetForegroundColor(Color::Cyan),
        Print(lang.label()),
        SetForegroundColor(Color::Reset),
        Print(title),
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
    } else if t.endless && t.paused() {
        queue!(
            out,
            at(status),
            SetForegroundColor(Color::DarkGrey),
            Print(format!(
                "Пауза · слов: {} · {:.0} зн/мин · точность {:.0}% · ↑ — начать заново",
                t.typed.iter().filter(|&&c| c == ' ').count(),
                t.cpm(),
                t.accuracy()
            )),
        )?;
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
                SetForegroundColor(finger_color(f)),
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
                eprintln!("Использование: fastword [ru|en]");
                std::process::exit(2);
            }
        }
    }

    let mut out = io::stdout();
    let _guard = TermGuard::enter(&mut out)?;
    let mut rng = Rng::new();
    let mut test = Test::for_mode(lang, lesson, &mut rng);
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
                        test = Test::for_mode(lang, lesson, &mut rng);
                    }
                    a @ (Action::ToggleLang | Action::PrevLesson | Action::NextLesson) => {
                        match a {
                            Action::ToggleLang => lang = lang.toggle(),
                            Action::PrevLesson => lesson = cycle_lesson(lesson, false),
                            _ => lesson = cycle_lesson(lesson, true),
                        }
                        save_state(lang, lesson);
                        test = Test::for_mode(lang, lesson, &mut rng);
                    }
                    Action::Backspace => test.backspace(),
                    Action::Type(c) => {
                        test.type_char(c, lang);
                        test.top_up(lang, &mut rng);
                    }
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
