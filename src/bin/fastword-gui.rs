#![windows_subsystem = "windows"]

use std::time::Duration;

use eframe::egui::{
    self, epaint::PathShape, text::LayoutJob, Align2, Color32, CornerRadius, FontId, Key, Pos2,
    Rect, Sense, Stroke, StrokeKind, TextFormat, Vec2,
};
use fastword::*;

struct Palette {
    bg: Color32,
    surface: Color32,
    text: Color32,
    dim: Color32,
    faint: Color32,
    accent: Color32,
    error: Color32,
    warn: Color32,
    fingers: [Color32; 5],
}

const P: Palette = Palette {
    bg: Color32::from_rgb(0x15, 0x15, 0x17),
    surface: Color32::from_rgb(0x20, 0x20, 0x24),
    text: Color32::from_rgb(0xe8, 0xe6, 0xe3),
    dim: Color32::from_rgb(0x6e, 0x6c, 0x6a),
    faint: Color32::from_rgb(0x34, 0x33, 0x36),
    accent: Color32::from_rgb(0xe2, 0xb7, 0x6e),
    error: Color32::from_rgb(0xe0, 0x6c, 0x75),
    warn: Color32::from_rgb(0xe5, 0xc0, 0x7b),
    fingers: [
        Color32::from_rgb(0xc6, 0x78, 0xdd),
        Color32::from_rgb(0x61, 0xaf, 0xef),
        Color32::from_rgb(0x98, 0xc3, 0x79),
        Color32::from_rgb(0xe5, 0xc0, 0x7b),
        Color32::from_rgb(0x56, 0xb6, 0xc2),
    ],
};

const COLUMN: f32 = 580.0;
const KEY: f32 = 34.0;
const GAP: f32 = 6.0;
const TOP_BAR: f32 = 56.0;
const FOOTER: f32 = 44.0;
const MENU_W: f32 = 168.0;
const MARGIN: f32 = 20.0;

struct App {
    lang: Lang,
    lesson: usize,
    rng: Rng,
    test: Test,
    menu_open: bool,
    content_h: f32,
}

enum Chip {
    Text(&'static str),
    Arrow(Vec2),
}

impl App {
    fn new(lang: Lang, lesson: usize) -> Self {
        let mut rng = Rng::new();
        let test = Test::new(make_text(lang, lesson, &mut rng));
        App { lang, lesson, rng, test, menu_open: true, content_h: 0.0 }
    }

    fn new_test(&mut self) {
        self.test = Test::new(make_text(self.lang, self.lesson, &mut self.rng));
    }

    fn switch(&mut self) {
        save_state(self.lang, self.lesson);
        self.new_test();
    }

    fn typing(&self) -> bool {
        self.test.started.is_some() && !self.test.done()
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|i| i.events.clone());
        for e in events {
            match e {
                egui::Event::Text(t) => {
                    for c in t.chars().filter(|c| !c.is_control()) {
                        self.test.type_char(c, self.lang);
                    }
                }
                egui::Event::Key { key, physical_key, pressed: true, modifiers, .. } => match key {
                    Key::Escape => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                    Key::Backspace => self.test.backspace(),
                    Key::Tab => {
                        self.lang = self.lang.toggle();
                        self.switch();
                    }
                    Key::ArrowLeft => {
                        self.lesson = cycle_lesson(self.lesson, false);
                        self.switch();
                    }
                    Key::ArrowRight => {
                        self.lesson = cycle_lesson(self.lesson, true);
                        self.switch();
                    }
                    Key::ArrowUp => self.new_test(),
                    Key::Enter if self.test.done() => self.new_test(),
                    _ if modifiers.ctrl && (key == Key::R || physical_key == Some(Key::R)) => self.new_test(),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn burger(&mut self, ui: &mut egui::Ui, full: Rect) {
        let r = Rect::from_min_size(full.min + Vec2::new(MARGIN, 18.0), Vec2::splat(22.0));
        let resp = ui.interact(r, ui.scope_id().with("burger"), Sense::CLICK);
        if resp.clicked() {
            self.menu_open = !self.menu_open;
        }
        let color = if resp.hovered() || (self.menu_open && !self.typing()) { P.text } else { P.dim };
        let p = ui.painter();
        for (i, len) in [16.0, 12.0, 16.0].into_iter().enumerate() {
            let y = r.top() + 5.0 + i as f32 * 6.0;
            p.line_segment([Pos2::new(r.left() + 3.0, y), Pos2::new(r.left() + 3.0 + len, y)], Stroke::new(2.0, color));
        }
    }

    fn logo(&self, ui: &egui::Ui, full: Rect) {
        let p = ui.painter();
        let font = FontId::proportional(18.0);
        let word = p.layout_no_wrap("Word".into(), font.clone(), P.accent);
        let fast = p.layout_no_wrap("Fast".into(), font, P.text);
        let cy = full.top() + 29.0;
        let mut x = full.right() - MARGIN - word.size().x;
        p.galley(Pos2::new(x, cy - word.size().y / 2.0), word, P.accent);
        x -= fast.size().x;
        p.galley(Pos2::new(x, cy - fast.size().y / 2.0), fast, P.text);

        let cap = Rect::from_center_size(Pos2::new(x - 10.0 - 13.0, cy), Vec2::splat(26.0));
        p.rect_filled(cap, CornerRadius::same(6), P.accent.gamma_multiply(0.12));
        p.rect_stroke(cap, CornerRadius::same(6), Stroke::new(1.5, P.accent), StrokeKind::Inside);
        p.line_segment(
            [Pos2::new(cap.left() + 5.0, cap.bottom() - 1.5), Pos2::new(cap.right() - 5.0, cap.bottom() - 1.5)],
            Stroke::new(2.0, P.accent),
        );
        p.text(cap.center() - Vec2::new(0.0, 1.0), Align2::CENTER_CENTER, "F", FontId::monospace(15.0), P.accent);

        for (i, len) in [10.0, 14.0, 8.0].into_iter().enumerate() {
            let y = cy - 6.0 + i as f32 * 6.0;
            let end = cap.left() - 5.0;
            p.line_segment([Pos2::new(end - len, y), Pos2::new(end, y)], Stroke::new(1.5, P.dim));
        }
    }

    fn menu(&mut self, ui: &mut egui::Ui, full: Rect) {
        let visible = self.menu_open && !self.typing();
        let t = ui.ctx().animate_bool_with_time(ui.scope_id().with("menu"), visible, 0.18);
        if t <= 0.0 {
            return;
        }
        let fade = |c: Color32| c.gamma_multiply(t);
        let left = full.left() + 14.0 - (1.0 - t) * 16.0;
        let mut y = full.top() + TOP_BAR + 8.0;
        let clickable = t > 0.6;

        ui.painter().text(Pos2::new(left, y), Align2::LEFT_TOP, "УРОКИ", FontId::proportional(11.0), fade(P.dim));
        y += 22.0;

        let mut chosen = None;
        for (i, lesson) in self.lang.lessons().iter().enumerate() {
            let row = Rect::from_min_size(Pos2::new(left, y), Vec2::new(MENU_W, 26.0));
            let resp = ui.interact(row, ui.scope_id().with(("lesson", i)), Sense::CLICK);
            let current = i == self.lesson;
            let p = ui.painter();
            if current {
                p.rect_filled(row, CornerRadius::same(6), fade(P.surface));
                let bar = Rect::from_min_size(row.min + Vec2::new(0.0, 6.0), Vec2::new(3.0, 14.0));
                p.rect_filled(bar, CornerRadius::same(2), fade(P.accent));
            } else if clickable && resp.hovered() {
                p.rect_filled(row, CornerRadius::same(6), fade(P.surface.gamma_multiply(0.6)));
            }
            let num_color = if current { P.accent } else { P.dim };
            let text_color = if current || (clickable && resp.hovered()) { P.text } else { P.dim };
            p.text(row.left_center() + Vec2::new(11.0, 0.0), Align2::LEFT_CENTER, format!("{}", i + 1), FontId::monospace(12.5), fade(num_color));
            p.text(row.left_center() + Vec2::new(26.0, 0.0), Align2::LEFT_CENTER, lesson.title, FontId::proportional(13.0), fade(text_color));
            if clickable && resp.clicked() {
                chosen = Some(i);
            }
            y += 28.0;
        }

        y += 14.0;
        ui.painter().text(Pos2::new(left, y), Align2::LEFT_TOP, "РАСКЛАДКА", FontId::proportional(11.0), fade(P.dim));
        y += 20.0;
        let mut new_lang = None;
        for (i, lang) in [Lang::Ru, Lang::En].into_iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(left + i as f32 * 50.0, y), Vec2::new(44.0, 24.0));
            let resp = ui.interact(r, ui.scope_id().with(("lang", i)), Sense::CLICK);
            let current = lang == self.lang;
            let p = ui.painter();
            let (fill, color) = if current {
                (P.accent.gamma_multiply(0.15), P.accent)
            } else if clickable && resp.hovered() {
                (P.surface, P.text)
            } else {
                (P.surface, P.dim)
            };
            p.rect_filled(r, CornerRadius::same(6), fade(fill));
            if current {
                p.rect_stroke(r, CornerRadius::same(6), Stroke::new(1.0, fade(P.accent)), StrokeKind::Inside);
            }
            p.text(r.center(), Align2::CENTER_CENTER, lang.label(), FontId::proportional(13.0), fade(color));
            if clickable && resp.clicked() {
                new_lang = Some(lang);
            }
        }

        if let Some(i) = chosen {
            if i != self.lesson {
                self.lesson = i;
                self.switch();
            }
        }
        if let Some(l) = new_lang {
            if l != self.lang {
                self.lang = l;
                self.switch();
            }
        }
    }

    fn lesson_line(&self, ui: &mut egui::Ui, w: f32) {
        let info = &self.lang.lessons()[self.lesson];
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 20.0), Sense::hover());
        let p = ui.painter();
        let mut job = LayoutJob::default();
        let font = FontId::proportional(13.5);
        let fmt = |color| TextFormat { font_id: font.clone(), color, ..Default::default() };
        job.append(&format!("Урок {} · ", self.lesson + 1), 0.0, fmt(P.dim));
        job.append(info.title, 0.0, fmt(P.text));
        if !info.new.is_empty() {
            let label = if self.lesson == 0 { "   буквы " } else { "   новые " };
            job.append(label, 0.0, fmt(P.dim));
            let letters: Vec<String> = info.new.chars().map(String::from).collect();
            job.append(&letters.join(" "), 0.0, fmt(P.accent));
        }
        let galley = p.layout_job(job);
        p.galley(Pos2::new(r.left(), r.center().y - galley.size().y / 2.0), galley, P.text);
        p.text(r.right_center(), Align2::RIGHT_CENTER, self.lang.label(), FontId::proportional(13.5), P.accent);
        let (line, _) = ui.allocate_exact_size(Vec2::new(w, 8.0), Sense::hover());
        ui.painter().hline(line.x_range(), line.center().y, Stroke::new(1.0, P.faint));
    }

    fn text(&self, ui: &mut egui::Ui, w: f32) {
        let t = &self.test;
        let mut job = LayoutJob::default();
        job.wrap.max_width = w;
        for (i, &c) in t.target.iter().enumerate() {
            let mut fmt = TextFormat {
                font_id: FontId::monospace(22.0),
                color: P.dim,
                line_height: Some(34.0),
                ..Default::default()
            };
            let mut shown = c;
            if i < t.typed.len() {
                if t.typed[i] == c {
                    fmt.color = P.text;
                } else {
                    fmt.color = P.error;
                    fmt.underline = Stroke::new(2.0, P.error);
                    if c == ' ' {
                        shown = '·';
                    }
                }
            } else if i == t.typed.len() {
                fmt.color = P.bg;
                fmt.background = P.accent;
            }
            job.append(&shown.to_string(), 0.0, fmt);
        }
        ui.label(job);
    }

    fn progress(&self, ui: &mut egui::Ui, w: f32) {
        let t = &self.test;
        let frac = t.typed.len() as f32 / t.target.len().max(1) as f32;
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 16.0), Sense::hover());
        let bar = Rect::from_min_max(Pos2::new(r.left(), r.center().y - 2.0), Pos2::new(r.right() - 46.0, r.center().y + 2.0));
        let p = ui.painter();
        p.rect_filled(bar, CornerRadius::same(2), P.faint);
        let mut done = bar;
        done.set_width(bar.width() * frac);
        p.rect_filled(done, CornerRadius::same(2), P.accent);
        p.text(r.right_center(), Align2::RIGHT_CENTER, format!("{:.0}%", frac * 100.0), FontId::proportional(12.5), P.dim);
    }

    fn stats(&self, ui: &mut egui::Ui) {
        let t = &self.test;
        let secs = t.elapsed().as_secs();
        let items = [
            (format!("{:.0}", t.cpm()), "зн/мин"),
            (format!("{:.0}", t.cpm() / 5.0), "сл/мин"),
            (format!("{:.0}%", t.accuracy()), "точность"),
            (format!("{}:{:02}", secs / 60, secs % 60), "время"),
        ];
        let value_color = if t.done() { P.accent } else { P.text };
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 46.0), Sense::hover());
        let cell = r.width() / items.len() as f32;
        let p = ui.painter();
        for (i, (value, label)) in items.into_iter().enumerate() {
            let cx = r.left() + cell * (i as f32 + 0.5);
            p.text(Pos2::new(cx, r.top() + 14.0), Align2::CENTER_CENTER, value, FontId::proportional(24.0), value_color);
            p.text(Pos2::new(cx, r.top() + 38.0), Align2::CENTER_CENTER, label, FontId::proportional(11.5), P.dim);
        }
    }

    fn message(&self, ui: &mut egui::Ui) {
        let t = &self.test;
        let (text, color) = if t.wrong_layout {
            (self.lang.layout_hint().to_string(), P.warn)
        } else if t.done() {
            let tail = if self.lesson + 1 == LESSON_COUNT {
                ""
            } else if t.accuracy() >= 95.0 {
                " · точность от 95% — можно к следующему уроку"
            } else {
                " · цель для следующего урока — точность от 95%"
            };
            (format!("Готово! Enter — ещё раз{tail}"), P.accent)
        } else if t.started.is_none() {
            ("Начните печатать — таймер запустится с первой клавиши".to_string(), P.dim)
        } else {
            (String::new(), P.dim)
        };
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), Sense::hover());
        ui.painter().text(r.center(), Align2::CENTER_CENTER, text, FontId::proportional(13.5), color);
    }

    fn finger_hint(&self, ui: &mut egui::Ui) {
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), Sense::hover());
        let Some(c) = self.test.target.get(self.test.typed.len()).copied() else { return };
        let Some(f) = finger_of(self.lang, c) else { return };
        let key = if c == ' ' { "Пробел — ".to_string() } else { format!("Клавиша «{c}» — ") };
        let mut job = LayoutJob::default();
        let font = FontId::proportional(16.0);
        job.append(&key, 0.0, TextFormat { font_id: font.clone(), color: P.dim, ..Default::default() });
        job.append(f.name(), 0.0, TextFormat { font_id: font, color: P.fingers[f.group()], ..Default::default() });
        let galley = ui.painter().layout_job(job);
        let pos = r.center() - galley.size() / 2.0;
        ui.painter().galley(pos, galley, P.text);
    }

    fn keyboard(&self, ui: &mut egui::Ui) {
        let unit = KEY + GAP;
        let offsets = [0.0, 0.35, 0.85];
        let width = 12.0 * unit - GAP;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 4.0 * unit - GAP), Sense::hover());
        let x0 = rect.center().x - width / 2.0;
        let painter = ui.painter();
        let next = self.test.target.get(self.test.typed.len()).copied();
        let hot = next.and_then(|c| locate(self.lang, c));
        let allowed = self.lang.allowed(self.lesson);
        let new = self.lang.lessons()[self.lesson].new;
        let radius = CornerRadius::same(7);

        for (r, keys) in self.lang.rows().iter().enumerate() {
            for (col, ch) in keys.chars().enumerate() {
                let min = Pos2::new(x0 + (offsets[r] + col as f32) * unit, rect.top() + r as f32 * unit);
                let kr = Rect::from_min_size(min, Vec2::splat(KEY));
                let fc = P.fingers[Finger::for_column(col).group()];
                let in_lesson = allowed.as_ref().map_or(true, |a| a.contains(&ch));
                let is_hot = hot == Some((r, col));
                let (fill, label) = if is_hot {
                    (fc, P.bg)
                } else if in_lesson {
                    (fc.gamma_multiply(0.13), fc)
                } else {
                    (P.surface, P.faint)
                };
                painter.rect_filled(kr, radius, fill);
                if !is_hot && new.contains(ch) {
                    painter.rect_stroke(kr, radius, Stroke::new(1.5, fc), StrokeKind::Inside);
                }
                painter.text(kr.center(), Align2::CENTER_CENTER, ch, FontId::proportional(16.0), label);
                if r == 1 && (col == 3 || col == 6) {
                    let y = kr.bottom() - 7.0;
                    painter.hline((kr.center().x - 4.0)..=(kr.center().x + 4.0), y, Stroke::new(2.0, label));
                }
            }
        }

        let sr = Rect::from_min_size(Pos2::new(x0 + 3.5 * unit, rect.top() + 3.0 * unit), Vec2::new(6.0 * unit - GAP, KEY));
        let tc = P.fingers[Finger::Thumb.group()];
        let (fill, label) = if next == Some(' ') { (tc, P.bg) } else { (tc.gamma_multiply(0.13), tc) };
        painter.rect_filled(sr, radius, fill);
        painter.text(sr.center(), Align2::CENTER_CENTER, "пробел", FontId::proportional(14.0), label);
    }

    fn footer(&self, ui: &egui::Ui, full: Rect) {
        let bar = Rect::from_min_max(Pos2::new(full.left(), full.bottom() - FOOTER), full.max);
        let p = ui.painter();
        p.rect_filled(bar, CornerRadius::ZERO, P.surface.gamma_multiply(0.5));
        p.hline(bar.x_range(), bar.top(), Stroke::new(1.0, P.faint));

        let left = Vec2::new(-1.0, 0.0);
        let right = Vec2::new(1.0, 0.0);
        let up = Vec2::new(0.0, -1.0);
        let items: [(&[Chip], &str); 5] = [
            (&[Chip::Arrow(left), Chip::Arrow(right)], "урок"),
            (&[Chip::Arrow(up)], "другие слова"),
            (&[Chip::Text("Tab")], "раскладка"),
            (&[Chip::Text("Enter")], "ещё раз"),
            (&[Chip::Text("Esc")], "выход"),
        ];
        let cell = bar.width() / items.len() as f32;
        let font = FontId::proportional(12.5);
        for (i, (chips, label)) in items.iter().enumerate() {
            let galley = p.layout_no_wrap(label.to_string(), font.clone(), P.dim);
            let chip_w: Vec<f32> = chips
                .iter()
                .map(|c| match c {
                    Chip::Text(s) => p.layout_no_wrap(s.to_string(), font.clone(), P.text).size().x + 12.0,
                    Chip::Arrow(_) => 20.0,
                })
                .collect();
            let total = chip_w.iter().sum::<f32>() + 4.0 * (chips.len() as f32 - 1.0) + 8.0 + galley.size().x;
            let mut x = bar.left() + cell * (i as f32 + 0.5) - total / 2.0;
            let cy = bar.center().y;
            for (chip, w) in chips.iter().zip(chip_w) {
                let r = Rect::from_min_size(Pos2::new(x, cy - 10.0), Vec2::new(w, 20.0));
                p.rect_filled(r, CornerRadius::same(5), P.surface);
                p.rect_stroke(r, CornerRadius::same(5), Stroke::new(1.0, P.faint), StrokeKind::Inside);
                match chip {
                    Chip::Text(s) => {
                        p.text(r.center(), Align2::CENTER_CENTER, *s, font.clone(), P.text);
                    }
                    Chip::Arrow(d) => {
                        let c = r.center();
                        let n = Vec2::new(-d.y, d.x);
                        let pts = vec![c + *d * 4.0, c - *d * 3.0 + n * 4.0, c - *d * 3.0 - n * 4.0];
                        p.add(PathShape::convex_polygon(pts, P.text, Stroke::NONE));
                    }
                }
                x += w + 4.0;
            }
            p.galley(Pos2::new(x + 4.0, cy - galley.size().y / 2.0), galley, P.dim);
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_input(&ctx);
        if self.typing() {
            ctx.request_repaint_after(Duration::from_millis(200));
        }

        let frame = egui::Frame::NONE.fill(P.bg);
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            let full = ui.max_rect();
            self.burger(ui, full);
            self.logo(ui, full);
            self.footer(ui, full);

            let area = Rect::from_min_max(
                Pos2::new(full.left(), full.top() + TOP_BAR),
                Pos2::new(full.right(), full.bottom() - FOOTER),
            );
            let w = COLUMN.min(area.width() - 2.0 * MARGIN);
            let top = area.top() + ((area.height() - self.content_h) / 2.0).max(0.0);
            let col = Rect::from_min_size(Pos2::new(area.center().x - w / 2.0, top), Vec2::new(w, area.bottom() - top));
            let resp = ui.scope_builder(egui::UiBuilder::new().max_rect(col), |ui| {
                ui.set_width(w);
                ui.spacing_mut().item_spacing.y = 6.0;
                self.lesson_line(ui, w);
                ui.add_space(12.0);
                self.text(ui, w);
                ui.add_space(12.0);
                self.progress(ui, w);
                ui.add_space(4.0);
                self.stats(ui);
                self.message(ui);
                ui.add_space(4.0);
                self.finger_hint(ui);
                ui.add_space(6.0);
                self.keyboard(ui);
            });
            let h = resp.response.rect.height();
            if (h - self.content_h).abs() > 1.0 {
                self.content_h = h;
                ctx.request_repaint();
            }

            self.menu(ui, full);
        });
    }
}

fn main() -> eframe::Result {
    let (lang, lesson) = load_state();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("FastWord")
            .with_app_id("fastword-gui")
            .with_inner_size([980.0, 640.0])
            .with_min_inner_size([760.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "FastWord",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(App::new(lang, lesson)))
        }),
    )
}
