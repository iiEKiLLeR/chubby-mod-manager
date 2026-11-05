//! Immediate-mode widgets and theme engine drawn with macroquad shapes and the system font.
//! The window uses the normal Windows arrow cursor only: nothing here draws a cursor.

use macroquad::prelude::*;

#[allow(dead_code)]
pub const BG_TOP: Color = Color::new(0.039, 0.055, 0.090, 1.0);
#[allow(dead_code)]
pub const BG_BOT: Color = Color::new(0.067, 0.094, 0.153, 1.0);
#[allow(dead_code)]
pub const PANEL: Color = Color::new(0.075, 0.106, 0.176, 0.94);
#[allow(dead_code)]
pub const PANEL2: Color = Color::new(0.106, 0.149, 0.235, 0.96);
#[allow(dead_code)]
pub const LINE: Color = Color::new(0.58, 0.72, 0.92, 0.12);
pub const TEXT: Color = Color::new(0.945, 0.961, 0.976, 1.0);
#[allow(dead_code)]
pub const MUTED: Color = Color::new(0.545, 0.620, 0.722, 1.0);
#[allow(dead_code)]
pub const ACCENT: Color = Color::new(0.055, 0.647, 0.914, 1.0);
#[allow(dead_code)]
pub const ACCENT_HI: Color = Color::new(0.220, 0.741, 0.973, 1.0);
#[allow(dead_code)]
pub const GOOD: Color = Color::new(0.063, 0.725, 0.506, 1.0);
#[allow(dead_code)]
pub const WARN: Color = Color::new(0.961, 0.620, 0.043, 1.0);
pub const BAD: Color = Color::new(0.957, 0.247, 0.369, 1.0);
pub const APP_ICON_JPG: &[u8] = include_bytes!("../assets/icon.jpg");

#[derive(Clone, Copy)]
pub struct Theme {
    pub id: &'static str,
    pub name: &'static str,
    pub bg_top: Color,
    pub bg_bot: Color,
    pub bg_glow: Color,
    pub top_bar: Color,
    pub status_bar: Color,
    pub panel: Color,
    pub panel2: Color,
    pub field_bg: Color,
    pub btn_bg: Color,
    pub btn_hover: Color,
    pub btn_disabled: Color,
    pub line: Color,
    pub line_hi: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_hi: Color,
    pub accent_soft: Color,
    pub good: Color,
    pub warn: Color,
    pub bad: Color,
}

pub const THEMES: &[Theme] = &[
    Theme {
        id: "obsidian_cyan",
        name: "Obsidian Cyan",
        bg_top: Color::new(0.039, 0.055, 0.090, 1.0),
        bg_bot: Color::new(0.063, 0.090, 0.145, 1.0),
        bg_glow: Color::new(0.055, 0.647, 0.914, 0.045),
        top_bar: Color::new(0.031, 0.043, 0.075, 0.97),
        status_bar: Color::new(0.027, 0.039, 0.067, 0.98),
        panel: Color::new(0.071, 0.102, 0.165, 0.94),
        panel2: Color::new(0.102, 0.145, 0.227, 0.96),
        field_bg: Color::new(0.035, 0.051, 0.086, 0.98),
        btn_bg: Color::new(0.125, 0.173, 0.263, 1.0),
        btn_hover: Color::new(0.173, 0.231, 0.345, 1.0),
        btn_disabled: Color::new(0.090, 0.122, 0.184, 0.75),
        line: Color::new(0.56, 0.71, 0.94, 0.13),
        line_hi: Color::new(0.75, 0.86, 1.0, 0.20),
        text: Color::new(0.945, 0.961, 0.976, 1.0),
        muted: Color::new(0.545, 0.620, 0.722, 1.0),
        accent: Color::new(0.055, 0.647, 0.914, 1.0),
        accent_hi: Color::new(0.220, 0.741, 0.973, 1.0),
        accent_soft: Color::new(0.055, 0.647, 0.914, 0.18),
        good: Color::new(0.063, 0.725, 0.506, 1.0),
        warn: Color::new(0.961, 0.620, 0.043, 1.0),
        bad: Color::new(0.957, 0.247, 0.369, 1.0),
    },
    Theme {
        id: "amethyst_noir",
        name: "Amethyst Noir",
        bg_top: Color::new(0.047, 0.039, 0.082, 1.0),
        bg_bot: Color::new(0.082, 0.063, 0.141, 1.0),
        bg_glow: Color::new(0.545, 0.361, 0.965, 0.045),
        top_bar: Color::new(0.039, 0.031, 0.067, 0.97),
        status_bar: Color::new(0.031, 0.027, 0.055, 0.98),
        panel: Color::new(0.086, 0.075, 0.153, 0.94),
        panel2: Color::new(0.125, 0.106, 0.216, 0.96),
        field_bg: Color::new(0.043, 0.035, 0.078, 0.98),
        btn_bg: Color::new(0.149, 0.129, 0.251, 1.0),
        btn_hover: Color::new(0.204, 0.176, 0.329, 1.0),
        btn_disabled: Color::new(0.102, 0.090, 0.173, 0.75),
        line: Color::new(0.68, 0.58, 0.96, 0.13),
        line_hi: Color::new(0.82, 0.75, 1.0, 0.20),
        text: Color::new(0.953, 0.945, 0.984, 1.0),
        muted: Color::new(0.608, 0.576, 0.725, 1.0),
        accent: Color::new(0.545, 0.361, 0.965, 1.0),
        accent_hi: Color::new(0.655, 0.545, 0.980, 1.0),
        accent_soft: Color::new(0.545, 0.361, 0.965, 0.20),
        good: Color::new(0.133, 0.773, 0.369, 1.0),
        warn: Color::new(0.961, 0.620, 0.043, 1.0),
        bad: Color::new(0.957, 0.247, 0.369, 1.0),
    },
    Theme {
        id: "biohazard_emerald",
        name: "Biohazard Emerald",
        bg_top: Color::new(0.031, 0.059, 0.047, 1.0),
        bg_bot: Color::new(0.055, 0.102, 0.082, 1.0),
        bg_glow: Color::new(0.063, 0.725, 0.506, 0.045),
        top_bar: Color::new(0.024, 0.047, 0.039, 0.97),
        status_bar: Color::new(0.020, 0.039, 0.031, 0.98),
        panel: Color::new(0.063, 0.114, 0.094, 0.94),
        panel2: Color::new(0.090, 0.161, 0.133, 0.96),
        field_bg: Color::new(0.027, 0.051, 0.043, 0.98),
        btn_bg: Color::new(0.114, 0.192, 0.161, 1.0),
        btn_hover: Color::new(0.157, 0.255, 0.216, 1.0),
        btn_disabled: Color::new(0.078, 0.133, 0.110, 0.75),
        line: Color::new(0.45, 0.85, 0.68, 0.13),
        line_hi: Color::new(0.65, 0.95, 0.82, 0.20),
        text: Color::new(0.937, 0.973, 0.957, 1.0),
        muted: Color::new(0.525, 0.663, 0.604, 1.0),
        accent: Color::new(0.063, 0.725, 0.506, 1.0),
        accent_hi: Color::new(0.204, 0.827, 0.600, 1.0),
        accent_soft: Color::new(0.063, 0.725, 0.506, 0.18),
        good: Color::new(0.204, 0.827, 0.600, 1.0),
        warn: Color::new(0.961, 0.620, 0.043, 1.0),
        bad: Color::new(0.957, 0.247, 0.369, 1.0),
    },
    Theme {
        id: "tactical_amber",
        name: "Tactical Amber",
        bg_top: Color::new(0.055, 0.047, 0.039, 1.0),
        bg_bot: Color::new(0.098, 0.082, 0.067, 1.0),
        bg_glow: Color::new(0.961, 0.620, 0.043, 0.04),
        top_bar: Color::new(0.043, 0.039, 0.031, 0.97),
        status_bar: Color::new(0.035, 0.031, 0.024, 0.98),
        panel: Color::new(0.102, 0.090, 0.075, 0.94),
        panel2: Color::new(0.145, 0.129, 0.106, 0.96),
        field_bg: Color::new(0.047, 0.043, 0.035, 0.98),
        btn_bg: Color::new(0.184, 0.161, 0.133, 1.0),
        btn_hover: Color::new(0.247, 0.216, 0.180, 1.0),
        btn_disabled: Color::new(0.125, 0.110, 0.094, 0.75),
        line: Color::new(0.92, 0.75, 0.48, 0.13),
        line_hi: Color::new(0.98, 0.85, 0.62, 0.20),
        text: Color::new(0.976, 0.961, 0.937, 1.0),
        muted: Color::new(0.678, 0.627, 0.561, 1.0),
        accent: Color::new(0.851, 0.467, 0.024, 1.0),
        accent_hi: Color::new(0.961, 0.620, 0.043, 1.0),
        accent_soft: Color::new(0.961, 0.620, 0.043, 0.18),
        good: Color::new(0.133, 0.773, 0.369, 1.0),
        warn: Color::new(0.984, 0.749, 0.141, 1.0),
        bad: Color::new(0.957, 0.247, 0.369, 1.0),
    },
    Theme {
        id: "umbrella_crimson",
        name: "Umbrella Crimson",
        bg_top: Color::new(0.051, 0.039, 0.055, 1.0),
        bg_bot: Color::new(0.098, 0.051, 0.075, 1.0),
        bg_glow: Color::new(0.882, 0.114, 0.282, 0.045),
        top_bar: Color::new(0.039, 0.027, 0.043, 0.97),
        status_bar: Color::new(0.031, 0.024, 0.035, 0.98),
        panel: Color::new(0.098, 0.071, 0.094, 0.94),
        panel2: Color::new(0.145, 0.102, 0.133, 0.96),
        field_bg: Color::new(0.043, 0.031, 0.047, 0.98),
        btn_bg: Color::new(0.184, 0.129, 0.165, 1.0),
        btn_hover: Color::new(0.247, 0.173, 0.220, 1.0),
        btn_disabled: Color::new(0.125, 0.090, 0.114, 0.75),
        line: Color::new(0.94, 0.52, 0.64, 0.13),
        line_hi: Color::new(1.0, 0.70, 0.78, 0.20),
        text: Color::new(0.976, 0.949, 0.961, 1.0),
        muted: Color::new(0.675, 0.576, 0.624, 1.0),
        accent: Color::new(0.882, 0.114, 0.282, 1.0),
        accent_hi: Color::new(0.984, 0.443, 0.522, 1.0),
        accent_soft: Color::new(0.882, 0.114, 0.282, 0.20),
        good: Color::new(0.133, 0.773, 0.369, 1.0),
        warn: Color::new(0.961, 0.620, 0.043, 1.0),
        bad: Color::new(0.984, 0.443, 0.522, 1.0),
    },
];

pub fn theme_index(id: &str) -> usize {
    THEMES.iter().position(|t| t.id == id).unwrap_or(0)
}

/// Non-overlapping rounded rectangle fill so translucent colors blend cleanly.
pub fn fill_rrect(r: Rect, rad: f32, c: Color) {
    let rad = rad.min(r.w * 0.5).min(r.h * 0.5).max(0.0);
    if rad <= 0.5 {
        draw_rectangle(r.x, r.y, r.w, r.h, c);
        return;
    }
    draw_rectangle(r.x + rad, r.y, r.w - rad * 2.0, r.h, c);
    draw_rectangle(r.x, r.y + rad, rad, r.h - rad * 2.0, c);
    draw_rectangle(r.x + r.w - rad, r.y + rad, rad, r.h - rad * 2.0, c);
    let corners = [
        (r.x + rad, r.y + rad, std::f32::consts::PI, std::f32::consts::FRAC_PI_2 * 3.0),
        (r.x + r.w - rad, r.y + rad, std::f32::consts::FRAC_PI_2 * 3.0, std::f32::consts::TAU),
        (r.x + r.w - rad, r.y + r.h - rad, 0.0, std::f32::consts::FRAC_PI_2),
        (r.x + rad, r.y + r.h - rad, std::f32::consts::FRAC_PI_2, std::f32::consts::PI),
    ];
    let steps = 6;
    for (cx, cy, a0, a1) in corners {
        let center = vec2(cx, cy);
        for i in 0..steps {
            let t0 = a0 + (a1 - a0) * (i as f32 / steps as f32);
            let t1 = a0 + (a1 - a0) * ((i + 1) as f32 / steps as f32);
            let p0 = vec2(cx + t0.cos() * rad, cy + t0.sin() * rad);
            let p1 = vec2(cx + t1.cos() * rad, cy + t1.sin() * rad);
            draw_triangle(center, p0, p1, c);
        }
    }
}

/// Crisp rounded rectangle outline.
pub fn stroke_rrect(r: Rect, rad: f32, thickness: f32, c: Color) {
    let rad = rad.min(r.w * 0.5).min(r.h * 0.5).max(0.0);
    if rad <= 0.5 {
        draw_rectangle_lines(r.x, r.y, r.w, r.h, thickness, c);
        return;
    }
    draw_line(r.x + rad, r.y, r.x + r.w - rad, r.y, thickness, c);
    draw_line(r.x + rad, r.y + r.h, r.x + r.w - rad, r.y + r.h, thickness, c);
    draw_line(r.x, r.y + rad, r.x, r.y + r.h - rad, thickness, c);
    draw_line(r.x + r.w, r.y + rad, r.x + r.w, r.y + r.h - rad, thickness, c);
    let corners = [
        (r.x + rad, r.y + rad, std::f32::consts::PI, std::f32::consts::FRAC_PI_2 * 3.0),
        (r.x + r.w - rad, r.y + rad, std::f32::consts::FRAC_PI_2 * 3.0, std::f32::consts::TAU),
        (r.x + r.w - rad, r.y + r.h - rad, 0.0, std::f32::consts::FRAC_PI_2),
        (r.x + rad, r.y + r.h - rad, std::f32::consts::FRAC_PI_2, std::f32::consts::PI),
    ];
    let steps = 6;
    for (cx, cy, a0, a1) in corners {
        for i in 0..steps {
            let t0 = a0 + (a1 - a0) * (i as f32 / steps as f32);
            let t1 = a0 + (a1 - a0) * ((i + 1) as f32 / steps as f32);
            draw_line(
                cx + t0.cos() * rad,
                cy + t0.sin() * rad,
                cx + t1.cos() * rad,
                cy + t1.sin() * rad,
                thickness,
                c,
            );
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Field {
    Search,
    Ip,
    Port,
    Name,
    GameDir,
    Fluffy,
    Preset,
    Chat,
    UpdateUrl,
}

pub struct Ui {
    pub font: Option<Font>,
    pub bold: Option<Font>,
    pub icon: Option<Texture2D>,
    pub mouse: Vec2,
    pub pressed: bool,
    pub consumed: bool,
    pub wheel: f32,
    pub focus: Option<Field>,
    pub caret_t: f64,
    pub active: bool, // our window is in front: only then do we look at the mouse
    pub theme_idx: usize,
}

impl Ui {
    pub fn new() -> Ui {
        let load = |p: &str| std::fs::read(p).ok().and_then(|b| load_ttf_font_from_bytes(&b).ok());
        let font = load("C:/Windows/Fonts/segoeui.ttf");
        let bold = load("C:/Windows/Fonts/segoeuib.ttf").or_else(|| font.clone());
        let icon = image::load_from_memory(APP_ICON_JPG).ok().map(|img| {
            let rgba = img.resize_exact(128, 128, image::imageops::FilterType::Lanczos3).to_rgba8();
            let tex = Texture2D::from_rgba8(rgba.width() as u16, rgba.height() as u16, rgba.as_raw());
            tex.set_filter(FilterMode::Linear);
            tex
        });
        Ui { font, bold, icon, ..Ui::new_light() }
    }

    /// Widgets without fonts (used by the self-test probe).
    pub fn new_light() -> Ui {
        Ui {
            font: None,
            bold: None,
            icon: None,
            mouse: vec2(-1.0, -1.0),
            pressed: false,
            consumed: false,
            wheel: 0.0,
            focus: None,
            caret_t: 0.0,
            active: true,
            theme_idx: 0,
        }
    }

    pub fn theme(&self) -> &'static Theme {
        &THEMES[self.theme_idx % THEMES.len()]
    }

    pub fn begin(&mut self, active: bool) {
        self.active = active;
        if active {
            let (x, y) = mouse_position();
            self.mouse = vec2(x, y);
            self.pressed = is_mouse_button_pressed(MouseButton::Left);
            self.wheel = mouse_wheel().1;
        } else {
            self.mouse = vec2(-1.0, -1.0);
            self.pressed = false;
            self.wheel = 0.0;
        }
        self.consumed = false;
    }

    pub fn hover(&self, r: Rect) -> bool {
        r.contains(self.mouse)
    }

    pub fn click(&mut self, r: Rect) -> bool {
        if self.pressed && !self.consumed && self.hover(r) {
            self.consumed = true;
            true
        } else {
            false
        }
    }

    pub fn text(&self, s: &str, x: f32, y: f32, size: f32, c: Color) {
        draw_text_ex(s, x, y, TextParams { font: self.font.as_ref(), font_size: size as u16, color: c, ..Default::default() });
    }

    pub fn btext(&self, s: &str, x: f32, y: f32, size: f32, c: Color) {
        draw_text_ex(s, x, y, TextParams { font: self.bold.as_ref(), font_size: size as u16, color: c, ..Default::default() });
    }

    pub fn width(&self, s: &str, size: f32) -> f32 {
        measure_text(s, self.font.as_ref(), size as u16, 1.0).width
    }

    /// Cut text to fit `w` pixels, adding "..." when cut.
    pub fn fit(&self, s: &str, size: f32, w: f32) -> String {
        if self.width(s, size) <= w {
            return s.to_string();
        }
        let mut out: String = s.to_string();
        while !out.is_empty() && self.width(&format!("{}...", out), size) > w {
            out.pop();
        }
        format!("{}...", out)
    }

    /// Word-wrapped lines; returns the y after the last line.
    pub fn wrap(&self, s: &str, x: f32, y: f32, w: f32, size: f32, c: Color, max_lines: usize) -> f32 {
        let mut yy = y;
        let mut lines = 0;
        for para in s.split('\n') {
            let mut line = String::new();
            for word in para.split(' ') {
                let cand = if line.is_empty() { word.to_string() } else { format!("{} {}", line, word) };
                if self.width(&cand, size) > w && !line.is_empty() {
                    self.text(&line, x, yy, size, c);
                    yy += size * 1.3;
                    lines += 1;
                    if lines >= max_lines {
                        return yy;
                    }
                    line = word.to_string();
                } else {
                    line = cand;
                }
            }
            self.text(&line, x, yy, size, c);
            yy += size * 1.3;
            lines += 1;
            if lines >= max_lines {
                return yy;
            }
        }
        yy
    }

    pub fn panel(&self, r: Rect, c: Color) {
        let t = self.theme();
        fill_rrect(Rect::new(r.x + 2.0, r.y + 4.0, r.w, r.h), 8.0, Color::new(0.0, 0.0, 0.0, 0.30));
        fill_rrect(r, 8.0, c);
        if r.w > 20.0 {
            draw_line(r.x + 8.0, r.y + 1.0, r.x + r.w - 8.0, r.y + 1.0, 1.0, t.line_hi);
        }
        stroke_rrect(r, 8.0, 1.0, t.line);
    }

    /// A button. `kind`: 0 normal, 1 primary (accent), 2 quiet.
    pub fn button(&mut self, r: Rect, label: &str, kind: u8, enabled: bool) -> bool {
        let t = self.theme();
        let hov = enabled && self.hover(r);
        let base = match kind {
            1 => t.accent,
            2 => Color::new(1.0, 1.0, 1.0, 0.05),
            _ => t.btn_bg,
        };
        let c = if !enabled {
            t.btn_disabled
        } else if hov {
            match kind {
                1 => t.accent_hi,
                2 => Color::new(1.0, 1.0, 1.0, 0.11),
                _ => t.btn_hover,
            }
        } else {
            base
        };
        fill_rrect(r, 6.0, c);
        if enabled && kind == 1 {
            fill_rrect(
                Rect::new(r.x + 1.0, r.y + 1.0, (r.w - 2.0).max(0.0), (r.h * 0.45).max(0.0)),
                5.0,
                Color::new(1.0, 1.0, 1.0, if hov { 0.14 } else { 0.08 }),
            );
            stroke_rrect(r, 6.0, 1.0, if hov { Color::new(1.0, 1.0, 1.0, 0.35) } else { t.accent_hi });
        } else if kind == 0 {
            stroke_rrect(r, 6.0, 1.0, if hov { t.line_hi } else { t.line });
        }
        let size = (r.h * 0.5).clamp(13.0, 17.0);
        let tw = self.width(label, size);
        let fg = if !enabled {
            t.muted
        } else if kind == 1 {
            Color::new(0.98, 0.99, 1.0, 1.0)
        } else {
            t.text
        };
        if kind == 1 && enabled {
            self.btext(label, r.x + (r.w - tw) / 2.0, r.y + r.h / 2.0 + size * 0.35, size, fg);
        } else {
            self.text(label, r.x + (r.w - tw) / 2.0, r.y + r.h / 2.0 + size * 0.35, size, fg);
        }
        enabled && self.click(r)
    }

    pub fn toggle(&mut self, r: Rect, label: &str, on: bool) -> bool {
        let t = self.theme();
        let hov = self.hover(r);
        if hov {
            fill_rrect(r, 6.0, Color::new(1.0, 1.0, 1.0, 0.045));
        }
        let sw = Rect::new(r.x + r.w - 46.0, r.y + (r.h - 20.0) / 2.0, 38.0, 20.0);
        fill_rrect(sw, 10.0, if on { t.accent } else { t.btn_bg });
        stroke_rrect(sw, 10.0, 1.0, if on { t.accent_hi } else { t.line });
        let kx = if on { sw.x + sw.w - 10.0 } else { sw.x + 10.0 };
        let ky = sw.y + sw.h * 0.5;
        draw_circle(kx, ky + 1.0, 7.5, Color::new(0.0, 0.0, 0.0, 0.25));
        draw_circle(kx, ky, 7.5, t.text);
        self.text(label, r.x + 8.0, r.y + r.h / 2.0 + 5.0, 15.0, t.text);
        self.click(r)
    }

    /// Single-line text box. Returns true when Enter is pressed in it.
    pub fn field(&mut self, r: Rect, id: Field, value: &mut String, hint: &str) -> bool {
        let t = self.theme();
        let focused = self.focus == Some(id);
        if self.click(r) {
            self.focus = Some(id);
        }
        fill_rrect(r, 6.0, t.field_bg);
        if focused {
            stroke_rrect(
                Rect::new(r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0),
                7.0,
                2.0,
                Color::new(t.accent.r, t.accent.g, t.accent.b, 0.28),
            );
            stroke_rrect(r, 6.0, 1.5, t.accent_hi);
        } else {
            stroke_rrect(r, 6.0, 1.0, t.line);
        }
        let size = 15.0;
        let ty = r.y + r.h / 2.0 + 5.0;
        if value.is_empty() && !focused {
            self.text(hint, r.x + 10.0, ty, size, t.muted);
        } else {
            let shown = {
                let mut s = value.clone();
                while self.width(&s, size) > r.w - 22.0 && !s.is_empty() {
                    s.remove(0);
                }
                s
            };
            self.text(&shown, r.x + 10.0, ty, size, t.text);
            if focused && (get_time() * 2.0) as i64 % 2 == 0 {
                let cx = r.x + 11.0 + self.width(&shown, size);
                draw_line(cx, r.y + 6.0, cx, r.y + r.h - 6.0, 1.5, t.accent_hi);
            }
        }
        let mut submitted = false;
        if focused && self.active {
            while let Some(c) = get_char_pressed() {
                if !c.is_control() {
                    value.push(c);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                value.pop();
            }
            let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
            if ctrl && is_key_pressed(KeyCode::V) {
                if let Some(t) = macroquad::miniquad::window::clipboard_get() {
                    value.push_str(t.trim());
                }
            }
            if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
                submitted = true;
            }
        }
        submitted
    }

    pub fn tag(&self, x: f32, y: f32, s: &str, c: Color) -> f32 {
        let w = self.width(s, 12.0) + 12.0;
        let r = Rect::new(x, y, w, 19.0);
        fill_rrect(r, 5.0, Color::new(c.r, c.g, c.b, 0.16));
        stroke_rrect(r, 5.0, 1.0, Color::new(c.r, c.g, c.b, 0.35));
        self.text(s, x + 6.0, y + 13.5, 12.0, c);
        w
    }

    pub fn gradient_bg(&self) {
        let t = self.theme();
        let (w, h) = (screen_width(), screen_height());
        let bands = 48;
        for i in 0..bands {
            let k = i as f32 / (bands - 1) as f32;
            let c = Color::new(
                t.bg_top.r + (t.bg_bot.r - t.bg_top.r) * k,
                t.bg_top.g + (t.bg_bot.g - t.bg_top.g) * k,
                t.bg_top.b + (t.bg_bot.b - t.bg_top.b) * k,
                1.0,
            );
            draw_rectangle(0.0, h * i as f32 / bands as f32, w, h / bands as f32 + 1.0, c);
        }
        // Subtle ambient accent wash across top-left and bottom-right.
        let glow = Color::new(t.bg_glow.r, t.bg_glow.g, t.bg_glow.b, 0.012);
        for step in 0..10 {
            let rad = (120.0 + step as f32 * 48.0) * (w / 1280.0).max(0.8);
            draw_circle(w * 0.16, h * 0.10, rad, glow);
            draw_circle(w * 0.86, h * 0.90, rad * 1.1, glow);
        }
        // Subtle diagonal architectural stripes.
        for i in 0..14 {
            let x = i as f32 * 140.0 - 200.0;
            draw_triangle(
                vec2(x, h),
                vec2(x + 56.0, h),
                vec2(x + 420.0, 0.0),
                Color::new(1.0, 1.0, 1.0, 0.011),
            );
        }
    }
}
