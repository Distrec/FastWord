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

const NUMBER_ROW: &str = "1234567890-=";

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Left,
    Right,
}

#[derive(Clone, Copy)]
struct KeySpot {
    row: usize,
    col: usize,
    base: char,
    shift: Option<Side>,
}

fn key_spot(lang: Lang, c: char) -> Option<KeySpot> {
    let shifted = match (lang, c) {
        (_, '!') => Some('1'),
        (Lang::En, '?') => Some('/'),
        (Lang::Ru, ',') => Some('.'),
        (Lang::En, ':') => Some(';'),
        (Lang::En, '<') => Some(','),
        (Lang::En, '>') => Some('.'),
        (Lang::En, '"') => Some('\''),
        _ if c.is_uppercase() => c.to_lowercase().next(),
        _ => None,
    };
    let base = shifted.unwrap_or(c);
    let (row, col) = match NUMBER_ROW.chars().position(|k| k == base) {
        Some(col) => (0, col),
        None => locate(lang, base).map(|(r, col)| (r + 1, col))?,
    };
    let left_hand = col < 5;
    let shift = shifted.map(|_| if left_hand { Side::Right } else { Side::Left });
    Some(KeySpot { row, col, base, shift })
}

fn shift_label(lang: Lang, ch: char) -> char {
    let (plain, upper) = match lang {
        Lang::En => ("1234567890-=[];',./", "!@#$%^&*()_+{}:\"<>?"),
        Lang::Ru => ("1234567890-=.", "!\"№;%:?*()_+,"),
    };
    match plain.chars().position(|c| c == ch) {
        Some(i) => upper.chars().nth(i).unwrap_or(ch),
        None => ch.to_uppercase().next().unwrap_or(ch),
    }
}

const TEXT_FAMILY: &str = "text";
const BUILTIN_FONT: &str = "IBM Plex Mono";
const FONT_MIN: f32 = 14.0;
const FONT_MAX: f32 = 32.0;
const SETTINGS_ROWS: usize = 5;

fn fonts_dir() -> Option<std::path::PathBuf> {
    Some(state_path()?.with_file_name("fonts"))
}

fn scan_fonts() -> Vec<(String, std::path::PathBuf)> {
    let mut out: Vec<(String, std::path::PathBuf)> = fonts_dir()
        .and_then(|d| std::fs::read_dir(d).ok())
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()).is_some_and(|e| matches!(e.to_lowercase().as_str(), "ttf" | "otf")))
        .filter_map(|p| Some((p.file_stem()?.to_string_lossy().to_string(), p)))
        .collect();
    out.sort();
    out
}

#[derive(Debug, PartialEq)]
struct Settings {
    size: f32,
    font: String,
    show_keyboard: bool,
    colored_keyboard: bool,
}

impl Settings {
    fn parse(text: &str) -> Self {
        let mut settings = Self {
            size: 20.0,
            font: BUILTIN_FONT.to_string(),
            show_keyboard: true,
            colored_keyboard: false,
        };
        for line in text.lines() {
            if let Some(v) = line.strip_prefix("size ") {
                if let Ok(size) = v.trim().parse::<f32>() {
                    if size.is_finite() {
                        settings.size = size.clamp(FONT_MIN, FONT_MAX);
                    }
                }
            } else if let Some(v) = line.strip_prefix("font ") {
                settings.font = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("keyboard ") {
                settings.show_keyboard = v.trim() != "off";
            } else if let Some(v) = line.strip_prefix("keyboard_colors ") {
                settings.colored_keyboard = v.trim() == "on";
            }
        }
        settings
    }

    fn encode(&self) -> String {
        format!("size {:.0}\nfont {}\nkeyboard {}\nkeyboard_colors {}\n", self.size, self.font,
            if self.show_keyboard { "on" } else { "off" },
            if self.colored_keyboard { "on" } else { "off" })
    }
}

fn load_settings() -> Settings {
    let text = state_path().and_then(|p| std::fs::read_to_string(p.with_file_name("settings")).ok()).unwrap_or_default();
    Settings::parse(&text)
}

fn save_settings(settings: &Settings) {
    if let Some(p) = state_path().map(|p| p.with_file_name("settings")) {
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(p, settings.encode());
    }
}

fn finger_color(group: usize) -> Color32 {
    [Color32::from_rgb(198, 120, 221), Color32::from_rgb(97, 175, 239),
     Color32::from_rgb(152, 195, 121), P.warn, Color32::from_rgb(86, 182, 194)][group]
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
    kind: Kind,
    train_endless: bool,
    results: Vec<Option<Best>>,
    saved: bool,
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
    font_size: f32,
    font_name: String,
    show_keyboard: bool,
    colored_keyboard: bool,
    fonts: Vec<(String, std::path::PathBuf)>,
    fonts_dirty: bool,
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
    fn new(lang: Lang, kind: Kind) -> Self {
        let settings = load_settings();
        let mut rng = Rng::new();
        let test = Test::for_kind(lang, kind, &mut rng);
        App {
            lang,
            kind,
            train_endless: kind.endless(),
            results: load_results(lang),
            saved: false,
            rng,
            test,
            content_h: 0.0,
            screen: Screen::Boot,
            boot: Instant::now(),
            shown: Instant::now(),
            tab: 0,
            pick: 0,
            grain: None,
            font_size: settings.size,
            font_name: settings.font,
            show_keyboard: settings.show_keyboard,
            colored_keyboard: settings.colored_keyboard,
            fonts: scan_fonts(),
            fonts_dirty: true,
            dots: None,
        }
    }

    fn go(&mut self, screen: Screen) {
        self.screen = screen;
        self.shown = Instant::now();
        if screen == Screen::Start {
            (self.tab, self.pick) = match self.kind {
                Kind::Lesson(i) => (0, i),
                Kind::Phrases(_) => (1, 0),
                Kind::Mixed(_) => (1, 1),
            };
        }
    }

    fn picks(&self) -> usize {
        match self.tab {
            0 => PAIR_COUNT,
            1 => 2,
            2 => SETTINGS_ROWS,
            _ => 0,
        }
    }

    fn launch(&mut self) {
        self.kind = match (self.tab, self.pick) {
            (0, i) => Kind::Lesson(i.min(PAIR_COUNT - 1)),
            (1, 0) => Kind::Phrases(self.train_endless),
            (1, 1) => Kind::Mixed(self.train_endless),
            (2, i) => {
                self.setting(i, 1);
                return;
            }
            _ => return,
        };
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
        let box_h = 420.0_f32.min(area.height() - 24.0);
        let frame = Rect::from_center_size(area.center(), Vec2::new(box_w, box_h));
        let btn_w = 260.0;
        let gap = 16.0;
        let btn_h = (box_h - 2.0 * gap) / 3.0;
        let modes = [
            ("01 / MODE", "УРОКИ", format!("{} уроков · пары клавиш", PAIR_COUNT)),
            ("02 / MODE", "ТРЕНИРОВКИ", "фразы · всё подряд".to_string()),
            ("03 / MODE", "НАСТРОЙКИ", "шрифт · клавиатура".to_string()),
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

        let label = ["LESSONS", "TRAINING", "SETTINGS"][self.tab];
        section(&p, list.left(), list.right(), list.top(), label, P.dim, P.line);
        let mut clicked = None;
        let mut y = list.top() + 30.0;
        let mut size_row: Option<Rect> = None;
        let rows: Vec<(String, String)> = if self.tab == 0 {
            self.lang
                .pairs()
                .iter()
                .zip(&self.results)
                .map(|(l, b)| (l.title.to_string(), b.map_or(String::new(), |b| format!("{} зн/мин · {}%", b.cpm, b.acc))))
                .collect()
        } else if self.tab == 1 {
            let mixed = "слова и сочетания букв";
            let phrases = match (self.train_endless, self.lang) {
                (true, _) => "фразы и слова вперемешку",
                (false, Lang::Ru) => "фразы на русском",
                (false, Lang::En) => "фразы на английском",
            };
            let w = (list.width().min(360.0) - 8.0) / 2.0;
            for (k, label) in ["КОНЕЧНЫЙ", "БЕСКОНЕЧНЫЙ ∞"].into_iter().enumerate() {
                let r = Rect::from_min_size(Pos2::new(list.left() + k as f32 * (w + 8.0), y - 4.0), Vec2::new(w, 28.0));
                let resp = ui.interact(r, ui.scope_id().with(("endless", k)), Sense::CLICK);
                let on = self.train_endless == (k == 1);
                let color = if on || resp.hovered() { P.text } else { P.dim };
                p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, if on { P.text } else { P.line }), StrokeKind::Inside);
                p.text(r.center(), Align2::CENTER_CENTER, label, FontId::monospace(12.0), color);
                if resp.clicked() {
                    self.train_endless = k == 1;
                }
            }
            y += 40.0;
            vec![
                ("Фразы".to_string(), phrases.to_string()),
                ("Всё подряд".to_string(), mixed.to_string()),
            ]
        } else {
            p.text(Pos2::new(list.left(), y - 6.0), Align2::LEFT_TOP, "Шрифт и клавиатура", FontId::proportional(15.0), P.text);
            y += 26.0;
            let current = self.fonts.iter().find(|(n, _)| *n == self.font_name).map_or(BUILTIN_FONT.to_string(), |(n, _)| n.clone());
            size_row = Some(Rect::from_min_size(Pos2::new(list.left(), y), Vec2::new(list.width(), 36.0)));
            let preview_y = y + 36.0 * SETTINGS_ROWS as f32 + 18.0;
            p.text(Pos2::new(list.left(), preview_y), Align2::LEFT_TOP, "ПРИМЕР", FontId::monospace(10.5), P.dim);
            let sample = if self.lang == Lang::Ru { "Съешь же ещё этих мягких булок." } else { "The quick brown fox jumps." };
            p.text(Pos2::new(list.left(), preview_y + 20.0), Align2::LEFT_TOP, sample, self.text_font(), P.text);
            let dir = fonts_dir().map_or(String::new(), |d| d.display().to_string());
            p.text(Pos2::new(list.left(), preview_y + 30.0 + self.font_size * 1.4), Align2::LEFT_TOP, format!("Свои шрифты (.ttf, .otf): {dir}"), FontId::monospace(10.5), P.faint);
            vec![
                ("Размер текста".to_string(), String::new()),
                ("Шрифт".to_string(), current),
                ("Добавить шрифт".to_string(), "открыть папку".to_string()),
                ("Клавиатура с подсказками".to_string(), if self.show_keyboard { "вкл" } else { "выкл" }.to_string()),
                ("Цветная клавиатура".to_string(), if self.colored_keyboard { "вкл" } else { "выкл" }.to_string()),
            ]
        };
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
            let font = if self.tab == 0 { FontId::monospace(14.0) } else { FontId::proportional(14.0) };
            p.text(row.left_center() + Vec2::new(12.0, 0.0), Align2::LEFT_CENTER, format!("{:02}", i + 1), FontId::monospace(12.0), if on { P.text } else { P.faint });
            p.text(row.left_center() + Vec2::new(44.0, 0.0), Align2::LEFT_CENTER, title, font, if on { P.text } else { P.dim });
            if self.tab == 2 && i == 0 {
                p.text(row.right_center() - Vec2::new(59.0, 0.0), Align2::CENTER_CENTER, format!("{:.0} px", self.font_size), FontId::monospace(12.0), if on { P.text } else { P.dim });
            } else {
                p.text(row.right_center(), Align2::RIGHT_CENTER, extra, FontId::monospace(11.5), if on { P.sub } else { P.faint });
            }
            p.hline(row.x_range(), row.bottom(), Stroke::new(1.0, P.surface.gamma_multiply(1.4)));
            if resp.clicked() {
                clicked = Some(i);
            }
            y += 36.0;
        }
        if let Some(size_row) = size_row {
            if clicked == Some(0) {
                clicked = None;
            }
            for (k, sign) in ["−", "+"].into_iter().enumerate() {
                let r = Rect::from_min_size(Pos2::new(size_row.right() - 26.0 - (1 - k) as f32 * 92.0, size_row.center().y - 12.0), Vec2::new(26.0, 24.0));
                let resp = ui.interact(r, ui.scope_id().with(("size", k)), Sense::CLICK);
                p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, if resp.hovered() { P.text } else { P.line }), StrokeKind::Inside);
                p.text(r.center(), Align2::CENTER_CENTER, sign, FontId::monospace(14.0), P.text);
                if resp.clicked() {
                    self.pick = 0;
                    self.setting(0, if k == 0 { -1 } else { 1 });
                }
            }
        }
        if self.picks() > 0 {
            let hint = match self.tab {
                1 => "Enter — начать · Пробел — конечный / ∞",
                2 => "Enter — изменить · − / + — размер",
                _ => "Enter — начать",
            };
            p.text(Pos2::new(list.left(), frame.bottom()), Align2::LEFT_BOTTOM, hint, FontId::monospace(11.0), P.faint);
        }

        if let Some(i) = new_tab {
            if i != self.tab {
                self.tab = i;
                self.pick = 0;
                if i == 2 {
                    self.fonts = scan_fonts();
                }
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

    fn text_font(&self) -> FontId {
        FontId::new(self.font_size, egui::FontFamily::Name(TEXT_FAMILY.into()))
    }

    fn setting(&mut self, row: usize, dir: i32) {
        match row {
            0 => {
                let next = self.font_size + 2.0 * dir as f32;
                self.font_size = if next > FONT_MAX { FONT_MIN } else if next < FONT_MIN { FONT_MAX } else { next };
            }
            1 => {
                self.fonts = scan_fonts();
                let names: Vec<String> = std::iter::once(BUILTIN_FONT.to_string()).chain(self.fonts.iter().map(|(n, _)| n.clone())).collect();
                let i = names.iter().position(|n| *n == self.font_name).unwrap_or(0);
                self.font_name = names[(i + 1) % names.len()].clone();
                self.fonts_dirty = true;
            }
            3 => self.show_keyboard = !self.show_keyboard,
            4 => self.colored_keyboard = !self.colored_keyboard,
            2 => {
                if let Some(dir) = fonts_dir() {
                    let _ = std::fs::create_dir_all(&dir);
                    let opener = if cfg!(windows) { "explorer" } else if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
                    let _ = std::process::Command::new(opener).arg(&dir).spawn();
                }
                self.fonts = scan_fonts();
            }
            _ => return,
        }
        self.content_h = 0.0;
        save_settings(&Settings {
            size: self.font_size,
            font: self.font_name.clone(),
            show_keyboard: self.show_keyboard,
            colored_keyboard: self.colored_keyboard,
        });
    }

    fn apply_fonts(&mut self, ctx: &egui::Context) {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "IBMPlexMono".into(),
            std::sync::Arc::new(egui::FontData::from_static(include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf"))),
        );
        for family in [egui::FontFamily::Monospace, egui::FontFamily::Proportional] {
            fonts.families.entry(family).or_default().insert(0, "IBMPlexMono".into());
        }
        let mut text = vec!["IBMPlexMono".to_string()];
        if let Some((name, path)) = self.fonts.iter().find(|(n, _)| *n == self.font_name) {
            if let Ok(bytes) = std::fs::read(path) {
                fonts.font_data.insert(name.clone(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
                text.insert(0, name.clone());
            }
        }
        fonts.families.insert(egui::FontFamily::Name(TEXT_FAMILY.into()), text);
        ctx.set_fonts(fonts);
        self.fonts_dirty = false;
    }

    fn new_test(&mut self) {
        self.test = Test::for_kind(self.lang, self.kind, &mut self.rng);
        self.saved = false;
    }

    fn switch(&mut self) {
        save_gui_state(self.lang, self.kind);
        self.results = load_results(self.lang);
        self.new_test();
    }

    fn record(&mut self) {
        let Kind::Lesson(lesson) = self.kind else { return };
        if self.saved || !self.test.done() {
            return;
        }
        self.saved = true;
        let t = &self.test;
        let best = Best { cpm: t.cpm().round() as u32, secs: t.elapsed().as_secs() as u32, acc: t.accuracy().round() as u32 };
        if save_result(self.lang, lesson, best) {
            self.results = load_results(self.lang);
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
                            self.tab = if key == Key::ArrowRight { (self.tab + 1) % 3 } else { (self.tab + 2) % 3 };
                            self.pick = 0;
                            if self.tab == 2 {
                                self.fonts = scan_fonts();
                            }
                        }
                        Key::ArrowUp if self.picks() > 0 => self.pick = (self.pick + self.picks() - 1) % self.picks(),
                        Key::ArrowDown if self.picks() > 0 => self.pick = (self.pick + 1) % self.picks(),
                        Key::Enter => self.launch(),
                        Key::Space if self.tab == 1 => self.train_endless = !self.train_endless,
                        Key::Space if self.tab == 2 => self.setting(self.pick, 1),
                        Key::Minus if self.tab == 2 => self.setting(0, -1),
                        Key::Plus | Key::Equals if self.tab == 2 => self.setting(0, 1),
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
                    self.record();
                }
                egui::Event::Key { key, physical_key, pressed: true, modifiers, .. } => match key {
                    Key::Escape => self.go(Screen::Start),
                    Key::Backspace => self.test.backspace(),
                    Key::ArrowUp => self.new_test(),
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
        let font = self.text_font();
        let line_h = (self.font_size * 1.55).round();
        let cw = ui.painter().layout_no_wrap("M".into(), font.clone(), P.text).size().x;
        let per_line = ((w / cw).floor() as usize).max(10);
        let all = wrap(&t.target, per_line);
        let final_end = all.last().map_or(0, |&(_, e)| e);
        let window = if self.font_size >= 26.0 { 2 } else { ENDLESS_LINES };
        let cur = all.iter().position(|&(_, e)| t.typed.len() < e).unwrap_or(all.len().saturating_sub(1));
        let first = cur.saturating_sub(1).min(all.len().saturating_sub(window));
        let lines: Vec<(usize, usize)> = all.into_iter().skip(first).take(window).collect();
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, lines.len() as f32 * line_h), Sense::hover());
        let p = ui.painter();
        for (n, &(s, e)) in lines.iter().enumerate() {
            let last = e == final_end && !t.endless;
            let content_end = if !last && t.target[e - 1] == ' ' { e - 1 } else { e };
            let spaces = t.target[s..content_end].iter().filter(|&&c| c == ' ').count();
            let short = ((content_end - s) as f32 * cw) < w * 0.75;
            let extra = if last || short || spaces == 0 { 0.0 } else { (w - (content_end - s) as f32 * cw).max(0.0) / spaces as f32 };
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
            let tail = if !matches!(self.kind, Kind::Lesson(i) if i + 1 < PAIR_COUNT) {
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
        let key = match (c, key_spot(self.lang, c)) {
            (' ', _) => "Пробел — ".to_string(),
            (_, Some(spot)) if spot.shift.is_some() => {
                let side = if spot.shift == Some(Side::Left) { "левый" } else { "правый" };
                format!("Клавиша «{c}» — {side} Shift + «{}» — ", spot.base)
            }
            _ => format!("Клавиша «{c}» — "),
        };
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
        if self.colored_keyboard {
            p.rect_filled(swatch, CornerRadius::ZERO, finger_color(f.group()));
        } else {
            hatch(p, swatch, f.group(), P.sub);
        }
        p.rect_stroke(swatch, CornerRadius::ZERO, Stroke::new(1.0, P.line), StrokeKind::Inside);
        p.galley(Pos2::new(x + sw + 10.0, r.center().y - galley.size().y / 2.0), galley, P.text);
    }

    fn key(&self, painter: &egui::Painter, kr: Rect, text: &str, group: usize, in_lesson: bool, is_hot: bool, size: f32) {
        let radius = CornerRadius::ZERO;
        let label = if is_hot {
            painter.rect_filled(kr, radius, if self.colored_keyboard { finger_color(group) } else { P.accent });
            P.bg
        } else if in_lesson {
            painter.rect_filled(kr, radius, P.bg);
            let color = if self.colored_keyboard { finger_color(group) } else { P.text };
            if !self.colored_keyboard {
                hatch(painter, kr.shrink(1.0), group, P.hatch);
            }
            painter.rect_stroke(kr, radius, Stroke::new(1.0, color), StrokeKind::Inside);
            let g = painter.layout_no_wrap(text.to_string(), FontId::proportional(size), color);
            painter.rect_filled(Rect::from_center_size(kr.center(), g.size() + Vec2::new(4.0, 0.0)), radius, P.bg);
            color
        } else {
            painter.rect_stroke(kr, radius, Stroke::new(1.0, P.surface.gamma_multiply(1.6)), StrokeKind::Inside);
            P.faint
        };
        painter.text(kr.center(), Align2::CENTER_CENTER, text, FontId::proportional(size), label);
    }

    fn keyboard(&self, ui: &mut egui::Ui) {
        let unit = KEY + GAP;
        let offsets = [0.0, 0.5, 0.85, 1.35];
        let width = 12.5 * unit - GAP;
        let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 5.0 * unit - GAP), Sense::hover());
        let x0 = rect.center().x - width / 2.0;
        let painter = ui.painter();
        let next = self.test.target.get(self.test.typed.len()).copied();
        let spot = next.and_then(|c| key_spot(self.lang, c));
        let shifted = spot.is_some_and(|s| s.shift.is_some());
        let (allowed, all_letters): (Vec<char>, bool) = match self.kind {
            Kind::Lesson(i) => (self.lang.pairs()[i].new.chars().collect(), false),
            Kind::Phrases(_) | Kind::Mixed(_) => (Vec::new(), true),
        };
        let lesson_spots: Vec<KeySpot> = allowed.iter().filter_map(|&c| key_spot(self.lang, c)).collect();
        let rows = self.lang.rows();
        let all_rows = [NUMBER_ROW, rows[0], rows[1], rows[2]];
        let at = |r: usize, x: f32, w: f32| Rect::from_min_size(Pos2::new(x0 + x * unit, rect.top() + r as f32 * unit), Vec2::new(w * unit - GAP, KEY));

        for (r, keys) in all_rows.iter().enumerate() {
            for (col, ch) in keys.chars().enumerate() {
                let kr = at(r, offsets[r] + col as f32, 1.0);
                let group = Finger::for_column(col.min(11)).group();
                let in_lesson = (all_letters && r > 0) || lesson_spots.iter().any(|s| s.row == r && s.col == col);
                let is_hot = spot.is_some_and(|s| s.row == r && s.col == col);
                let text = if shifted { shift_label(self.lang, ch) } else { ch };
                self.key(painter, kr, &text.to_string(), group, in_lesson, is_hot, 16.0);
                if r == 2 && (col == 3 || col == 6) {
                    let color = if is_hot { P.bg } else if in_lesson { P.text } else { P.faint };
                    painter.hline((kr.center().x - 4.0)..=(kr.center().x + 4.0), kr.bottom() - 7.0, Stroke::new(2.0, color));
                }
            }
        }

        for side in [Side::Left, Side::Right] {
            let kr = if side == Side::Left { at(3, 0.0, 1.35) } else { at(3, 11.35, 1.15) };
            let group = if side == Side::Left { Finger::LPinky.group() } else { Finger::RPinky.group() };
            let in_lesson = matches!(self.kind, Kind::Phrases(_)) || lesson_spots.iter().any(|s| s.shift == Some(side));
            let is_hot = spot.is_some_and(|s| s.shift == Some(side));
            self.key(painter, kr, "shift", group, in_lesson, is_hot, 12.0);
        }

        let sr = at(4, 4.0, 6.0);
        self.key(painter, sr, "пробел", Finger::Thumb.group(), true, next == Some(' '), 14.0);
    }

    fn practice(&mut self, ui: &mut egui::Ui, col: Rect) -> egui::scroll_area::ScrollAreaOutput<()> {
        let mut output = None;
        ui.scope_builder(egui::UiBuilder::new().max_rect(col), |ui| {
            ui.set_clip_rect(ui.clip_rect().intersect(col));
            output = Some(egui::ScrollArea::vertical()
                .id_salt("practice")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let w = ui.available_width();
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.add_space(((col.height() - self.content_h) / 2.0).max(0.0));
                    let top = ui.cursor().top();
                    self.lesson_line(ui, w);
                    ui.add_space(12.0);
                    self.text(ui, w);
                    ui.add_space(12.0);
                    self.stats(ui);
                    self.message(ui);
                    if self.show_keyboard {
                        ui.add_space(4.0);
                        self.finger_hint(ui);
                        ui.add_space(6.0);
                        self.keyboard(ui);
                    }
                    let h = ui.min_rect().bottom() - top;
                    if (h - self.content_h).abs() > 1.0 {
                        self.content_h = h;
                        ui.ctx().request_repaint();
                    }
                }));
        });
        output.unwrap()
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
                (vec![Chip::Text("Enter")], if self.tab == 2 { "изменить" } else { "начать" }),
                (vec![Chip::Text("Tab")], "раскладка"),
                (vec![Chip::Text("Esc")], "выход"),
            ]
        } else if self.kind.endless() {
            vec![(vec![Chip::Arrow(up)], "заново"), (vec![Chip::Text("Esc")], "меню")]
        } else {
            vec![
                (vec![Chip::Arrow(up)], "другие слова"),
                (vec![Chip::Text("Enter")], "ещё раз"),
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
const BASE_H: f32 = 720.0;
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
        if self.fonts_dirty {
            self.apply_fonts(&ctx);
            ctx.request_repaint();
            return;
        }
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
            let col = Rect::from_min_max(
                Pos2::new(area.center().x - w / 2.0, area.top() + 8.0),
                Pos2::new(area.center().x + w / 2.0, area.bottom() - 8.0),
            );
            self.practice(ui, col);

            if since < 1.0 {
                ui.painter().rect_filled(full, CornerRadius::ZERO, P.bg.gamma_multiply(1.0 - since));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_keep_old_fonts_and_roundtrip_keyboard_options() {
        let old = Settings::parse("size 28\nfont My Font\n");
        assert_eq!(old.size, 28.0);
        assert_eq!(old.font, "My Font");
        assert!(old.show_keyboard);
        assert!(!old.colored_keyboard);
        let changed = Settings { show_keyboard: false, colored_keyboard: true, ..old };
        assert_eq!(Settings::parse(&changed.encode()), changed);
        assert_eq!(Settings::parse("size NaN\n").size, 20.0);
    }

    #[test]
    fn practice_fits_above_footer_with_keyboard_and_all_font_sizes() {
        for size in [Vec2::new(980.0, 640.0), Vec2::new(760.0, 600.0), Vec2::new(1920.0, 1136.0)] {
            for lang in [Lang::Ru, Lang::En] {
                for font_size in [14.0, 20.0, 24.0, 26.0, 32.0] {
                    let ctx = egui::Context::default();
                    let mut app = App::new(lang, Kind::Lesson(4));
                    app.font_size = font_size;
                    app.show_keyboard = true;
                    app.apply_fonts(&ctx);
                    let mut content = Vec2::ZERO;
                    let mut viewport = Rect::NOTHING;
                    for _ in 0..5 {
                        let zoom = ctx.zoom_factor();
                        let input = egui::RawInput {
                            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size / zoom)),
                            ..Default::default()
                        };
                        let output = ctx.run_ui(input, |ui| {
                            fit_zoom(&ctx);
                            egui::CentralPanel::default().frame(egui::Frame::NONE).show(ui, |ui| {
                                let full = ui.max_rect();
                                let w = COLUMN.min(full.width() - 2.0 * MARGIN);
                                let col = Rect::from_min_max(
                                    Pos2::new(full.center().x - w / 2.0, full.top() + TOP_BAR + 8.0),
                                    Pos2::new(full.center().x + w / 2.0, full.bottom() - FOOTER - 8.0),
                                );
                                let result = app.practice(ui, col);
                                content = result.content_size;
                                viewport = result.inner_rect;
                                assert!(viewport.bottom() <= full.bottom() - FOOTER);
                            });
                        });
                        output.drop_without_applying_deltas();
                    }
                    assert!(content.y <= viewport.height() + 1.0,
                        "size={size:?} font={font_size}: content={content:?} viewport={viewport:?}");
                    assert!(content.x <= viewport.width() + 1.0);
                }
            }
        }
    }

    #[test]
    fn hiding_keyboard_removes_keys_and_finger_hint_from_practice() {
        let ctx = egui::Context::default();
        let mut app = App::new(Lang::Ru, Kind::Lesson(4));
        app.apply_fonts(&ctx);
        let mut heights = Vec::new();
        for show in [true, false] {
            app.show_keyboard = show;
            app.content_h = 0.0;
            let output = ctx.run_ui(egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(980.0, 720.0))),
                ..Default::default()
            }, |ui| {
                let col = Rect::from_min_size(Pos2::new(200.0, 64.0), Vec2::new(580.0, 604.0));
                app.practice(ui, col);
            });
            heights.push(app.content_h);
            output.drop_without_applying_deltas();
        }
        assert!(heights[0] - heights[1] >= 220.0, "{heights:?}");
    }
}

fn main() -> eframe::Result {
    let (lang, kind) = load_gui_state();
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
            let mut app = App::new(lang, kind);
            app.apply_fonts(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}
