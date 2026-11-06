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

use macroquad::models::{draw_mesh, Mesh, Vertex};

#[inline]
fn vtx(x: f32, y: f32, c: Color) -> Vertex {
    Vertex::new(x, y, 0.0, 0.0, 0.0, c)
}

struct PointNormal {
    p: Vec2,
    n: Vec2,
}

fn rrect_contour(r: Rect, rad: f32) -> Vec<PointNormal> {
    let w = r.w.max(0.0);
    let h = r.h.max(0.0);
    let rad = rad.min(w * 0.5).min(h * 0.5).max(0.0);
    let mut out = Vec::with_capacity(36);

    if rad <= 0.5 {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        out.push(PointNormal { p: vec2(r.x + w, r.y), n: vec2(s, -s) });
        out.push(PointNormal { p: vec2(r.x + w, r.y + h), n: vec2(s, s) });
        out.push(PointNormal { p: vec2(r.x, r.y + h), n: vec2(-s, s) });
        out.push(PointNormal { p: vec2(r.x, r.y), n: vec2(-s, -s) });
        return out;
    }

    let c0 = vec2(r.x + w - rad, r.y + rad);       // Top-right
    let c1 = vec2(r.x + w - rad, r.y + h - rad);   // Bottom-right
    let c2 = vec2(r.x + rad, r.y + h - rad);       // Bottom-left
    let c3 = vec2(r.x + rad, r.y + rad);           // Top-left

    let centers = [c0, c1, c2, c3];
    let start_angles = [
        -std::f32::consts::FRAC_PI_2,
        0.0,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
    ];

    let steps = 8;
    for k in 0..4 {
        let c = centers[k];
        let a0 = start_angles[k];
        for i in 0..=steps {
            let theta = a0 + std::f32::consts::FRAC_PI_2 * (i as f32 / steps as f32);
            let n = vec2(theta.cos(), theta.sin());
            out.push(PointNormal { p: c + n * rad, n });
        }
    }
    out
}

/// Silky-smooth anti-aliased rounded rectangle fill using subpixel alpha fringe feathering.
pub fn fill_rrect(r: Rect, rad: f32, c: Color) {
    if r.w <= 0.0 || r.h <= 0.0 || c.a <= 0.001 {
        return;
    }
    let pts = rrect_contour(r, rad);
    let n = pts.len();
    if n < 3 {
        return;
    }

    let c_trans = Color::new(c.r, c.g, c.b, 0.0);
    let center = vec2(r.x + r.w * 0.5, r.y + r.h * 0.5);

    let mut vertices = Vec::with_capacity(1 + n * 2);
    vertices.push(vtx(center.x, center.y, c));

    for pt in &pts {
        let p_in = pt.p - pt.n * 0.5;
        vertices.push(vtx(p_in.x, p_in.y, c));
    }
    for pt in &pts {
        let p_out = pt.p + pt.n * 0.5;
        vertices.push(vtx(p_out.x, p_out.y, c_trans));
    }

    let mut indices = Vec::with_capacity(n * 9);
    for i in 0..n {
        let next = (i + 1) % n;
        let in_curr = (1 + i) as u16;
        let in_next = (1 + next) as u16;
        let out_curr = (1 + n + i) as u16;
        let out_next = (1 + n + next) as u16;

        // Interior fan
        indices.push(0);
        indices.push(in_curr);
        indices.push(in_next);

        // Outer feather ribbon
        indices.push(in_curr);
        indices.push(out_curr);
        indices.push(out_next);

        indices.push(in_curr);
        indices.push(out_next);
        indices.push(in_next);
    }

    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: None,
    });
}

/// Silky-smooth anti-aliased rounded rectangle outline rendered as a continuous connected quad ribbon.
pub fn stroke_rrect(r: Rect, rad: f32, thickness: f32, c: Color) {
    if r.w <= 0.0 || r.h <= 0.0 || c.a <= 0.001 || thickness <= 0.05 {
        return;
    }
    let pts = rrect_contour(r, rad);
    let n = pts.len();
    if n < 3 {
        return;
    }

    let c_trans = Color::new(c.r, c.g, c.b, 0.0);

    if thickness <= 1.0 {
        let alpha = c.a * thickness.clamp(0.0, 1.0);
        let c_mid = Color::new(c.r, c.g, c.b, alpha);

        let mut vertices = Vec::with_capacity(n * 3);
        for pt in &pts {
            let p_in = pt.p - pt.n * 0.5;
            vertices.push(vtx(p_in.x, p_in.y, c_trans));
        }
        for pt in &pts {
            vertices.push(vtx(pt.p.x, pt.p.y, c_mid));
        }
        for pt in &pts {
            let p_out = pt.p + pt.n * 0.5;
            vertices.push(vtx(p_out.x, p_out.y, c_trans));
        }

        let mut indices = Vec::with_capacity(n * 12);
        for i in 0..n {
            let next = (i + 1) % n;
            let in_curr = i as u16;
            let in_next = next as u16;
            let mid_curr = (n + i) as u16;
            let mid_next = (n + next) as u16;
            let out_curr = (2 * n + i) as u16;
            let out_next = (2 * n + next) as u16;

            indices.push(in_curr);
            indices.push(mid_curr);
            indices.push(mid_next);

            indices.push(in_curr);
            indices.push(mid_next);
            indices.push(in_next);

            indices.push(mid_curr);
            indices.push(out_curr);
            indices.push(out_next);

            indices.push(mid_curr);
            indices.push(out_next);
            indices.push(mid_next);
        }

        draw_mesh(&Mesh {
            vertices,
            indices,
            texture: None,
        });
    } else {
        let half_t = thickness * 0.5;
        let r_in_fade = half_t + 0.5;
        let r_in_core = half_t - 0.5;
        let r_out_core = half_t - 0.5;
        let r_out_fade = half_t + 0.5;

        let mut vertices = Vec::with_capacity(n * 4);
        for pt in &pts {
            let p = pt.p - pt.n * r_in_fade;
            vertices.push(vtx(p.x, p.y, c_trans));
        }
        for pt in &pts {
            let p = pt.p - pt.n * r_in_core;
            vertices.push(vtx(p.x, p.y, c));
        }
        for pt in &pts {
            let p = pt.p + pt.n * r_out_core;
            vertices.push(vtx(p.x, p.y, c));
        }
        for pt in &pts {
            let p = pt.p + pt.n * r_out_fade;
            vertices.push(vtx(p.x, p.y, c_trans));
        }

        let mut indices = Vec::with_capacity(n * 18);
        for i in 0..n {
            let next = (i + 1) % n;
            let in_f_curr = i as u16;
            let in_f_next = next as u16;
            let in_c_curr = (n + i) as u16;
            let in_c_next = (n + next) as u16;
            let out_c_curr = (2 * n + i) as u16;
            let out_c_next = (2 * n + next) as u16;
            let out_f_curr = (3 * n + i) as u16;
            let out_f_next = (3 * n + next) as u16;

            // Inner fade ribbon
            indices.push(in_f_curr);
            indices.push(in_c_curr);
            indices.push(in_c_next);

            indices.push(in_f_curr);
            indices.push(in_c_next);
            indices.push(in_f_next);

            // Solid core ribbon
            indices.push(in_c_curr);
            indices.push(out_c_curr);
            indices.push(out_c_next);

            indices.push(in_c_curr);
            indices.push(out_c_next);
            indices.push(in_c_next);

            // Outer fade ribbon
            indices.push(out_c_curr);
            indices.push(out_f_curr);
            indices.push(out_f_next);

            indices.push(out_c_curr);
            indices.push(out_f_next);
            indices.push(out_c_next);
        }

        draw_mesh(&Mesh {
            vertices,
            indices,
            texture: None,
        });
    }
}

/// Silky-smooth anti-aliased circle fill.
pub fn fill_circle_aa(cx: f32, cy: f32, rad: f32, c: Color) {
    if rad <= 0.0 || c.a <= 0.001 {
        return;
    }
    let steps = 32;
    let mut vertices = Vec::with_capacity(1 + steps * 2);
    let mut indices = Vec::with_capacity(steps * 9);
    let c_trans = Color::new(c.r, c.g, c.b, 0.0);
    vertices.push(vtx(cx, cy, c));

    let r_in = (rad - 0.5).max(0.0);
    let r_out = rad + 0.5;

    for i in 0..steps {
        let theta = std::f32::consts::TAU * (i as f32 / steps as f32);
        let nx = theta.cos();
        let ny = theta.sin();
        vertices.push(vtx(cx + nx * r_in, cy + ny * r_in, c));
    }
    for i in 0..steps {
        let theta = std::f32::consts::TAU * (i as f32 / steps as f32);
        let nx = theta.cos();
        let ny = theta.sin();
        vertices.push(vtx(cx + nx * r_out, cy + ny * r_out, c_trans));
    }

    for i in 0..steps {
        let next = (i + 1) % steps;
        let in_curr = (1 + i) as u16;
        let in_next = (1 + next) as u16;
        let out_curr = (1 + steps + i) as u16;
        let out_next = (1 + steps + next) as u16;

        indices.push(0);
        indices.push(in_curr);
        indices.push(in_next);

        indices.push(in_curr);
        indices.push(out_curr);
        indices.push(out_next);

        indices.push(in_curr);
        indices.push(out_next);
        indices.push(in_next);
    }

    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: None,
    });
}

/// Silky-smooth anti-aliased line.
pub fn draw_line_aa(x1: f32, y1: f32, x2: f32, y2: f32, thickness: f32, c: Color) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len = (dx * dx + dy * dy).sqrt();
    if len <= 0.001 || thickness <= 0.01 || c.a <= 0.001 {
        return;
    }
    let nx = -dy / len;
    let ny = dx / len;
    let tx = dx / len;
    let ty = dy / len;

    let half_t = thickness * 0.5;
    let r_core = (half_t - 0.5).max(0.0);
    let r_fade = half_t + 0.5;
    let c_trans = Color::new(c.r, c.g, c.b, 0.0);

    let p1 = vec2(x1, y1);
    let p2 = vec2(x2, y2);
    let n = vec2(nx, ny);
    let t = vec2(tx, ty);

    let c0 = p1 - n * r_core;
    let c1 = p1 + n * r_core;
    let c2 = p2 + n * r_core;
    let c3 = p2 - n * r_core;

    let o0 = p1 - n * r_fade - t * 0.5;
    let o1 = p1 + n * r_fade - t * 0.5;
    let o2 = p2 + n * r_fade + t * 0.5;
    let o3 = p2 - n * r_fade + t * 0.5;

    let vertices = vec![
        vtx(c0.x, c0.y, c),
        vtx(c1.x, c1.y, c),
        vtx(c2.x, c2.y, c),
        vtx(c3.x, c3.y, c),
        vtx(o0.x, o0.y, c_trans),
        vtx(o1.x, o1.y, c_trans),
        vtx(o2.x, o2.y, c_trans),
        vtx(o3.x, o3.y, c_trans),
    ];
    let indices = vec![
        0, 1, 2, 0, 2, 3,
        1, 5, 6, 1, 6, 2,
        4, 0, 3, 4, 3, 7,
        4, 5, 1, 4, 1, 0,
        3, 2, 6, 3, 6, 7,
    ];
    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: None,
    });
}

/// Crystal-clear vector anti-aliased play triangle icon.
pub fn draw_play_triangle_aa(x: f32, y: f32, size: f32, c: Color) {
    if size <= 0.0 || c.a <= 0.001 {
        return;
    }
    let p0 = vec2(x, y);
    let p1 = vec2(x + size * 0.92, y + size * 0.5);
    let p2 = vec2(x, y + size);

    let d0 = (p1 - p0).normalize();
    let n0 = vec2(d0.y, -d0.x);
    let d1 = (p2 - p1).normalize();
    let n1 = vec2(d1.y, -d1.x);
    let n2 = vec2(-1.0, 0.0);

    let cn0 = (n2 + n0).normalize();
    let cn1 = (n0 + n1).normalize();
    let cn2 = (n1 + n2).normalize();

    let c_trans = Color::new(c.r, c.g, c.b, 0.0);

    let in0 = p0 - cn0 * 0.5;
    let in1 = p1 - cn1 * 0.5;
    let in2 = p2 - cn2 * 0.5;

    let out0 = p0 + cn0 * 0.5;
    let out1 = p1 + cn1 * 0.5;
    let out2 = p2 + cn2 * 0.5;

    let vertices = vec![
        vtx(in0.x, in0.y, c),
        vtx(in1.x, in1.y, c),
        vtx(in2.x, in2.y, c),
        vtx(out0.x, out0.y, c_trans),
        vtx(out1.x, out1.y, c_trans),
        vtx(out2.x, out2.y, c_trans),
    ];

    let indices = vec![
        0, 1, 2,
        0, 3, 4, 0, 4, 1,
        1, 4, 5, 1, 5, 2,
        2, 5, 3, 2, 3, 0,
    ];

    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: None,
    });
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
        let win_dir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
        let load = |name: &str| {
            let p = format!("{}\\Fonts\\{}", win_dir, name);
            std::fs::read(&p).ok().and_then(|b| {
                let mut f = load_ttf_font_from_bytes(&b).ok()?;
                f.set_filter(FilterMode::Linear);
                Some(f)
            })
        };
        let font = load("segoeui.ttf").or_else(|| load("arial.ttf")).or_else(|| load("tahoma.ttf"));
        let bold = load("segoeuib.ttf").or_else(|| load("arialbd.ttf")).or_else(|| font.clone());
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

    pub fn play_icon(&self, x: f32, y: f32, size: f32, c: Color) {
        draw_play_triangle_aa(x, y, size, c);
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
            draw_line_aa(r.x + 8.0, r.y + 1.0, r.x + r.w - 8.0, r.y + 1.0, 1.0, t.line_hi);
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
        fill_circle_aa(kx, ky + 1.0, 7.5, Color::new(0.0, 0.0, 0.0, 0.25));
        fill_circle_aa(kx, ky, 7.5, t.text);
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
                draw_line_aa(cx, r.y + 6.0, cx, r.y + r.h - 6.0, 1.5, t.accent_hi);
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
