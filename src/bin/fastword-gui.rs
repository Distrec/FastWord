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
const SETTINGS_ROWS: usize = 6;

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
    stop_on_error: bool,
    text_width: f32,
    text_lines: usize,
}

impl Settings {
    fn parse(text: &str) -> Self {
        let mut settings = Self {
            size: 20.0,
            font: BUILTIN_FONT.to_string(),
            show_keyboard: true,
            colored_keyboard: false,
            stop_on_error: false,
            text_width: 1.0,
            text_lines: 0,
        };
        for line in text.lines() {
            if let Some(v) = line.strip_prefix("size ") {
                if let Ok(size) = v.trim().parse::<f32>() {
                    if size.is_finite() {
                        settings.size = size.clamp(FONT_MIN, FONT_MAX);
                    }
                }
            } else if let Some(v) = line.strip_prefix("text_width ") {
                if let Ok(width) = v.parse::<f32>() {
                    if width.is_finite() { settings.text_width = width.clamp(0.35, 1.0); }
                }
            } else if let Some(v) = line.strip_prefix("text_lines ") {
                settings.text_lines = v.parse::<usize>().unwrap_or(0).min(10);
            } else if let Some(v) = line.strip_prefix("font ") {
                settings.font = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("keyboard ") {
                settings.show_keyboard = v.trim() != "off";
            } else if let Some(v) = line.strip_prefix("keyboard_colors ") {
                settings.colored_keyboard = v.trim() == "on";
            } else if let Some(v) = line.strip_prefix("stop_on_error ") {
                settings.stop_on_error = v.trim() == "on";
            }
        }
        settings
    }

    fn encode(&self) -> String {
        format!("size {:.0}\nfont {}\nkeyboard {}\nkeyboard_colors {}\nstop_on_error {}\ntext_width {:.4}\ntext_lines {}\n", self.size, self.font,
            if self.show_keyboard { "on" } else { "off" },
            if self.colored_keyboard { "on" } else { "off" },
            if self.stop_on_error { "on" } else { "off" }, self.text_width, self.text_lines)
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

#[cfg(test)]
const COLUMN: f32 = 580.0;
const KEY: f32 = 34.0;
const GAP: f32 = 6.0;
const TOP_BAR: f32 = 56.0;
const FOOTER: f32 = 44.0;

const MARGIN: f32 = 20.0;

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Cards,
    Start,
    Practice,
    History,
}

struct App {
    lang: Lang,
    kind: Kind,
    train_endless: bool,
    choosing_language: bool,
    results: Vec<Option<Best>>,
    saved: bool,
    history: String,
    rng: Rng,
    test: Test,
    content_h: f32,
    screen: Screen,
    axis: [f32; 3],
    card_words: Vec<String>,
    eye: Vec<Vec<String>>,
    eye_texture: Option<egui::TextureHandle>,
    next_blink: f32,
    shown: Instant,
    tab: usize,
    pick: usize,
    grain: Option<egui::TextureHandle>,
    font_size: f32,
    font_name: String,
    show_keyboard: bool,
    colored_keyboard: bool,
    stop_on_error: bool,
    text_width: f32,
    text_lines: usize,
    fonts: Vec<(String, std::path::PathBuf)>,
    fonts_dirty: bool,
}

fn hash(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263) ^ seed.wrapping_mul(2246822519);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

const EYE_FRAMES: usize = 13;

const EYE_COLUMNS: usize = 61;
const EYE_ROWS: usize = 19;

fn eye_rows(frame: usize) -> Vec<String> {
    let open = frame as f32 / (EYE_FRAMES - 1) as f32;
    (0..EYE_ROWS).map(|row| {
        (0..EYE_COLUMNS).map(|col| {
            let x = 2.0 * col as f32 / (EYE_COLUMNS - 1) as f32 - 1.0;
            let y = (row as f32 - 9.0) / 8.0;
            let arch = (1.0 - x * x).max(0.0);
            let seam = 0.26 * arch - 0.07 * x;
            let upper = seam - 1.13 * arch * open;
            let lower = seam + 0.47 * arch * open;
            let grain = hash(col as u32, row as u32, 31);
            let lid = (y - upper).abs().min((y - lower).abs());
            if lid < 0.085 {
                if x.abs() > 0.76 { ':' } else if grain > 0.55 { 'c' } else { 'o' }
            } else if y < upper || y > lower {
                let crease = upper - 0.22 * arch * open;
                if open > 0.1 && (y - crease).abs() < 0.09 { '.' } else { ' ' }
            } else {
                let dx = x * 2.55;
                let dy = y + 0.17;
                let radius = dx.hypot(dy);
                if (dx + 0.20).hypot(dy + 0.31) < 0.13 { 'W' }
                else if radius < 0.32 { ' ' }
                else if radius < 0.76 {
                    let fiber = (dy.atan2(dx) * 17.0 + radius * 3.0).cos();
                    let tone = (0.55 - 0.26 * dy + 0.18 * fiber + 0.17 * grain).clamp(0.0, 1.0);
                    b"cldxOKXNW@"[(tone * 9.0) as usize] as char
                } else if radius < 0.87 { 'l' }
                else if y - upper < 0.21 * arch { if grain > 0.4 { ';' } else { ':' } }
                else if lower - y < 0.14 * arch || (x.abs() > 0.69 && grain > 0.64) { '.' }
                else { ' ' }
            }
        }).collect()
    }).collect()
}

fn blink_frame(time: f32) -> usize {
    let opening = if time < 0.0 { 1.0 } else if time < 0.12 {
        1.0 - time / 0.12
    } else {
        ((time - 0.17) / 0.19).clamp(0.0, 1.0)
    };
    (opening * (EYE_FRAMES - 1) as f32).round() as usize
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
    let mut image = egui::ColorImage::filled([640, 420], Color32::TRANSPARENT);
    let mut plot = |x: f32, y: f32, light: f32| {
        let x = x.round() as isize;
        let y = y.round() as isize;
        if (0..640).contains(&x) && (0..420).contains(&y) {
            let pixel = &mut image[(x as usize, y as usize)];
            let alpha = (light.clamp(0.0, 1.0) * 255.0) as u8;
            if alpha > pixel.a() {
                *pixel = Color32::from_white_alpha(alpha);
            }
        }
    };
    for rib in 0..19 {
        let t = (rib as f32 + 0.55) / 19.0;
        let radius = (std::f32::consts::PI * t).sin().powf(0.72);
        let axis_x = 190.0 + 275.0 * t;
        let axis_y = 314.0 - 221.0 * t;
        for side in [-1.0_f32, 1.0] {
            for step in 0..420 {
                let u = step as f32 / 419.0;
                let arc = (u * 2.58).sin();
                let bend = side * radius * arc;
                let x = axis_x + 100.0 * bend + 18.0 * u;
                let y = axis_y + 74.0 * bend - 70.0 * radius * u;
                let depth = (0.64 + 0.30 * side) * (1.0 - 0.65 * u.powi(2));
                for grain in 0..4 {
                    let seed = rib * 8 + grain;
                    let spread = 1.4 + 6.0 * u;
                    let dx = (hash(step, seed, 3) - 0.5) * spread;
                    let dy = (hash(step, seed, 8) - 0.5) * spread;
                    plot(x + dx, y + dy, depth * hash(step, seed, 15));
                }
                if step % 71 == 23 && rib % 2 == 0 {
                    for offset in -2..=2 {
                        plot(x + offset as f32, y, depth);
                        if rib % 4 == 0 {
                            plot(x, y + offset as f32, depth);
                        }
                    }
                }
            }
        }
    }
    image
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
        let (tab, pick) = Self::menu_selection(kind);
        App {
            lang,
            kind,
            train_endless: kind.endless(),
            choosing_language: false,
            results: load_results(lang),
            saved: false,
            history: String::new(),
            rng,
            test,
            content_h: 0.0,
            screen: Screen::Start,
            axis: [0.0, 1.0, 0.0],
            card_words: Vec::new(),
            eye: (0..EYE_FRAMES).map(eye_rows).collect(),
            eye_texture: None,
            next_blink: 0.0,
            shown: Instant::now(),
            tab,
            pick,
            grain: None,
            font_size: settings.size,
            font_name: settings.font,
            show_keyboard: settings.show_keyboard,
            colored_keyboard: settings.colored_keyboard,
            stop_on_error: settings.stop_on_error,
            text_width: settings.text_width,
            text_lines: settings.text_lines,
            fonts: scan_fonts(),
            fonts_dirty: true,
        }
    }

    fn go(&mut self, screen: Screen) {
        self.screen = screen;
        self.shown = Instant::now();
        if screen == Screen::Start {
            (self.tab, self.pick) = Self::menu_selection(self.kind);
        }
    }

    fn open_cards(&mut self) {
        let z = self.rng.below(20001) as f32 / 10000.0 - 1.0;
        let phi = std::f32::consts::TAU * self.rng.below(65536) as f32 / 65536.0;
        let radius = (1.0 - z*z).sqrt();
        self.axis = [radius * phi.cos(), radius * phi.sin(), z];
        let words: Vec<_> = self.lang.words().split_whitespace().collect();
        self.card_words = (0..18).map(|_| {
            (0..10).map(|_| words[self.rng.below(words.len())]).collect::<Vec<_>>().join("   ")
        }).collect();
        self.next_blink = 2.0 + self.rng.below(6001) as f32 / 1000.0;
        self.go(Screen::Cards);
    }

    fn cards(&mut self, ui: &mut egui::Ui) {
        let full = ui.max_rect();
        self.draw_grain(ui.painter(), full, 255);
        self.logo(ui, full);
        let scale = (full.width() * 0.70 / 710.0).min(full.height() * 0.60 / 420.0);
        let origin = full.center() - Vec2::new(355.0, 210.0)*scale;
        let elapsed = self.shown.elapsed().as_secs_f32();
        if elapsed >= self.next_blink + 0.36 {
            self.next_blink = elapsed + 2.0 + self.rng.below(6001) as f32 / 1000.0;
        }
        let (sn, cs) = (elapsed * 0.45).sin_cos();
        let [a,b,c] = self.axis;
        let mut enter = ui.input(|i| i.key_pressed(Key::Enter));
        ui.input(|i| {
            if i.key_pressed(Key::ArrowLeft) { self.tab = 1; }
            if i.key_pressed(Key::ArrowRight) && self.tab == 1 { self.tab = 0; }
            if i.key_pressed(Key::ArrowDown) { self.tab = 2; }
            if i.key_pressed(Key::ArrowUp) { self.tab = 0; }
            if i.key_pressed(Key::Tab) { self.tab = match self.tab { 1 => 0, 0 => 2, _ => 1 }; }
        });
        for (id, x, y, w, h, name) in [(1,0.,0.,318.,420.,"TRAINING"),(0,332.,0.,378.,300.,"LESSONS"),(2,332.,316.,378.,104.,"Settings")] {
            let r = Rect::from_min_size(origin+Vec2::new(x,y)*scale, Vec2::new(w,h)*scale);
            let hit = ui.interact(r, ui.scope_id().with(("card",id)), Sense::CLICK);
            let p = ui.painter().with_clip_rect(r);
            p.rect_filled(r, CornerRadius::ZERO, if id == 1 { Color32::from_gray(22) } else { P.surface });
            self.draw_grain(&p, r, 112);
            if id == 1 {
                for (j,row) in self.card_words.iter().enumerate() {
                    p.text(r.min + Vec2::new(-8.0,j as f32*27.0-7.0)*scale, Align2::LEFT_TOP, row, FontId::monospace(20.0*scale), Color32::from_gray(47));
                }
                let center = r.min + Vec2::new(187.,213.)*scale;
                for j in 0..14 {
                    let t = j as f32/13.;
                    let mut points = Vec::new();
                    for (x,y,z) in [(-95.+61.*t,43.+66.*t,-22.+42.*t),(-7.+69.*t,-112.+42.*t,32.-44.*t),(38.+48.*t,-6.+18.*t,-34.+46.*t),(-34.+22.*t,103.-39.*t,23.-38.*t)] {
                        let d = (a*x+b*y+c*z)*(1.-cs);
                        points.push(center+Vec2::new(x*cs+(b*z-c*y)*sn+a*d, y*cs+(c*x-a*z)*sn+b*d)*scale);
                    }
                    p.add(egui::Shape::line(points, Stroke::new(1.45*scale,P.text)));
                }
                for (j,ch) in name.chars().enumerate() {
                    p.text(r.min+Vec2::new(28.,38.+45.*j as f32)*scale, Align2::LEFT_TOP,ch,FontId::monospace(27.*scale),P.text);
                }
            } else {
                if id == 0 {
                    let frame = blink_frame(elapsed - self.next_blink);
                    const GLYPHS: &[u8] = b".:;cldoxOKXNW@";
                    let texture = self.eye_texture.get_or_insert_with(|| {
                        let mask = eframe::icon_data::from_png_bytes(include_bytes!("../../assets/eye/blurred-glyphs.png")).expect("eye glyph texture");
                        let image = egui::ColorImage::from_rgba_unmultiplied([mask.width as usize, mask.height as usize], &mask.rgba);
                        ui.ctx().load_texture("blurred-eye", image, egui::TextureOptions::LINEAR)
                    });
                    let step = (r.width() - 8.0 * scale) / EYE_COLUMNS as f32;
                    let pixel = step / 10.8;
                    let mut mesh = egui::Mesh::with_texture(texture.id());
                    for (row, text) in self.eye[frame].iter().enumerate() {
                        let y = r.center().y + (row as f32 - (EYE_ROWS - 1) as f32 * 0.5) * step * 1.46;
                        for (col, symbol) in text.bytes().enumerate() {
                            let Some(index) = GLYPHS.iter().position(|&glyph| glyph == symbol) else { continue };
                            let x = r.left() + 4.0 * scale + (col as f32 + 0.5) * step;
                            let tile = Rect::from_center_size(Pos2::new(x, y), Vec2::new(32.0, 36.0) * pixel);
                            let uv = Rect::from_min_max(Pos2::new(index as f32 / GLYPHS.len() as f32, 0.0),
                                Pos2::new((index + 1) as f32 / GLYPHS.len() as f32, 1.0));
                            mesh.add_rect_with_uv(tile, uv, Color32::from_gray(182));
                        }
                    }
                    p.add(mesh);
                    let area = r.shrink(12.0 * scale);
                    let count = name.chars().count();
                    let cell_width = area.width() / count as f32;
                    let font = FontId::monospace(cell_width * 1.45);
                    for (index, letter) in name.chars().enumerate() {
                        let glyph = p.layout_no_wrap(letter.to_string(), font.clone(), P.text);
                        let fraction = index as f32 / (count - 1) as f32;
                        let x = area.left() + (index as f32 + 0.5) * cell_width;
                        let y = area.top() + fraction * (area.height() - glyph.size().y);
                        p.galley(Pos2::new(x - glyph.size().x * 0.5, y), glyph, P.text);
                    }
                } else {
                    let words = ["font", "size", "keyboard", "layout", "color", "width", "height", "lines"];
                    let pitch = 12.0 * scale;
                    let column_width = 15.0 * scale;
                    let columns = (r.width() / column_width).ceil() as usize;
                    for column in 0..columns {
                        let word = words[column % words.len()];
                        let period = (word.len() + 1) as f32 * pitch;
                        let speed = (3.0 + (column * 3 % 7) as f32 * 0.55) * scale;
                        let offset = (elapsed * speed + column as f32 * pitch * 2.3) % period;
                        let x = r.left() + (column as f32 + 0.5) * column_width;
                        let mut top = r.top() + offset - period;
                        while top < r.bottom() {
                            for (index, letter) in word.chars().enumerate() {
                                let y = top + index as f32 * pitch;
                                if y + pitch >= r.top() && y < r.bottom() {
                                    p.text(Pos2::new(x, y), Align2::CENTER_TOP, letter, FontId::monospace(12.0 * scale), Color32::from_gray(48));
                                }
                            }
                            top += period;
                        }
                    }
                    p.text(r.min + Vec2::new(36., 24.) * scale, Align2::LEFT_TOP, name, FontId::monospace(32. * scale), P.text);
                    for (j,offset) in [10.,28.,17.].into_iter().enumerate() {
                        let top=r.min+Vec2::new(288.+18.*j as f32,30.)*scale;
                        p.vline(top.x,top.y..=top.y+42.*scale,Stroke::new(scale,P.dim));
                        p.rect_filled(Rect::from_center_size(top+Vec2::new(0.,offset)*scale, Vec2::new(9.,5.)*scale),CornerRadius::ZERO,P.sub);
                    }
                }
            }
            p.rect_stroke(r,CornerRadius::ZERO,Stroke::new(1.,if self.tab==id || hit.hovered(){P.text}else{P.line}),StrokeKind::Inside);
            if hit.clicked() { self.tab=id; enter=true; }
        }
        ui.painter().text(origin+Vec2::new(0.,434.)*scale,Align2::LEFT_TOP,"Arrow keys to select",FontId::monospace(13.*scale),P.sub);
        ui.painter().text(origin+Vec2::new(710.,434.)*scale,Align2::RIGHT_TOP,"Enter to open",FontId::monospace(13.*scale),P.sub);
        if enter {
            self.pick=0;
            self.screen=Screen::Start;
            self.shown=Instant::now();
        }
        ui.ctx().request_repaint_after(Duration::from_millis(24));
    }

    fn menu_selection(kind: Kind) -> (usize, usize) {
        match kind {
            Kind::Lesson(i) => (0, i),
            Kind::Phrases(_) => (1, 0),
            Kind::Mixed(_) => (1, 1),
            Kind::Code(_, _) => (1, 2),
        }
    }

    fn picks(&self) -> usize {
        match self.tab {
            0 => PAIR_COUNT,
            1 if self.choosing_language => CodeLanguage::ALL.len() + 1,
            1 => 4,
            2 => SETTINGS_ROWS,
            _ => 0,
        }
    }

    fn launch(&mut self) {
        self.kind = match (self.tab, self.pick) {
            (0, i) => Kind::Lesson(i.min(PAIR_COUNT - 1)),
            (1, i) if self.choosing_language => {
                self.choosing_language = false;
                let Some(&language) = CodeLanguage::ALL.get(i) else {
                    self.pick = 2;
                    return;
                };
                self.lang = Lang::En;
                Kind::Code(language, self.train_endless)
            }
            (1, 0) => Kind::Phrases(self.train_endless),
            (1, 1) => Kind::Mixed(self.train_endless),
            (1, 2) => {
                self.choosing_language = true;
                self.pick = match self.kind {
                    Kind::Code(language, _) => CodeLanguage::ALL.iter().position(|l| *l == language).unwrap_or(0),
                    _ => 0,
                };
                return;
            }
            (1, 3) => {
                self.history = state_path().and_then(|p| std::fs::read_to_string(p.with_file_name("history.tsv")).ok()).unwrap_or_default();
                self.go(Screen::History);
                return;
            }
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
    }

    fn draw_grain(&self, p: &egui::Painter, r: Rect, opacity: u8) {
        if let Some(tex) = &self.grain {
            let off = Pos2::new(hash(0, 1, 9), hash(0, 2, 9));
            let uv = Rect::from_min_size(off, r.size() / 256.0);
            p.image(tex.id(), r, uv, Color32::from_white_alpha(opacity));
        }
    }

    fn start(&mut self, ui: &mut egui::Ui, full: Rect) {
        let p = ui.painter().clone();
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
        let shift = ((area.width() - box_w) * 0.5 - MARGIN).clamp(0.0, 80.0);
        let frame = Rect::from_center_size(area.center() - Vec2::new(shift, 0.0), Vec2::new(box_w, box_h));
        let titles = ["Уроки", "Тренировки", "Настройки"];
        let numbers = ["01", "02", "03"];
        let left_w = frame.width() * 0.39;
        let left = frame.left();
        let top = area.top() - 8.0;
        let list = Rect::from_min_max(
            Pos2::new(left + left_w + 28.0, area.center().y - 150.0),
            frame.max,
        );
        let rule = Stroke::new(1.0, P.line);
        p.hline(left..=left + left_w, top, rule);
        p.text(Pos2::new(left, top + 9.0), Align2::LEFT_TOP, titles[self.tab],
            FontId::proportional((left_w * 0.162).min(48.0)), P.text);
        p.text(Pos2::new(left - 2.0, top + 62.0), Align2::LEFT_TOP, numbers[self.tab],
            FontId::monospace(88.0), P.text);
        let detail_x = left + left_w - 102.0;
        let detail_y = top + 85.0;
        let detail = match self.tab {
            0 => ("RU / EN", "8 УРОКОВ", "ПАРЫ КЛАВИШ"),
            1 => ("RU / EN", "4 РЕЖИМА", "ТЕКСТ / КОД"),
            _ => ("RU / EN", "6 ПУНКТОВ", "ШРИФТ / ВВОД"),
        };
        p.text(Pos2::new(detail_x, detail_y), Align2::LEFT_TOP, detail.0,
            FontId::monospace(10.0), P.sub);
        p.hline(detail_x..=left + left_w, detail_y + 21.0, rule);
        p.text(Pos2::new(detail_x, detail_y + 27.0), Align2::LEFT_TOP, detail.1,
            FontId::monospace(10.0), P.dim);
        p.text(Pos2::new(detail_x, detail_y + 44.0), Align2::LEFT_TOP, detail.2,
            FontId::monospace(9.0), P.sub);
        let switch_y = top + 192.0;
        p.hline(left..=left + left_w, switch_y, rule);
        let mut new_tab = None;
        let switch_width = left_w / 3.0;
        for (i, title) in titles.iter().enumerate() {
            let cell = Rect::from_min_size(Pos2::new(left + i as f32 * switch_width, switch_y),
                Vec2::new(switch_width, 35.0));
            let response = ui.interact(cell, ui.scope_id().with(("mode", i)), Sense::CLICK);
            let selected = i == self.tab;
            let ink = if selected || response.hovered() { P.text } else { P.sub };
            p.text(cell.center(), Align2::CENTER_CENTER, *title, FontId::proportional(13.0), ink);
            p.hline(cell.x_range(), cell.bottom(), if selected { Stroke::new(2.0, ink) } else { rule });
            if response.clicked() { new_tab = Some(i); }
        }
        let label = if self.choosing_language { "ЯЗЫКИ ПРОГРАММИРОВАНИЯ" } else {
            ["ПАРЫ КЛАВИШ", "РЕЖИМ ТРЕНИРОВКИ", "ПАРАМЕТРЫ"][self.tab]
        };
        section(&p, list.left(), list.right(), list.top() + 12.0, label, P.sub, P.line);
        let mut clicked = None;
        let mut y = list.top() + 44.0;
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
            if self.choosing_language {
                CodeLanguage::ALL.iter().map(|l| (l.name().to_string(), String::new()))
                    .chain(std::iter::once(("Назад".to_string(), String::new()))).collect()
            } else { vec![
                ("Фразы".to_string(), phrases.to_string()),
                ("Всё подряд".to_string(), mixed.to_string()),
                ("Языки программирования".to_string(), String::new()),
                ("Результаты".to_string(), "история занятий".to_string()),
            ] }
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
                ("Остановка на ошибке".to_string(), if self.stop_on_error { "вкл" } else { "выкл" }.to_string()),
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
                self.choosing_language = false;
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
        self.draw_grain(&p, full, 255);
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
            5 => self.stop_on_error = !self.stop_on_error,
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
        self.persist_settings();
    }

    fn persist_settings(&self) {
        save_settings(&Settings {
            size: self.font_size,
            font: self.font_name.clone(),
            show_keyboard: self.show_keyboard,
            colored_keyboard: self.colored_keyboard,
            stop_on_error: self.stop_on_error,
            text_width: self.text_width,
            text_lines: self.text_lines,
        });
    }

    fn apply_fonts(&mut self, ctx: &egui::Context) {
        let mut fonts = base_fonts();
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
        if self.saved || !self.test.done() {
            return;
        }
        self.saved = true;
        let t = &self.test;
        let best = Best { cpm: t.cpm().round() as u32, secs: t.elapsed().as_secs() as u32, acc: t.accuracy().round() as u32 };
        let mode = match self.kind {
            Kind::Lesson(i) => format!("Урок {}", i + 1),
            Kind::Phrases(_) => "Фразы".into(),
            Kind::Mixed(_) => "Всё подряд".into(),
            Kind::Code(language, _) => language.name().into(),
        };
        if let Some(path) = state_path() {
            use std::io::Write;
            if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path.with_file_name("history.tsv")) {
                let _ = writeln!(file, "{}\t{}\t{}\t{}\t{}", mode, self.lang.label(), best.cpm, best.acc, best.secs);
            }
        }
        let Kind::Lesson(lesson) = self.kind else { return };
        if save_result(self.lang, lesson, best) {
            self.results = load_results(self.lang);
        }
    }

    fn typing(&self) -> bool {
        self.test.started.is_some() && !self.test.done() && !(self.test.endless && self.test.paused())
    }

    fn handle_input(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|i| i.events.clone());
        if self.screen == Screen::Start {
            for e in events {
                if let egui::Event::Key { key, pressed: true, .. } = e {
                    match key {
                        Key::Backspace if self.choosing_language => { self.choosing_language = false; self.pick = 2; }
                        Key::Backspace => self.open_cards(),
                        Key::Tab => {
                            self.lang = self.lang.toggle();
                            self.switch();
                        }
                        Key::ArrowLeft | Key::ArrowRight => {
                            self.choosing_language = false;
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
                        self.test.type_char_with_stop(c, self.lang, self.stop_on_error);
                        self.test.top_up(self.lang, &mut self.rng);
                    }
                    self.record();
                }
                egui::Event::Key { key, physical_key, pressed: true, modifiers, .. } => match key {
                    Key::F1 => self.go(Screen::Start),
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

    fn text(&mut self, ui: &mut egui::Ui, available: f32) {
        let w = available * self.text_width;
        let t = &self.test;
        let font = self.text_font();
        let line_h = (self.font_size * 1.55).round();
        let cw = ui.painter().layout_no_wrap("M".into(), font.clone(), P.text).size().x;
        let per_line = ((w / cw).floor() as usize).max(10);
        let all = wrap(&t.target, per_line);
        let final_end = all.last().map_or(0, |&(_, e)| e);
        let window = if self.text_lines > 0 { self.text_lines } else if self.font_size >= 26.0 { 2 } else { ENDLESS_LINES };
        let cur = all.iter().position(|&(_, e)| t.typed.len() < e).unwrap_or(all.len().saturating_sub(1));
        let first = cur.saturating_sub(usize::from(window > 1)).min(all.len().saturating_sub(window));
        let lines: Vec<(usize, usize)> = all.into_iter().skip(first).take(window).collect();
        let (r, _) = ui.allocate_exact_size(Vec2::new(w, (if self.text_lines == 0 { lines.len() } else { window }) as f32 * line_h), Sense::hover());
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
                    let cursor = Rect::from_min_size(Pos2::new(x, y + 2.0), Vec2::new(cw, line_h - 4.0));
                    if t.blocked_error {
                        p.rect_stroke(cursor, CornerRadius::ZERO, Stroke::new(1.0, P.accent), StrokeKind::Inside);
                        color = P.error;
                        if c == ' ' {
                            shown = '·';
                        }
                        p.hline(cell.left()..=cell.right(), cell.bottom() - 4.0, Stroke::new(2.0, P.error));
                    } else {
                        color = P.bg;
                        p.rect_filled(cursor, CornerRadius::ZERO, P.accent);
                    }
                }
                if shown != ' ' {
                    p.text(Pos2::new(x, cell.center().y), Align2::LEFT_CENTER, shown, font.clone(), color);
                }
                x += cell_w;
            }
        }
        let (rail, _) = ui.allocate_exact_size(Vec2::new(w, 10.0), Sense::hover());
        let painter = ui.painter();
        painter.hline(rail.x_range(), rail.center().y, Stroke::new(1.0, P.line));
        for offset in [0.0, 5.0] {
            let end = rail.right_center() - Vec2::new(offset, 0.0);
            painter.line_segment([end - Vec2::new(7.0, -3.0), end + Vec2::new(0.0, -4.0)], Stroke::new(1.5, P.sub));
        }
        let grip = Rect::from_min_size(rail.right_center() - Vec2::new(24.0, 9.0), Vec2::new(26.0, 18.0));
        let drag = ui.interact(grip, ui.scope_id().with("resize_text"), Sense::click_and_drag())
            .on_hover_cursor(egui::CursorIcon::ResizeNwSe)
            .on_hover_text("Потяните угол, чтобы изменить ширину и высоту. Двойной щелчок — сброс.");
        if drag.dragged() {
            if let Some(pointer) = drag.interact_pointer_pos() {
                self.text_width = ((pointer.x - r.left()) / available).clamp(0.35, 1.0);
                self.text_lines = ((pointer.y - r.top()) / line_h).round().clamp(1.0, 10.0) as usize;
            }
        }
        if drag.double_clicked() {
            self.text_width = 1.0;
            self.text_lines = 0;
        }
        if drag.drag_stopped() || drag.double_clicked() { self.persist_settings(); }
    }

    fn stats(&self, ui: &mut egui::Ui) {
        let t = &self.test;
        let secs = t.elapsed().as_secs();
        let items = [
            (format!("{:.0}", t.cpm()), "ЗН/МИН"),
            (format!("{:.0}", t.cpm() / 5.0), "СЛ/МИН"),
            (if t.done() { format!("{:.1}%", t.accuracy()) } else { format!("{:.0}%", t.accuracy()) }, "ТОЧН"),
            (format!("{}:{:02}", secs / 60, secs % 60), "ВРЕМЯ"),
        ];
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), if t.done() { 96.0 } else { 56.0 }), Sense::hover());
        let p = ui.painter();
        section(p, r.left(), r.right(), r.top(), if t.done() { "ИТОГ ЗАНЯТИЯ" } else { "ПОКАЗАТЕЛИ" }, P.sub, P.line);
        for (i, (value, label)) in items.into_iter().enumerate() {
            let fraction = i as f32 / 3.0;
            let x = r.left() + r.width() * fraction;
            let align = match i { 0 => Align2::LEFT_BOTTOM, 3 => Align2::RIGHT_BOTTOM, _ => Align2::CENTER_BOTTOM };
            let size = if t.done() && i == 2 { 48.0 } else if t.done() { 32.0 } else { 23.0 };
            p.text(Pos2::new(x, r.bottom() - 13.0), align, value, FontId::monospace(size), P.text);
            p.text(Pos2::new(x, r.bottom()), align, if i == 2 { "ТОЧНОСТЬ" } else { label }, FontId::monospace(11.0), P.sub);
        }
        if t.done() && !t.typed.is_empty() {
            let (chart, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 48.0), Sense::hover());
            let painter = ui.painter();
            section(painter, chart.left(), chart.right(), chart.top(), "СОВПАДЕНИЯ ПО УЧАСТКАМ ТЕКСТА", P.sub, P.line);
            let chunk = t.typed.len().div_ceil(32);
            let count = t.typed.len().div_ceil(chunk);
            let width = chart.width() / count as f32;
            for (i, part) in t.typed.chunks(chunk).enumerate() {
                let start = i * chunk;
                let matches = part.iter().zip(&t.target[start..]).filter(|(a, b)| a == b).count();
                let height = 2.0 + 20.0 * matches as f32 / part.len() as f32;
                let corner = Pos2::new(chart.left() + i as f32 * width, chart.bottom() - height);
                painter.rect_filled(Rect::from_min_size(corner, Vec2::new(width - 2.0, height)), CornerRadius::ZERO, P.sub);
            }
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
        if text.is_empty() { return; }
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
            Kind::Phrases(_) | Kind::Mixed(_) | Kind::Code(_, _) => (Vec::new(), true),
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
                    ui.add_space(2.0);
                    let top = ui.cursor().top();
                    self.lesson_line(ui, w);
                    ui.add_space(3.0);
                    self.text(ui, w);
                    ui.add_space(3.0);
                    self.stats(ui);
                    self.message(ui);
                    if self.show_keyboard && !self.test.done() {
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
                (vec![Chip::Text("Backspace")], "назад"), (vec![Chip::Text("Esc")], "выход"),
            ]
        } else if self.kind.endless() {
            vec![(vec![Chip::Arrow(up)], "заново"), (vec![Chip::Text("F1")], "меню"), (vec![Chip::Text("Esc")], "выход")]
        } else {
            vec![
                (vec![Chip::Arrow(up)], "другие слова"),
                (vec![Chip::Text("Enter")], "ещё раз"),
                (vec![Chip::Text("F1")], "меню"), (vec![Chip::Text("Esc")], "выход"),
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
        if self.screen == Screen::History {
            egui::CentralPanel::default().frame(egui::Frame::NONE.inner_margin(20.0).fill(P.bg)).show(ui, |ui| {
                ui.heading("Результаты занятий");
                if ui.button("Назад").clicked() || ctx.input(|i| i.key_pressed(Key::Backspace)) {
                    self.go(Screen::Start);
                    self.tab = 1;
                    self.pick = 3;
                }
                if self.history.is_empty() {
                    ui.label("Здесь появятся результаты завершённых занятий.");
                } else {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("history").striped(true).show(ui, |ui| {
                            for title in ["Занятие", "Язык", "Зн/мин", "Точность, %", "Время, с"] { ui.strong(title); }
                            ui.end_row();
                            for row in self.history.lines().rev() {
                                for cell in row.split('\t').take(5) { ui.label(cell); }
                                ui.end_row();
                            }
                        });
                    });
                }
            });
            return;
        }
        if self.screen != Screen::Cards {
            self.handle_input(&ctx);
            if self.screen == Screen::Cards { ctx.request_repaint(); return; }
        }
        if self.screen == Screen::Cards {
            egui::CentralPanel::default().frame(egui::Frame::NONE.fill(P.bg)).show(ui, |ui| self.cards(ui));
            return;
        }
        if self.screen == Screen::Start {
            let frame = egui::Frame::NONE.fill(P.bg);
            egui::CentralPanel::default().frame(frame).show(ui, |ui| {
                let full = ui.max_rect();
                self.footer(ui, full);
                self.start(ui, full);
            });
            return;
        }
        let since = self.shown.elapsed().as_secs_f32() / 0.35;
        if since < 1.0 {
            ctx.request_repaint();
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
            let w = area.width() - 2.0 * MARGIN;
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
    fn lessons_eye_closes_and_reopens() {
        assert_eq!(blink_frame(-1.0), EYE_FRAMES - 1);
        assert_eq!(blink_frame(0.14), 0);
        assert_eq!(blink_frame(0.36), EYE_FRAMES - 1);
        assert!(blink_frame(0.24) < blink_frame(0.30));
        let visible = |frame| eye_rows(frame).iter().flat_map(|row| row.bytes()).filter(|&b| b != b' ').count();
        assert!(visible(0) > 0);
        assert!(visible(EYE_FRAMES - 1) > 3 * visible(0));
        for frame in 0..EYE_FRAMES {
            assert!(eye_rows(frame).iter().all(|row| row.is_ascii() && row.len() == EYE_COLUMNS));
        }
    }

    #[test]
    fn training_card_uses_current_dictionary_and_unit_axis() {
        let mut app = App::new(Lang::Ru, Kind::Lesson(0));
        app.rng = Rng(42);
        for lang in [Lang::Ru, Lang::En] {
            app.lang = lang;
            let previous = app.axis;
            app.open_cards();
            assert!(app.screen == Screen::Cards);
            assert_ne!(previous, app.axis);
            assert!((app.axis.iter().map(|v| v*v).sum::<f32>() - 1.0).abs() < 1e-5);
            let dictionary: std::collections::HashSet<_> = lang.words().split_whitespace().collect();
            assert!(app.card_words.iter().flat_map(|r| r.split_whitespace()).all(|w| dictionary.contains(w)));
        }
    }

    #[test]
    fn code_result_survives_reopening_history() {
        if std::env::var_os("FASTWORD_HISTORY_TEST_CHILD").is_none() {
            let dir = std::env::temp_dir().join(format!("fastword-history-{}", std::process::id()));
            let result = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "tests::code_result_survives_reopening_history"])
                .env("FASTWORD_HISTORY_TEST_CHILD", "1")
                .env("XDG_STATE_HOME", &dir)
                .env("APPDATA", &dir)
                .status().unwrap();
            std::fs::remove_dir_all(dir).unwrap();
            assert!(result.success());
            return;
        }
        let mut app = App::new(Lang::Ru, Kind::Lesson(0));
        app.tab = 1;
        app.pick = 2;
        app.launch();
        assert!(app.choosing_language);
        app.pick = 3;
        app.launch();
        assert_eq!(app.lang, Lang::En);
        for c in app.test.target.clone() {
            app.test.type_char(c, app.lang);
        }
        app.record();
        app.record();
        let mut reopened = App::new(Lang::En, Kind::Code(CodeLanguage::CSharp, false));
        reopened.pick = 3;
        reopened.launch();
        assert!(reopened.screen == Screen::History);
        assert_eq!(reopened.history.lines().count(), 1);
        let fields: Vec<_> = reopened.history.trim().split('\t').collect();
        assert_eq!(fields.len(), 5);
        assert_eq!(fields[0], "C#");
        assert_eq!(fields[3], "100");
    }


    #[test]
    fn logo_flicker_dims_briefly_and_reschedules() {
        let mut flicker = LogoFlicker::new(Rng(1));
        let first = flicker.next;
        assert!((3.5..=8.5).contains(&first));
        assert_eq!(flicker.color(first - 0.1), P.text);
        assert_ne!(flicker.color(first + 0.08), P.text);
        assert_ne!(flicker.color(first + 0.24), P.text);
        let end = first + LogoFlicker::DURATION;
        assert_eq!(flicker.color(end + 0.01), P.text);
        let delay = flicker.next - end;
        assert!((3.49..=8.51).contains(&delay));
        assert!((delay - first).abs() > 0.01);
    }

    #[test]
    fn welcome_animates_the_title_and_waits_for_enter() {
        let ctx = egui::Context::default();
        ctx.set_fonts(base_fonts());
        let dots = ctx.load_texture("dots", dots_image(), egui::TextureOptions::NEAREST);
        let run = |elapsed, events, title| {
            let mut started = false;
            let output = ctx.run_ui(egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(980.0, 640.0))),
                events,
                ..Default::default()
            }, |ui| {
                started = draw_welcome(ui, &dots, elapsed, P.text);
            });
            assert!(output.shapes.iter().any(|shape| {
                matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text() == title)
            }));
            output.drop_without_applying_deltas();
            started
        };

        assert!(!run(0.35, vec![], "F"));
        assert!(!run(10.0, vec![], "FastWord"));
        assert!(run(11.0, vec![egui::Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }], "FastWord"));
    }

    #[test]
    fn settings_keep_old_fonts_and_roundtrip_keyboard_options() {
        let old = Settings::parse("size 28\nfont My Font\n");
        assert_eq!(old.size, 28.0);
        assert_eq!(old.font, "My Font");
        assert!(old.show_keyboard);
        assert!(!old.colored_keyboard);
        assert!(!old.stop_on_error);
        let changed = Settings { show_keyboard: false, colored_keyboard: true, stop_on_error: true, ..old };
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

fn base_fonts() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "IBMPlexMono".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf"))),
    );
    fonts.font_data.insert(
        "IBMPlexMonoLight".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!("../../assets/fonts/IBMPlexMono-Light.ttf"))),
    );
    for family in [egui::FontFamily::Monospace, egui::FontFamily::Proportional] {
        fonts.families.entry(family).or_default().insert(0, "IBMPlexMonoLight".into());
    }
    fonts
}

struct LogoFlicker {
    rng: Rng,
    next: f32,
}

impl LogoFlicker {
    const DURATION: f32 = 0.32;

    fn new(mut rng: Rng) -> Self {
        let next = Self::delay(&mut rng);
        Self { rng, next }
    }

    fn delay(rng: &mut Rng) -> f32 {
        3.5 + rng.below(5001) as f32 / 1000.0
    }

    fn color(&mut self, elapsed: f32) -> Color32 {
        let end = self.next + Self::DURATION;
        if elapsed >= end {
            self.next = elapsed + Self::delay(&mut self.rng);
        }
        if elapsed < self.next {
            return P.text;
        }
        let phase = (elapsed - self.next) / Self::DURATION;
        let dimming = (phase * std::f32::consts::TAU).sin().abs();
        P.text.gamma_multiply(1.0 - dimming * 0.32)
    }
}

enum Startup {
    Welcome {
        dots: egui::TextureHandle,
        shown: Option<Instant>,
        flicker: LogoFlicker,
    },
    Ready(Box<App>),
}

fn draw_welcome(ui: &mut egui::Ui, dots: &egui::TextureHandle, elapsed: f32, logo_color: Color32) -> bool {
    let frame = egui::Frame::NONE.fill(P.bg);
    egui::CentralPanel::default().frame(frame).show(ui, |ui| {
        let full = ui.max_rect();
        let p = ui.painter();
        let x0 = full.left() + full.width() * 0.30;
        let background = Rect::from_min_max(Pos2::new(x0, full.top()), full.max);
        let drift = Vec2::new((elapsed * 0.65).sin() * 0.030, (elapsed * 0.48).sin() * 0.020);
        let uv = Rect::from_min_max(Pos2::new(0.04, 0.03) + drift, Pos2::new(0.96, 0.97) + drift);
        p.image(dots.id(), background, uv, Color32::WHITE.gamma_multiply(0.93));
        let edge = 140.0;
        for i in 0..28 {
            let k = i as f32 / 28.0;
            let band = Rect::from_min_size(
                Pos2::new(x0 + k * edge, full.top()),
                Vec2::new(edge / 28.0 + 1.0, full.height()),
            );
            p.rect_filled(band, CornerRadius::ZERO, P.bg.gamma_multiply(1.0 - k));
        }

        let content = Rect::from_center_size(
            full.center(),
            Vec2::new((full.width() - 120.0).clamp(1.0, 860.0), full.height().min(420.0)),
        );
        let font_size = (content.width() / 5.2).min(118.0);
        const TITLE: &str = "FastWord";
        let typed = (((elapsed - 0.3) / 0.16).floor() + 1.0).clamp(0.0, TITLE.len() as f32) as usize;
        let font = FontId::monospace(font_size);
        let end = p.text(content.left_center(), Align2::LEFT_CENTER, &TITLE[..typed], font, logo_color);
        if typed < TITLE.len() && (elapsed * 2.5) as u32 % 2 == 0 {
            let cursor = Rect::from_center_size(
                Pos2::new(end.right() + font_size * 0.20, content.center().y),
                Vec2::new(font_size * 0.30, font_size * 0.81),
            );
            p.rect_filled(cursor, CornerRadius::ZERO, P.text);
        }
        p.text(content.left_top(), Align2::LEFT_TOP, "ТРЕНАЖЁР СЛЕПОЙ ПЕЧАТИ", FontId::monospace(12.0), P.sub);

        p.text(content.right_bottom(), Align2::RIGHT_BOTTOM, "Enter — меню", FontId::monospace(12.0), P.dim);
        ui.input(|input| input.key_pressed(Key::Enter))
    }).inner
}

impl eframe::App for Startup {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if ui.input(|input| input.key_pressed(Key::Escape)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        match self {
            Self::Welcome { dots, shown, flicker } => {
                let elapsed = shown.get_or_insert_with(Instant::now).elapsed().as_secs_f32();
                if !draw_welcome(ui, dots, elapsed, flicker.color(elapsed)) {
                    ui.ctx().request_repaint_after(Duration::from_millis(33));
                    return;
                }
                let (lang, kind) = load_gui_state();
                let mut app = App::new(lang, kind);
                app.tab = 1;
                app.open_cards();
                app.apply_fonts(ui.ctx());
                *self = Self::Ready(Box::new(app));
                ui.ctx().request_repaint();
            }
            Self::Ready(app) => app.ui(ui, frame),
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("FastWord")
            .with_app_id("fastword-gui")
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icon/fastword.png")).expect("FastWord icon"))
            .with_inner_size([980.0, 640.0])
            .with_min_inner_size([760.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "FastWord",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            cc.egui_ctx.set_fonts(base_fonts());
            let dots = cc.egui_ctx.load_texture("dots", dots_image(), egui::TextureOptions::NEAREST);
            Ok(Box::new(Startup::Welcome {
                dots,
                shown: None,
                flicker: LogoFlicker::new(Rng::new()),
            }))
        }),
    )
}
