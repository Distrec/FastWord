use std::time::Duration;

use eframe::egui::{
    self, text::LayoutJob, Align, Align2, Color32, CornerRadius, FontId, Key, Layout, Pos2, Rect,
    RichText, Sense, Stroke, StrokeKind, TextFormat, Vec2,
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

const COLUMN: f32 = 760.0;
const KEY: f32 = 38.0;
const GAP: f32 = 6.0;

struct App {
    lang: Lang,
    lesson: usize,
    rng: Rng,
    test: Test,
    content_h: f32,
}

impl App {
    fn new(lang: Lang, lesson: usize) -> Self {
        let mut rng = Rng::new();
        let test = Test::new(make_text(lang, lesson, &mut rng));
        App { lang, lesson, rng, test, content_h: 0.0 }
    }

    fn new_test(&mut self) {
        self.test = Test::new(make_text(self.lang, self.lesson, &mut self.rng));
    }

    fn switch(&mut self) {
        save_state(self.lang, self.lesson);
        self.new_test();
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

    fn header(&self, ui: &mut egui::Ui, w: f32) {
        let info = &self.lang.lessons()[self.lesson];
        ui.horizontal(|ui| {
            ui.label(RichText::new("FastWord").color(P.dim).size(15.0));
            ui.label(RichText::new("·").color(P.faint).size(15.0));
            ui.label(RichText::new(self.lang.label()).color(P.accent).size(15.0).strong());
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("Урок {}/{LESSON_COUNT} · {}", self.lesson + 1, info.title))
                        .color(P.text)
                        .size(15.0),
                );
            });
        });
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 10.0), Sense::hover());
        ui.painter().hline(r.x_range(), r.center().y, Stroke::new(1.0, P.faint));
        let letters: Vec<String> = info.new.chars().map(String::from).collect();
        let caption = if letters.is_empty() {
            "вся клавиатура".to_string()
        } else {
            let label = if self.lesson == 0 { "буквы" } else { "новые буквы" };
            format!("{label}: {}", letters.join(" "))
        };
        ui.label(RichText::new(caption).color(P.dim).size(13.0));
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
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(Vec2::new(w - 52.0, 4.0), Sense::hover());
            let p = ui.painter();
            p.rect_filled(r, CornerRadius::same(2), P.faint);
            let mut done = r;
            done.set_width(r.width() * frac);
            p.rect_filled(done, CornerRadius::same(2), P.accent);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(format!("{:.0}%", frac * 100.0)).color(P.dim).size(13.0));
            });
        });
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
        ui.columns(items.len(), |cols| {
            for (col, (value, label)) in cols.iter_mut().zip(items) {
                col.vertical_centered(|ui| {
                    ui.label(RichText::new(value).size(26.0).color(value_color));
                    ui.label(RichText::new(label).size(12.0).color(P.dim));
                });
            }
        });
    }

    fn message(&self, ui: &mut egui::Ui) {
        let t = &self.test;
        let (text, color) = if t.wrong_layout {
            (self.lang.layout_hint().to_string(), P.warn)
        } else if t.done() {
            let tail = if self.lesson + 1 == LESSON_COUNT {
                ""
            } else if t.accuracy() >= 95.0 {
                " · точность от 95% — можно дальше (вправо)"
            } else {
                " · цель для следующего урока — точность от 95%"
            };
            (format!("Готово! Enter — ещё раз{tail}"), P.accent)
        } else if t.started.is_none() {
            ("Начните печатать — таймер запустится с первой клавиши".to_string(), P.dim)
        } else {
            (String::new(), P.dim)
        };
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), Sense::hover());
        ui.painter().text(r.center(), Align2::CENTER_CENTER, text, FontId::proportional(14.0), color);
    }

    fn finger_hint(&self, ui: &mut egui::Ui) {
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::hover());
        let Some(c) = self.test.target.get(self.test.typed.len()).copied() else { return };
        let Some(f) = finger_of(self.lang, c) else { return };
        let key = if c == ' ' { "Пробел — ".to_string() } else { format!("Клавиша «{c}» — ") };
        let mut job = LayoutJob::default();
        let font = FontId::proportional(17.0);
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
        let radius = CornerRadius::same(8);

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
                painter.text(kr.center(), Align2::CENTER_CENTER, ch, FontId::proportional(18.0), label);
                if r == 1 && (col == 3 || col == 6) {
                    let y = kr.bottom() - 8.0;
                    painter.hline((kr.center().x - 5.0)..=(kr.center().x + 5.0), y, Stroke::new(2.0, label));
                }
            }
        }

        let space_w = 6.0 * unit - GAP;
        let sr = Rect::from_min_size(
            Pos2::new(x0 + 3.5 * unit, rect.top() + 3.0 * unit),
            Vec2::new(space_w, KEY),
        );
        let tc = P.fingers[Finger::Thumb.group()];
        let space_hot = next == Some(' ');
        let (fill, label) = if space_hot { (tc, P.bg) } else { (tc.gamma_multiply(0.13), tc) };
        painter.rect_filled(sr, radius, fill);
        painter.text(sr.center(), Align2::CENTER_CENTER, "пробел", FontId::proportional(15.0), label);
    }

    fn footer(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("Влево/вправо — урок · Вверх — другие слова · Tab — раскладка · Esc — выход")
                    .color(P.dim)
                    .size(12.0),
            );
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_input(&ctx);
        if self.test.started.is_some() && !self.test.done() {
            ctx.request_repaint_after(Duration::from_millis(200));
        }

        let frame = egui::Frame::NONE.fill(P.bg).inner_margin(24.0);
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            let w = ui.available_width().min(COLUMN);
            let side = ((ui.available_width() - w) / 2.0).max(0.0);
            let top = ((ui.available_height() - self.content_h) / 2.0).max(0.0);
            ui.add_space(top);
            let resp = ui.horizontal_top(|ui| {
                ui.add_space(side);
                ui.vertical(|ui| {
                    ui.set_width(w);
                    ui.spacing_mut().item_spacing.y = 6.0;
                    self.header(ui, w);
                    ui.add_space(16.0);
                    self.text(ui, w);
                    ui.add_space(14.0);
                    self.progress(ui, w);
                    ui.add_space(8.0);
                    self.stats(ui);
                    ui.add_space(6.0);
                    self.message(ui);
                    ui.add_space(10.0);
                    self.finger_hint(ui);
                    ui.add_space(8.0);
                    self.keyboard(ui);
                    ui.add_space(12.0);
                    self.footer(ui);
                })
                .response
            });
            let h = resp.inner.rect.height();
            if (h - self.content_h).abs() > 1.0 {
                self.content_h = h;
                ctx.request_repaint();
            }
        });
    }
}

fn main() -> eframe::Result {
    let (lang, lesson) = load_state();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("FastWord")
            .with_app_id("fastword-gui")
            .with_inner_size([860.0, 640.0])
            .with_min_inner_size([720.0, 560.0]),
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
