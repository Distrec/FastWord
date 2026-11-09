#![windows_subsystem = "windows"]

use std::time::{Duration, Instant};

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
    sub: Color32,
    error: Color32,
    warn: Color32,
    line: Color32,
    hatch: Color32,
}

const P: Palette = Palette {
    bg: Color32::from_rgb(0x00, 0x00, 0x00),
    surface: Color32::from_rgb(0x16, 0x16, 0x16),
    text: Color32::from_rgb(0xfc, 0xfc, 0xfc),
    dim: Color32::from_rgb(0x75, 0x75, 0x75),
    faint: Color32::from_rgb(0x40, 0x40, 0x40),
    accent: Color32::from_rgb(0xf8, 0xf8, 0xf8),
    sub: Color32::from_rgb(0x9c, 0x9c, 0x9c),
    error: Color32::from_rgb(0xe0, 0x6c, 0x75),
    warn: Color32::from_rgb(0xe5, 0xc0, 0x7b),
    line: Color32::from_rgb(0x48, 0x48, 0x48),
    hatch: Color32::from_rgb(0x38, 0x38, 0x38),
};

fn section(p: &egui::Painter, left: f32, right: f32, y: f32, label: &str, color: Color32, line: Color32) -> f32 {
    let g = p.layout_no_wrap(label.to_string(), FontId::monospace(10.5), color);
    let h = g.size().y;
    let end = left + g.size().x;
    p.galley(Pos2::new(left, y), g, color);
    if right > end + 8.0 {
        p.hline((end + 8.0)..=right, y + h / 2.0, Stroke::new(1.0, line));
    }
    end
}

fn hatch(p: &egui::Painter, r: Rect, group: usize, color: Color32) {
    let p = p.with_clip_rect(r);
    let s = Stroke::new(1.0, color);
    let (w, h) = (r.width(), r.height());
    match group {
        0 => {
            let mut d = -h;
            while d < w {
                p.line_segment([Pos2::new(r.left() + d, r.bottom()), Pos2::new(r.left() + d + h, r.top())], s);
                d += 4.0;
            }
        }
        1 => {
            let mut y = r.top() + 3.0;
            while y < r.bottom() {
                let mut x = r.left() + 3.0;
                while x < r.right() {
                    p.circle_filled(Pos2::new(x, y), 0.9, color);
                    x += 5.0;
                }
                y += 5.0;
            }
        }
        2 => {
            let mut y = r.top() + 2.5;
            while y < r.bottom() {
                p.hline(r.x_range(), y, s);
                y += 4.0;
            }
        }
        3 => {
            let mut d = -h;
            while d < w {
                p.line_segment([Pos2::new(r.left() + d, r.top()), Pos2::new(r.left() + d + h, r.bottom())], s);
                d += 6.0;
            }
        }
        _ => {
            let mut x = r.left() + 2.5;
            while x < r.right() {
                p.vline(x, r.y_range(), s);
                x += 4.0;
            }
        }
    }
}

const COLUMN: f32 = 580.0;
const KEY: f32 = 34.0;
const GAP: f32 = 6.0;
const TOP_BAR: f32 = 56.0;
const FOOTER: f32 = 44.0;
const MENU_W: f32 = 172.0;
const MARGIN: f32 = 20.0;

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Boot,
    Start,
    Practice,
}

struct App {
    lang: Lang,
    lesson: usize,
    mix: Mix,
    rng: Rng,
    test: Test,
    content_h: f32,
    screen: Screen,
    boot: Instant,
    shown: Instant,
    tab: usize,
    pick: usize,
    grain: Option<egui::TextureHandle>,
    dots: Option<egui::TextureHandle>,
}

const BOOT_LEN: f32 = 2.6;
const FADE: f32 = 0.35;

fn hash(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263) ^ seed.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

fn grain_image() -> egui::ColorImage {
    let n = 256;
    let mut px = Vec::with_capacity(n * n * 4);
    for y in 0..n {
        for x in 0..n {
            let a = (hash(x as u32, y as u32, 7) * 22.0) as u8;
            px.extend_from_slice(&[255, 255, 255, a]);
        }
    }
    egui::ColorImage::from_rgba_unmultiplied([n, n], &px)
}

fn dots_image() -> egui::ColorImage {
    let (w, h) = (480usize, 320usize);
    let g = |u: f32, v: f32, cu: f32, cv: f32, su: f32, sv: f32| (-((u - cu).powi(2) / su + (v - cv).powi(2) / sv)).exp();
    let mut px = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let u = x as f32 / w as f32;
            let v = y as f32 / h as f32;
            let blob = g(u, v, 0.52, 0.42, 0.035, 0.07);
            let side = 0.65 * g(u, v, 0.88, 0.78, 0.02, 0.03);
            let d = v - (1.05 - 0.95 * u);
            let ridge = 0.75 * (-(d * d) / 0.004).exp() * ((u - 0.2) / 0.4).clamp(0.0, 1.0);
            let f = (blob.max(side).max(ridge) * 0.92).min(1.0);
            let on = hash(x as u32, y as u32, 3) < f.powf(2.1);
            let c = (90.0 + 130.0 * f) as u8;
            px.extend_from_slice(&if on { [c, c, c, 255] } else { [0, 0, 0, 0] });
        }
    }
    egui::ColorImage::from_rgba_unmultiplied([w, h], &px)
}

enum Chip {
    Text(&'static str),
    Arrow(Vec2),
}

impl App {
    fn new(lang: Lang, lesson: usize, mix: Mix) -> Self {
        let mut rng = Rng::new();
        let test = Test::for_mode_mix(lang, lesson, mix, &mut rng);
        App {
            lang,
            lesson,
            mix,
            rng,
            test,
            content_h: 0.0,
            screen: Screen::Boot,
            boot: Instant::now(),
            shown: Instant::now(),
            tab: if is_endless(lesson) { 1 } else { 0 },
            pick: if is_endless(lesson) { if mix == Mix::All { 1 } else { 0 } } else { lesson },
            grain: None,
            dots: None,
        }
    }

    fn go(&mut self, screen: Screen) {
        self.screen = screen;
        self.shown = Instant::now();
        if screen == Screen::Start {
            self.tab = if is_endless(self.lesson) { 1 } else { 0 };
            self.pick = if self.tab == 1 { if self.mix == Mix::All { 1 } else { 0 } } else { self.lesson };
        }
    }

    fn picks(&self) -> usize {
        if self.tab == 0 { LESSON_COUNT } else { 2 }
    }

    fn launch(&mut self) {
        if self.tab == 0 {
            self.lesson = self.pick.min(LESSON_COUNT - 1);
        } else {
            self.lesson = ENDLESS;
            self.mix = if self.pick == 1 { Mix::All } else { Mix::Words };
        }
        self.switch();
        self.go(Screen::Practice);
    }

    fn textures(&mut self, ctx: &egui::Context) {
        if self.grain.is_none() {
            self.grain = Some(ctx.load_texture("grain", grain_image(), egui::TextureOptions::NEAREST_REPEAT));
        }
        if self.dots.is_none() {
            self.dots = Some(ctx.load_texture("dots", dots_image(), egui::TextureOptions::NEAREST));
        }
    }

    fn draw_grain(&self, p: &egui::Painter, r: Rect, now: f32) {
        if let Some(tex) = &self.grain {
            let k = (now * 12.0) as u32;
            let off = Vec2::new(hash(k, 1, 9), hash(k, 2, 9));
            let uv = Rect::from_min_size(off.to_pos2(), r.size() / 256.0);
            p.image(tex.id(), r, uv, Color32::WHITE);
        }
    }

    fn draw_dots(&self, p: &egui::Painter, r: Rect, now: f32, alpha: f32) {
        if let Some(tex) = &self.dots {
            let drift = (now * 0.02).min(0.06);
            let uv = Rect::from_min_max(Pos2::new(drift, drift * 0.5), Pos2::new(1.0 - 0.06 + drift, 1.0 - 0.03 + drift * 0.5));
            p.image(tex.id(), r, uv, Color32::WHITE.gamma_multiply(alpha));
        }
    }

    fn splash(&self, ui: &egui::Ui, full: Rect) {
        let p = ui.painter().clone();
        let now = self.boot.elapsed().as_secs_f32();
        let frac = (now / BOOT_LEN).min(1.0);
        let fade_in = (now / 0.5).min(1.0);
        let light = Color32::from_gray(0xc8);
        {
            let x0 = full.left() + full.width() * 0.30;
            let r = Rect::from_min_max(Pos2::new(x0, full.top()), full.max);
            self.draw_dots(&p, r, now, fade_in * 0.55);
            let edge = 140.0;
            for i in 0..28 {
                let k = i as f32 / 28.0;
                let band = Rect::from_min_size(Pos2::new(x0 + k * edge, full.top()), Vec2::new(edge / 28.0 + 1.0, full.height()));
                p.rect_filled(band, CornerRadius::ZERO, P.bg.gamma_multiply(1.0 - k));
            }
        }
        {
            let frame = Rect::from_center_size(full.center(), Vec2::new((full.width() - 120.0).min(860.0), 420.0));
            let sub = P.sub.gamma_multiply(fade_in);
            let small = FontId::monospace(12.0);
            p.text(frame.left_top(), Align2::LEFT_TOP, format!("Ver /\nV.{}", env!("CARGO_PKG_VERSION")), small.clone(), sub);
            p.text(frame.left_top() + Vec2::new(150.0, 18.0), Align2::LEFT_TOP, "TOUCH TYPING TRAINER", FontId::monospace(13.0), sub);
            p.text(frame.right_top() + Vec2::new(0.0, 18.0), Align2::RIGHT_TOP, self.lang.label(), FontId::monospace(13.0), P.text.gamma_multiply(fade_in));

            let word = "FastWord";
            let big = FontId::monospace(118.0);
            let typed = (((now - 0.3) / 0.16).floor() + 1.0).clamp(0.0, word.len() as f32) as usize;
            let cw = p.layout_no_wrap("M".into(), big.clone(), light).size().x;
            let wx = frame.left() - 6.0;
            let wy = frame.center().y;
            for (i, ch) in word.chars().take(typed).enumerate() {
                p.text(Pos2::new(wx + i as f32 * cw, wy), Align2::LEFT_CENTER, ch, big.clone(), light);
            }
            if (now * 2.5) as i32 % 2 == 0 && now < BOOT_LEN - 0.3 {
                let cur = Rect::from_min_size(Pos2::new(wx + typed as f32 * cw + 4.0, wy - 46.0), Vec2::new(cw * 0.55, 96.0));
                p.rect_filled(cur, CornerRadius::ZERO, light);
            }

            let by = frame.bottom() - 40.0;
            p.text(Pos2::new(frame.left(), by), Align2::LEFT_TOP, "© 2026 DISTREC\nALL KEYS RESERVED.", small.clone(), sub);
            p.text(Pos2::new(frame.right(), by), Align2::RIGHT_TOP, format!("{:03}%", (frac * 100.0) as u32), small.clone(), P.text.gamma_multiply(fade_in));
            let len = 16.0 + (frame.width() - 16.0) * frac;
            p.hline(frame.left()..=(frame.left() + len), frame.bottom() + 6.0, Stroke::new(1.0, P.text));
        }
        self.draw_grain(&p, full, now);
    }

    fn start(&mut self, ui: &mut egui::Ui, full: Rect) {
        let p = ui.painter().clone();
        let now = self.shown.elapsed().as_secs_f32();
        self.logo(ui, full);
        let mut new_lang = None;
        for (i, lang) in [Lang::Ru, Lang::En].into_iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(full.right() - MARGIN - 94.0 + i as f32 * 50.0, full.top() + 17.0), Vec2::new(44.0, 24.0));
            let resp = ui.interact(r, ui.scope_id().with(("slang", i)), Sense::CLICK);
            let on = lang == self.lang;
            let color = if on || resp.hovered() { P.text } else { P.dim };
            p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, if on { P.text } else { P.line }), StrokeKind::Inside);
            p.text(r.center(), Align2::CENTER_CENTER, lang.label(), FontId::monospace(13.0), color);
            if resp.clicked() {
                new_lang = Some(lang);
            }
        }

        let area = Rect::from_min_max(Pos2::new(full.left(), full.top() + 56.0), Pos2::new(full.right(), full.bottom() - FOOTER));
        let box_w = (area.width() - 2.0 * MARGIN).min(780.0);
        let box_h = 360.0;
        let frame = Rect::from_center_size(area.center(), Vec2::new(box_w, box_h));
        let btn_w = 260.0;
        let gap = 16.0;
        let btn_h = (box_h - gap) / 2.0;
        let modes = [
            ("01 / MODE", "ТРЕНИРОВКИ", format!("{} уроков · по рядам", LESSON_COUNT)),
            ("02 / MODE", "БЕСКОНЕЧНЫЙ", "∞ · слова без конца".to_string()),
        ];
        let mut new_tab = None;
        for (i, (tag, title, desc)) in modes.iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(frame.left(), frame.top() + i as f32 * (btn_h + gap)), Vec2::new(btn_w, btn_h));
            let resp = ui.interact(r, ui.scope_id().with(("mode", i)), Sense::CLICK);
            let on = self.tab == i;
            let border = if on { P.text } else if resp.hovered() { P.dim } else { P.line };
            p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, border), StrokeKind::Inside);
            let inner = r.shrink(18.0);
            p.text(inner.left_top(), Align2::LEFT_TOP, *tag, FontId::monospace(10.5), P.dim);
            let mark = Rect::from_min_size(Pos2::new(inner.right() - 8.0, inner.top() + 2.0), Vec2::splat(8.0));
            if on {
                p.rect_filled(mark, CornerRadius::ZERO, P.text);
            } else {
                p.rect_stroke(mark, CornerRadius::ZERO, Stroke::new(1.0, P.line), StrokeKind::Inside);
            }
            p.text(inner.left_center() + Vec2::new(0.0, -4.0), Align2::LEFT_CENTER, *title, FontId::monospace(24.0), if on || resp.hovered() { P.text } else { P.dim });
            p.text(inner.left_bottom(), Align2::LEFT_BOTTOM, desc, FontId::monospace(11.5), P.dim);
            if on {
                p.text(inner.right_bottom(), Align2::RIGHT_BOTTOM, "→", FontId::monospace(16.0), P.text);
            }
            if resp.clicked() {
                new_tab = Some(i);
            }
        }

        let list = Rect::from_min_max(Pos2::new(frame.left() + btn_w + 48.0, frame.top()), frame.max);
        let btn_mid = frame.top() + self.tab as f32 * (btn_h + gap) + btn_h / 2.0;
        let head_y = list.top() + 7.0;
        let jx = frame.left() + btn_w + 24.0;
        let s = Stroke::new(1.0, P.dim);
        p.hline((frame.left() + btn_w)..=jx, btn_mid, s);
        p.vline(jx, head_y.min(btn_mid)..=head_y.max(btn_mid), s);
        p.hline(jx..=(list.left() - 6.0), head_y, s);
        p.circle_filled(Pos2::new(list.left() - 6.0, head_y), 2.5, P.text);

        let label = if self.tab == 0 { "LESSONS" } else { "ENDLESS" };
        section(&p, list.left(), list.right(), list.top(), label, P.dim, P.line);
        let mut y = list.top() + 30.0;
        let mut clicked = None;
        let rows: Vec<(String, String)> = if self.tab == 0 {
            (0..LESSON_COUNT)
                .map(|i| {
                    let l = self.lang.lesson(i);
                    (l.title.to_string(), l.new.chars().map(|c| c.to_string()).collect::<Vec<_>>().join(" "))
                })
                .collect()
        } else {
            vec![
                ("Слова".to_string(), "только настоящие слова от 3 букв".to_string()),
                ("Всё подряд".to_string(), "слова и сочетания букв".to_string()),
            ]
        };
        if self.tab == 1 {
            p.text(Pos2::new(list.left(), y), Align2::LEFT_TOP, "Текст не кончается. 3 секунды без нажатий — пауза.", FontId::proportional(12.5), P.dim);
            y += 34.0;
        }
        for (i, (title, extra)) in rows.iter().enumerate() {
            let row = Rect::from_min_size(Pos2::new(list.left(), y), Vec2::new(list.width(), 36.0));
            let resp = ui.interact(row, ui.scope_id().with(("pick", self.tab, i)), Sense::CLICK);
            if resp.hovered() && resp.ctx.input(|i| i.pointer.delta() != Vec2::ZERO) {
                self.pick = i;
            }
            let on = self.pick == i;
            if on {
                p.rect_filled(Rect::from_min_size(row.min + Vec2::new(0.0, 10.0), Vec2::new(3.0, 16.0)), CornerRadius::ZERO, P.text);
            }
            p.text(row.left_center() + Vec2::new(12.0, 0.0), Align2::LEFT_CENTER, format!("{:02}", i + 1), FontId::monospace(12.0), if on { P.text } else { P.faint });
            p.text(row.left_center() + Vec2::new(44.0, 0.0), Align2::LEFT_CENTER, title, FontId::proportional(14.0), if on { P.text } else { P.dim });
            p.text(row.right_center(), Align2::RIGHT_CENTER, extra, FontId::monospace(11.5), if on { P.sub } else { P.faint });
            p.hline(row.x_range(), row.bottom(), Stroke::new(1.0, P.surface.gamma_multiply(1.4)));
            if resp.clicked() {
                clicked = Some(i);
            }
            y += 36.0;
        }
        p.text(Pos2::new(list.left(), frame.bottom()), Align2::LEFT_BOTTOM, "Enter — начать", FontId::monospace(11.0), P.faint);

        if let Some(i) = new_tab {
            if i != self.tab {
                self.tab = i;
                self.pick = 0;
            }
        }
        if let Some(l) = new_lang {
            if l != self.lang {
                self.lang = l;
                self.switch();
            }
        }
        if let Some(i) = clicked {
            self.pick = i;
            self.launch();
        }
        self.draw_grain(&p, full, now * 0.0);
    }

    fn new_test(&mut self) {
        self.test = Test::for_mode_mix(self.lang, self.lesson, self.mix, &mut self.rng);
    }

    fn switch(&mut self) {
        save_state_mix(self.lang, self.lesson, self.mix);
        self.new_test();
    }

    fn set_mix(&mut self, mix: Mix) {
        if mix != self.mix {
            self.mix = mix;
            self.switch();
        }
    }

    fn typing(&self) -> bool {
        self.test.started.is_some() && !self.test.done() && !(self.test.endless && self.test.paused())
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|i| i.events.clone());
        if self.screen == Screen::Boot {
            for e in &events {
                match e {
                    egui::Event::Key { key: Key::Escape, pressed: true, .. } => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                    egui::Event::Key { pressed: true, .. } | egui::Event::PointerButton { pressed: true, .. } => self.go(Screen::Start),
                    _ => {}
                }
            }
            return;
        }
        if self.screen == Screen::Start {
            for e in events {
                if let egui::Event::Key { key, pressed: true, .. } = e {
                    match key {
                        Key::Escape => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                        Key::Tab => {
                            self.lang = self.lang.toggle();
                            self.switch();
                        }
                        Key::ArrowLeft | Key::ArrowRight => {
                            self.tab = 1 - self.tab;
                            self.pick = 0;
                        }
                        Key::ArrowUp => self.pick = (self.pick + self.picks() - 1) % self.picks(),
                        Key::ArrowDown => self.pick = (self.pick + 1) % self.picks(),
                        Key::Enter => self.launch(),
                        _ => {}
                    }
                }
            }
            return;
        }
        for e in events {
            match e {
                egui::Event::Text(t) => {
                    for c in t.chars().filter(|c| !c.is_control()) {
                        self.test.type_char(c, self.lang);
                        self.test.top_up(self.lang, &mut self.rng);
                    }
                }
                egui::Event::Key { key, physical_key, pressed: true, modifiers, .. } => match key {
                    Key::Escape => self.go(Screen::Start),
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
                    Key::ArrowDown if is_endless(self.lesson) => self.set_mix(self.mix.toggle()),
                    Key::Enter if self.test.done() => self.new_test(),
                    _ if modifiers.ctrl && (key == Key::R || physical_key == Some(Key::R)) => self.new_test(),
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn logo(&self, ui: &egui::Ui, full: Rect) {
        let p = ui.painter();
        let cy = full.top() + 29.0;
        let font = FontId::proportional(18.0);
        let fast = p.layout_no_wrap("Fast".into(), font.clone(), P.text);
        let word = p.layout_no_wrap("Word".into(), font, P.sub);
        let mut x = full.left() + MARGIN;
        p.galley(Pos2::new(x, cy - fast.size().y / 2.0), fast.clone(), P.text);
        x += fast.size().x;
        p.galley(Pos2::new(x, cy - word.size().y / 2.0), word, P.sub);
    }

    fn menu(&mut self, ui: &mut egui::Ui, full: Rect) {
        let visible = !self.typing();
        let t = ui.ctx().animate_bool_with_time(ui.scope_id().with("menu"), visible, 0.18);
        if t <= 0.0 {
            return;
        }
        let fade = |c: Color32| c.gamma_multiply(t);
        let left = full.left() + MARGIN - (1.0 - t) * 16.0;
        let mut y = full.top() + 54.0;
        let clickable = t > 0.6;

        section(ui.painter(), left, left + MENU_W, y, "LESSONS", fade(P.dim), fade(P.line));
        y += 22.0;

        let mut chosen = None;
        let mut new_mix = None;
        for i in 0..MODE_COUNT {
            let lesson = self.lang.lesson(i);
            if is_endless(i) {
                y += 8.0;
            }
            let row = Rect::from_min_size(Pos2::new(left, y), Vec2::new(MENU_W, 26.0));
            let resp = ui.interact(row, ui.scope_id().with(("lesson", i)), Sense::CLICK);
            let current = i == self.lesson;
            let p = ui.painter();
            if current {
                let bar = Rect::from_min_size(row.min + Vec2::new(0.0, 6.0), Vec2::new(3.0, 14.0));
                p.rect_filled(bar, CornerRadius::ZERO, fade(P.accent));
            }
            let num_color = if current { P.text } else { P.faint };
            let text_color = if current || (clickable && resp.hovered()) { P.text } else { P.dim };
            p.text(row.left_center() + Vec2::new(11.0, 0.0), Align2::LEFT_CENTER, if is_endless(i) { "∞".to_string() } else { format!("{:02}", i + 1) }, FontId::monospace(12.0), fade(num_color));
            p.text(row.left_center() + Vec2::new(30.0, 0.0), Align2::LEFT_CENTER, lesson.title, FontId::proportional(12.0), fade(text_color));
            if clickable && resp.clicked() {
                chosen = Some(i);
            }
            y += 28.0;
            if is_endless(i) && current {
                y += 2.0;
                let w = (MENU_W - 6.0) / 2.0;
                for (k, mix) in [Mix::Words, Mix::All].into_iter().enumerate() {
                    let r = Rect::from_min_size(Pos2::new(left + k as f32 * (w + 6.0), y), Vec2::new(w, 24.0));
                    let resp = ui.interact(r, ui.scope_id().with(("mix", k)), Sense::CLICK);
                    let on = mix == self.mix;
                    let p = ui.painter();
                    let color = if on || (clickable && resp.hovered()) { P.text } else { P.dim };
                    let border = if on { P.text } else { P.line };
                    p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, fade(border)), StrokeKind::Inside);
                    p.text(r.center(), Align2::CENTER_CENTER, mix.label(), FontId::proportional(12.0), fade(color));
                    if clickable && resp.clicked() {
                        new_mix = Some(mix);
                    }
                }
                y += 30.0;
            }
        }

        y += 14.0;
        section(ui.painter(), left, left + MENU_W, y, "LAYOUT", fade(P.dim), fade(P.line));
        y += 20.0;
        let mut new_lang = None;
        for (i, lang) in [Lang::Ru, Lang::En].into_iter().enumerate() {
            let r = Rect::from_min_size(Pos2::new(left + i as f32 * 50.0, y), Vec2::new(44.0, 24.0));
            let resp = ui.interact(r, ui.scope_id().with(("lang", i)), Sense::CLICK);
            let current = lang == self.lang;
            let p = ui.painter();
            let color = if current || (clickable && resp.hovered()) { P.text } else { P.dim };
            let border = if current { P.text } else { P.line };
            p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, fade(border)), StrokeKind::Inside);
            p.text(r.center(), Align2::CENTER_CENTER, lang.label(), FontId::proportional(13.0), fade(color));
            if clickable && resp.clicked() {
                new_lang = Some(lang);
            }
        }

        if let Some(m) = new_mix {
            self.set_mix(m);
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
        let t = &self.test;
        let frac = t.typed.len() as f32 / t.target.len().max(1) as f32;
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, 24.0), Sense::hover());
        let p = ui.painter();
        let value = if t.endless {
            format!("{} СЛОВ", t.typed.iter().filter(|&&c| c == ' ').count())
        } else {
            format!("{:.0}%", frac * 100.0)
        };
        let lang = p.layout_no_wrap(self.lang.label().to_string(), FontId::monospace(12.0), P.text);
        let lang_w = lang.size().x;
        p.galley(Pos2::new(r.right() - lang_w, r.center().y - lang.size().y / 2.0), lang, P.text);
        let val = p.layout_no_wrap(value, FontId::monospace(16.0), P.text);
        let val_x = r.right() - lang_w - 16.0 - val.size().x;
        p.galley(Pos2::new(val_x, r.center().y - val.size().y / 2.0), val, P.text);
        let label = if t.endless { "ENDLESS" } else { "PROGRESS" };
        section(p, r.left(), val_x - 10.0, r.center().y - 7.0, label, P.dim, P.line);

        const SEGMENTS: usize = 40;
        const SEG_GAP: f32 = 3.0;
        let (bar, _) = ui.allocate_exact_size(Vec2::new(w, 10.0), Sense::hover());
        if t.endless {
            return;
        }
        let seg_w = (w - SEG_GAP * (SEGMENTS - 1) as f32) / SEGMENTS as f32;
        let filled = (frac * SEGMENTS as f32).round() as usize;
        let p = ui.painter();
        for i in 0..SEGMENTS {
            let x = bar.left() + i as f32 * (seg_w + SEG_GAP);
            let seg = Rect::from_min_size(Pos2::new(x, bar.top()), Vec2::new(seg_w, bar.height()));
            let color = if i < filled { P.text } else { P.faint };
            p.rect_filled(seg, CornerRadius::ZERO, color);
        }
    }

    fn text(&self, ui: &mut egui::Ui, w: f32) {
        let t = &self.test;
        let font = FontId::monospace(20.0);
        let line_h = 31.0;
        let cw = ui.painter().layout_no_wrap("M".into(), font.clone(), P.text).size().x;
        let per_line = ((w / cw).floor() as usize).max(10);
        let mut lines = wrap(&t.target, per_line);
        if t.endless {
            let cur = lines.iter().position(|&(_, e)| t.typed.len() < e).unwrap_or(0);
            let first = cur.saturating_sub(1);
            lines = lines.into_iter().skip(first).take(ENDLESS_LINES).collect();
        }
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, lines.len() as f32 * line_h), Sense::hover());
        let p = ui.painter();
        for (n, &(s, e)) in lines.iter().enumerate() {
            let last = n + 1 == lines.len() && !t.endless;
            let content_end = if !last && t.target[e - 1] == ' ' { e - 1 } else { e };
            let spaces = t.target[s..content_end].iter().filter(|&&c| c == ' ').count();
            let extra = if last || spaces == 0 { 0.0 } else { (w - (content_end - s) as f32 * cw).max(0.0) / spaces as f32 };
            let y = r.top() + n as f32 * line_h;
            let mut x = r.left();
            for i in s..e {
                let c = t.target[i];
                let cell_w = if c == ' ' && i < content_end { cw + extra } else { cw };
                let cell = Rect::from_min_size(Pos2::new(x, y), Vec2::new(cell_w, line_h));
                let mut color = P.dim;
                let mut shown = c;
                if i < t.typed.len() {
                    if t.typed[i] == c {
                        color = P.text;
                    } else {
                        color = P.error;
                        if c == ' ' {
                            shown = '·';
                        }
                        let uy = cell.bottom() - 4.0;
                        p.hline(cell.left()..=cell.right(), uy, Stroke::new(2.0, P.error));
                    }
                } else if i == t.typed.len() {
                    color = P.bg;
                    let cursor = Rect::from_min_size(Pos2::new(x, y + 2.0), Vec2::new(cw, line_h - 4.0));
                    p.rect_filled(cursor, CornerRadius::ZERO, P.accent);
                }
                if shown != ' ' {
                    p.text(Pos2::new(x, cell.center().y), Align2::LEFT_CENTER, shown, font.clone(), color);
                }
                x += cell_w;
            }
        }
    }

    fn stats(&self, ui: &mut egui::Ui) {
        let t = &self.test;
        let secs = t.elapsed().as_secs();
        let items = [
            (format!("{:.0}", t.cpm()), "ЗН/МИН"),
            (format!("{:.0}", t.cpm() / 5.0), "СЛ/МИН"),
            (format!("{:.0}%", t.accuracy()), "ТОЧН"),
            (format!("{}:{:02}", secs / 60, secs % 60), "ВРЕМЯ"),
        ];
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 68.0), Sense::hover());
        let p = ui.painter();
        section(p, r.left(), r.right(), r.top(), "STATS", P.dim, P.line);
        let cell = r.width() / items.len() as f32;
        for (i, (value, label)) in items.into_iter().enumerate() {
            let x = r.left() + cell * i as f32;
            p.text(Pos2::new(x, r.top() + 22.0), Align2::LEFT_TOP, value, FontId::monospace(24.0), P.text);
            p.text(Pos2::new(x, r.top() + 54.0), Align2::LEFT_TOP, label, FontId::monospace(10.5), P.dim);
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
            (format!("Готово! Enter — ещё раз{tail}"), P.text)
        } else if t.started.is_none() {
            ("Начните печатать — таймер запустится с первой клавиши".to_string(), P.dim)
        } else if t.endless && t.paused() {
            ("Пауза — продолжайте печатать, ↑ — начать заново".to_string(), P.dim)
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
        job.append(f.name(), 0.0, TextFormat { font_id: font, color: P.text, ..Default::default() });
        let galley = ui.painter().layout_job(job);
        let sw = 16.0;
        let total = sw + 10.0 + galley.size().x;
        let x = r.center().x - total / 2.0;
        let swatch = Rect::from_min_size(Pos2::new(x, r.center().y - sw / 2.0), Vec2::splat(sw));
        let p = ui.painter();
        hatch(p, swatch, f.group(), P.sub);
        p.rect_stroke(swatch, CornerRadius::ZERO, Stroke::new(1.0, P.line), StrokeKind::Inside);
        p.galley(Pos2::new(x + sw + 10.0, r.center().y - galley.size().y / 2.0), galley, P.text);
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
        let new = self.lang.lesson(self.lesson).new;
        let radius = CornerRadius::ZERO;

        for (r, keys) in self.lang.rows().iter().enumerate() {
            for (col, ch) in keys.chars().enumerate() {
                let min = Pos2::new(x0 + (offsets[r] + col as f32) * unit, rect.top() + r as f32 * unit);
                let kr = Rect::from_min_size(min, Vec2::splat(KEY));
                let group = Finger::for_column(col).group();
                let in_lesson = allowed.as_ref().map_or(true, |a| a.contains(&ch));
                let is_hot = hot == Some((r, col));
                let label = if is_hot {
                    painter.rect_filled(kr, radius, P.accent);
                    P.bg
                } else if in_lesson {
                    painter.rect_filled(kr, radius, P.bg);
                    hatch(painter, kr.shrink(1.0), group, P.hatch);
                    let border = if new.contains(ch) { P.text } else { P.line };
                    painter.rect_stroke(kr, radius, Stroke::new(1.0, border), StrokeKind::Inside);
                    P.text
                } else {
                    painter.rect_stroke(kr, radius, Stroke::new(1.0, P.surface.gamma_multiply(1.6)), StrokeKind::Inside);
                    P.faint
                };
                if in_lesson && !is_hot {
                    let g = painter.layout_no_wrap(ch.to_string(), FontId::proportional(16.0), label);
                    let bg = Rect::from_center_size(kr.center(), g.size() + Vec2::new(4.0, 0.0));
                    painter.rect_filled(bg, CornerRadius::ZERO, P.bg);
                }
                painter.text(kr.center(), Align2::CENTER_CENTER, ch, FontId::proportional(16.0), label);
                if r == 1 && (col == 3 || col == 6) {
                    let y = kr.bottom() - 7.0;
                    painter.hline((kr.center().x - 4.0)..=(kr.center().x + 4.0), y, Stroke::new(2.0, label));
                }
            }
        }

        let sr = Rect::from_min_size(Pos2::new(x0 + 3.5 * unit, rect.top() + 3.0 * unit), Vec2::new(6.0 * unit - GAP, KEY));
        let label = if next == Some(' ') {
            painter.rect_filled(sr, radius, P.accent);
            P.bg
        } else {
            hatch(painter, sr.shrink(1.0), Finger::Thumb.group(), P.hatch);
            painter.rect_stroke(sr, radius, Stroke::new(1.0, P.line), StrokeKind::Inside);
            let g = painter.layout_no_wrap("пробел".to_string(), FontId::proportional(14.0), P.text);
            painter.rect_filled(Rect::from_center_size(sr.center(), g.size() + Vec2::new(8.0, 0.0)), CornerRadius::ZERO, P.bg);
            P.text
        };
        painter.text(sr.center(), Align2::CENTER_CENTER, "пробел", FontId::proportional(14.0), label);
    }

    fn footer(&self, ui: &egui::Ui, full: Rect) {
        let bar = Rect::from_min_max(Pos2::new(full.left(), full.bottom() - FOOTER), full.max);
        let p = ui.painter();
        p.hline(bar.x_range(), bar.top(), Stroke::new(1.0, P.line));

        let left = Vec2::new(-1.0, 0.0);
        let right = Vec2::new(1.0, 0.0);
        let up = Vec2::new(0.0, -1.0);
        let down = Vec2::new(0.0, 1.0);
        let items: Vec<(Vec<Chip>, &str)> = if self.screen == Screen::Start {
            vec![
                (vec![Chip::Arrow(left), Chip::Arrow(right)], "режим"),
                (vec![Chip::Arrow(up), Chip::Arrow(down)], "выбор"),
                (vec![Chip::Text("Enter")], "начать"),
                (vec![Chip::Text("Tab")], "раскладка"),
                (vec![Chip::Text("Esc")], "выход"),
            ]
        } else {
            let fourth = if is_endless(self.lesson) {
                (vec![Chip::Arrow(down)], "слова / всё")
            } else {
                (vec![Chip::Text("Enter")], "ещё раз")
            };
            vec![
                (vec![Chip::Arrow(left), Chip::Arrow(right)], "урок"),
                (vec![Chip::Arrow(up)], "другие слова"),
                (vec![Chip::Text("Tab")], "раскладка"),
                fourth,
                (vec![Chip::Text("Esc")], "меню"),
            ]
        };
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
                p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, P.line), StrokeKind::Inside);
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

const BASE_W: f32 = 980.0;
const BASE_H: f32 = 580.0;
const COLUMN_MAX: f32 = 860.0;

fn fit_zoom(ctx: &egui::Context) {
    let zoom = ctx.zoom_factor();
    let size = ctx.content_rect().size() * zoom;
    let target = (size.x / BASE_W).min(size.y / BASE_H).clamp(0.75, 3.0);
    if (target - zoom).abs() > 0.01 {
        ctx.set_zoom_factor(target);
        ctx.request_repaint();
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        fit_zoom(&ctx);
        self.textures(&ctx);
        if self.screen == Screen::Boot && self.boot.elapsed().as_secs_f32() >= BOOT_LEN {
            self.go(Screen::Start);
        }
        self.handle_input(&ctx);
        let since = self.shown.elapsed().as_secs_f32() / FADE;
        if since < 1.0 || self.screen == Screen::Boot {
            ctx.request_repaint();
        }
        if self.screen != Screen::Practice {
            let frame = egui::Frame::NONE.fill(P.bg);
            egui::CentralPanel::default().frame(frame).show(ui, |ui| {
                let full = ui.max_rect();
                if self.screen == Screen::Boot {
                    self.splash(ui, full);
                } else {
                    self.footer(ui, full);
                    self.start(ui, full);
                    if since < 1.0 {
                        ui.painter().rect_filled(full, CornerRadius::ZERO, P.bg.gamma_multiply(1.0 - since));
                    }
                }
            });
            return;
        }
        if self.typing() {
            ctx.request_repaint_after(Duration::from_millis(200));
        }

        let frame = egui::Frame::NONE.fill(P.bg);
        egui::CentralPanel::default().frame(frame).show(ui, |ui| {
            let full = ui.max_rect();
            self.logo(ui, full);
            self.footer(ui, full);

            let area = Rect::from_min_max(
                Pos2::new(full.left(), full.top() + TOP_BAR),
                Pos2::new(full.right(), full.bottom() - FOOTER),
            );
            let side = 2.0 * (MENU_W + 2.0 * MARGIN);
            let w = (area.width() - side).clamp(COLUMN, COLUMN_MAX).min(area.width() - 2.0 * MARGIN);
            let top = area.top() + ((area.height() - self.content_h) / 2.0).max(0.0);
            let col = Rect::from_min_size(Pos2::new(area.center().x - w / 2.0, top), Vec2::new(w, area.bottom() - top));
            let resp = ui.scope_builder(egui::UiBuilder::new().max_rect(col), |ui| {
                ui.set_width(w);
                ui.spacing_mut().item_spacing.y = 6.0;
                self.lesson_line(ui, w);
                ui.add_space(12.0);
                self.text(ui, w);
                ui.add_space(12.0);
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
            if since < 1.0 {
                ui.painter().rect_filled(full, CornerRadius::ZERO, P.bg.gamma_multiply(1.0 - since));
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
            .with_inner_size([980.0, 640.0])
            .with_min_inner_size([760.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "FastWord",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "IBMPlexMono".into(),
                std::sync::Arc::new(egui::FontData::from_static(include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf"))),
            );
            for family in [egui::FontFamily::Monospace, egui::FontFamily::Proportional] {
                fonts.families.entry(family).or_default().insert(0, "IBMPlexMono".into());
            }
            cc.egui_ctx.set_fonts(fonts);
            Ok(Box::new(App::new(lang, lesson, load_mix())))
        }),
    )
}
