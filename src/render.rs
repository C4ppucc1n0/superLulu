//! All drawing: procedural world art (soft, rounded cartoon style, one look per level theme) + the Lulu sprite.

mod boss;

use std::sync::OnceLock;

use macroquad::prelude::*;

use crate::game::*;
use crate::level::{Theme, Tile};
use crate::physics::*;
use crate::save::Progress;

/// UI font: Microsoft YaHei from Windows (Chinese text). Falls back to the built-in font if it's missing.
static FONT: OnceLock<Option<Font>> = OnceLock::new();

pub fn load_font() {
    let dir = std::path::PathBuf::from(std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".into())).join("Fonts");
    let font = ["msyhbd.ttc", "msyh.ttc", "simhei.ttf", "Deng.ttf"]
        .iter()
        .filter_map(|f| std::fs::read(dir.join(f)).ok())
        .find_map(|bytes| load_ttf_font_from_bytes(&bytes).ok());
    let _ = FONT.set(font);
}

fn font() -> Option<&'static Font> {
    FONT.get().and_then(|f| f.as_ref())
}

fn tp(size: f32, color: Color) -> TextParams<'static> {
    TextParams { font: font(), font_size: size.round().max(1.0) as u16, font_scale: 1.0, color, ..Default::default() }
}

fn measure(s: &str, size: f32) -> TextDimensions {
    measure_text(s, font(), size.round().max(1.0) as u16, 1.0)
}

pub struct Assets {
    pub lulu: Texture2D,
    pub lulu_title: Texture2D,
}

impl Assets {
    pub fn load() -> Assets {
        let lulu = Texture2D::from_file_with_format(include_bytes!("../assets/sprites/lulu.png"), Some(ImageFormat::Png));
        let lulu_title = Texture2D::from_file_with_format(include_bytes!("../assets/sprites/lulu_title.png"), Some(ImageFormat::Png));
        lulu.set_filter(FilterMode::Linear);
        lulu_title.set_filter(FilterMode::Linear);
        Assets { lulu, lulu_title }
    }
}

/// Drawn height of Lulu in world units (hitbox is 38 tall; the orange on the cap sticks out above it).
pub const SPRITE_H: f32 = 50.0;

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
}
const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0)
}
fn with_a(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * a)
}

const COOKIE: Color = rgb(240, 200, 132);
const COOKIE_DARK: Color = rgb(196, 146, 82);
const CHOC: Color = rgb(112, 70, 44);
const METAL: Color = rgb(222, 226, 238);
const METAL_DARK: Color = rgb(96, 102, 124);
const ORANGE: Color = rgb(255, 150, 30);
const ORANGE_LIGHT: Color = rgb(255, 205, 120);
const ORANGE_DARK: Color = rgb(214, 104, 20);
const LEAF: Color = rgb(80, 170, 60);
const GOLD: Color = rgb(255, 214, 52);
const GOLD_LIGHT: Color = rgb(255, 246, 180);
const INK: Color = rgb(52, 34, 26);
const HEART: Color = rgb(255, 84, 110);
const HEART_DARK: Color = rgb(196, 40, 70);
const JELLY_PINK: Color = rgb(255, 120, 190);
const JELLY_LIGHT: Color = rgb(255, 200, 232);
const TIE: Color = rgb(255, 104, 160);
const TIE_DARK: Color = rgb(176, 44, 104);
const TIE_LIGHT: Color = rgb(255, 196, 220);

/// Per-theme tile colours.
struct Pal {
    fill: Color,
    fill_dark: Color,
    speck: Color,
    top: Color,
    top_light: Color,
    top_dark: Color,
    stone: Color,
    stone_light: Color,
    stone_mid: Color,
    stone_dark: Color,
    plank: Color,
    plank_light: Color,
    plank_dark: Color,
}

const MEADOW: Pal = Pal {
    fill: rgb(214, 146, 86),
    fill_dark: rgb(181, 113, 62),
    speck: rgb(194, 126, 70),
    top: rgb(118, 202, 82),
    top_light: rgb(168, 230, 118),
    top_dark: rgb(84, 160, 62),
    stone: rgb(156, 166, 196),
    stone_light: rgb(192, 201, 226),
    stone_mid: rgb(128, 137, 168),
    stone_dark: rgb(96, 104, 134),
    plank: rgb(198, 140, 80),
    plank_light: rgb(232, 182, 116),
    plank_dark: rgb(146, 98, 54),
};

const FACTORY: Pal = Pal {
    fill: rgb(206, 148, 90),
    fill_dark: rgb(150, 96, 56),
    speck: rgb(104, 62, 40),
    top: rgb(255, 172, 204),
    top_light: rgb(255, 222, 236),
    top_dark: rgb(222, 120, 164),
    stone: rgb(160, 170, 190),
    stone_light: rgb(204, 212, 228),
    stone_mid: rgb(132, 140, 162),
    stone_dark: rgb(88, 94, 116),
    plank: rgb(238, 200, 150),
    plank_light: rgb(255, 230, 190),
    plank_dark: rgb(190, 140, 90),
};

const SKY: Pal = Pal {
    fill: rgb(246, 249, 255),
    fill_dark: rgb(168, 190, 234),
    speck: rgb(226, 236, 252),
    top: rgb(255, 255, 255),
    top_light: rgb(255, 255, 255),
    top_dark: rgb(210, 226, 250),
    stone: rgb(206, 182, 240),
    stone_light: rgb(236, 222, 255),
    stone_mid: rgb(176, 150, 220),
    stone_dark: rgb(132, 106, 182),
    plank: rgb(255, 255, 255),
    plank_light: rgb(255, 255, 255),
    plank_dark: rgb(186, 206, 240),
};

const STORM: Pal = Pal {
    fill: rgb(134, 142, 180),
    fill_dark: rgb(78, 84, 124),
    speck: rgb(116, 124, 164),
    top: rgb(184, 192, 222),
    top_light: rgb(214, 220, 242),
    top_dark: rgb(108, 116, 156),
    stone: rgb(110, 102, 154),
    stone_light: rgb(150, 142, 194),
    stone_mid: rgb(90, 82, 132),
    stone_dark: rgb(62, 56, 98),
    plank: rgb(196, 204, 230),
    plank_light: rgb(226, 230, 246),
    plank_dark: rgb(112, 120, 160),
};

fn pal(t: Theme) -> &'static Pal {
    match t {
        Theme::Meadow => &MEADOW,
        Theme::Factory => &FACTORY,
        Theme::Sky => &SKY,
        Theme::Storm => &STORM,
    }
}

/// Run `f` with the world rotated by `angle` (radians) around `pivot` (world space).
fn with_rotation(cam: Vec2, view_w: f32, pivot: Vec2, angle: f32, f: impl FnOnce()) {
    let base = world_cam(cam.x, cam.y, view_w);
    let offset = vec2(base.zoom.x * (pivot.x - base.target.x), -base.zoom.y * (pivot.y - base.target.y));
    set_camera(&Camera2D { target: pivot, zoom: base.zoom, rotation: angle.to_degrees(), offset, ..Default::default() });
    f();
    set_camera(&base);
}

pub fn world_cam(x: f32, y: f32, w: f32) -> Camera2D {
    Camera2D { target: vec2(x + w / 2.0, y + VIEW_H / 2.0), zoom: vec2(2.0 / w, 2.0 / VIEW_H), ..Default::default() }
}

fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

fn gradient_rect(x: f32, y: f32, w: f32, h: f32, top: Color, bottom: Color) {
    let v = |px: f32, py: f32, c: Color| Vertex::new(px, py, 0.0, 0.0, 0.0, c);
    let mesh = Mesh {
        vertices: vec![v(x, y, top), v(x + w, y, top), v(x + w, y + h, bottom), v(x, y + h, bottom)],
        indices: vec![0, 1, 2, 0, 2, 3],
        texture: None,
    };
    draw_mesh(&mesh);
}

fn cloud(cx: f32, cy: f32, s: f32, color: Color) {
    draw_circle(cx, cy, 18.0 * s, color);
    draw_circle(cx + 20.0 * s, cy - 8.0 * s, 22.0 * s, color);
    draw_circle(cx + 44.0 * s, cy, 17.0 * s, color);
    draw_rectangle(cx, cy, 44.0 * s, 17.0 * s, color);
}

fn draw_heart(c: Vec2, r: f32, color: Color, dark: Color) {
    for col in [dark, color] {
        let k = if col == dark { 1.0 + 1.6 / r.max(1.0) } else { 1.0 };
        let rr = r * k;
        draw_circle(c.x - rr * 0.48, c.y - rr * 0.18, rr * 0.54, col);
        draw_circle(c.x + rr * 0.48, c.y - rr * 0.18, rr * 0.54, col);
        draw_triangle(vec2(c.x - rr * 0.98, c.y - 0.02 * rr), vec2(c.x + rr * 0.98, c.y - 0.02 * rr), vec2(c.x, c.y + rr * 0.95), col);
    }
    draw_circle(c.x - r * 0.5, c.y - r * 0.32, r * 0.18, rgba(255, 255, 255, 190));
}

fn draw_gear(cx: f32, cy: f32, r: f32, rot: f32, color: Color, hole: Color) {
    let teeth = 10;
    for k in 0..teeth {
        let a = rot + k as f32 * std::f32::consts::TAU / teeth as f32;
        draw_rectangle_ex(
            cx + a.cos() * r,
            cy + a.sin() * r,
            r * 0.36,
            r * 0.3,
            DrawRectangleParams { offset: vec2(0.5, 0.5), rotation: a, color },
        );
    }
    draw_circle(cx, cy, r, color);
    draw_circle(cx, cy, r * 0.34, hole);
}

// ------------------------------------------------------------------------------------------ backgrounds

fn layer_cam(cam: Vec2, view_w: f32, level_h: f32, fx: f32, fy: f32) -> f32 {
    let max_y = level_h - VIEW_H;
    let lx = cam.x * fx;
    let ly = max_y + (cam.y - max_y) * fy;
    set_camera(&world_cam(lx, ly, view_w));
    lx
}

fn span(lx: f32, view_w: f32, spacing: f32) -> std::ops::RangeInclusive<i32> {
    ((lx / spacing).floor() as i32 - 2)..=(((lx + view_w) / spacing).ceil() as i32 + 2)
}

pub fn draw_background(theme: Theme, cam: Vec2, view_w: f32, level_h: f32, t: f32) {
    match theme {
        Theme::Meadow => bg_meadow(cam, view_w, level_h, t),
        Theme::Factory => bg_factory(cam, view_w, level_h, t),
        Theme::Sky => bg_sky(cam, view_w, level_h, t),
        Theme::Storm => bg_storm(cam, view_w, level_h, t, 0.0),
    }
}

/// Dark storm sky; `clear` (0..1) fades it into the sunny sky once the boss is beaten.
fn bg_storm(cam: Vec2, view_w: f32, level_h: f32, t: f32, clear: f32) {
    if clear > 0.0 {
        bg_sky(cam, view_w, level_h, t);
    }
    let a = 1.0 - clear;
    if a <= 0.0 {
        return;
    }
    set_default_camera();
    let (sw, sh) = (screen_width(), screen_height());
    gradient_rect(0.0, 0.0, sw, sh, with_a(rgb(40, 42, 80), a), with_a(rgb(104, 88, 132), a));
    // heavy cloud layers
    for (fx, fy, col, base, seed, spacing) in
        [(0.08, 0.2, rgb(74, 74, 114), 90.0, 3, 230.0), (0.2, 0.35, rgb(60, 60, 98), 200.0, 4, 190.0), (0.35, 0.5, rgb(84, 86, 126), 470.0, 6, 150.0)]
    {
        let lx = layer_cam(cam, view_w, level_h, fx, fy);
        for k in span(lx, view_w, spacing) {
            let h = hash(k, seed);
            let cx = k as f32 * spacing + (h % 80) as f32 + (t * 8.0 * fx) % spacing;
            let cy = base + ((h >> 8) % 90) as f32;
            cloud(cx, cy, 1.0 + ((h >> 16) % 60) as f32 / 100.0, with_a(col, a));
        }
        if base > 400.0 {
            draw_rectangle(lx - 50.0, base + 15.0, view_w + 100.0, 2000.0, with_a(col, a));
        }
    }
    // rain
    set_default_camera();
    let u = sh / VIEW_H;
    for i in 0..70 {
        let h = hash(i, 77);
        let x = ((h % 1000) as f32 / 1000.0 * (sw + 200.0) + t * 90.0 * u) % (sw + 200.0) - 100.0;
        let y = (((h >> 10) % 1000) as f32 / 1000.0 * sh + t * 420.0 * u) % sh;
        draw_line(x, y, x - 5.0 * u, y + 14.0 * u, 1.2 * u, with_a(rgba(190, 200, 240, 90), a));
    }
}

fn bg_meadow(cam: Vec2, view_w: f32, level_h: f32, t: f32) {
    set_default_camera();
    gradient_rect(0.0, 0.0, screen_width(), screen_height(), rgb(96, 170, 245), rgb(196, 232, 255));
    // clouds
    let lx = layer_cam(cam, view_w, level_h, 0.12, 0.25);
    for k in span(lx, view_w, 260.0) {
        let h = hash(k, 7);
        let cx = k as f32 * 260.0 + (h % 120) as f32 + (t * 6.0) % 260.0;
        let cy = 120.0 + ((h >> 8) % 140) as f32;
        cloud(cx, cy, 0.7 + ((h >> 16) % 50) as f32 / 100.0, rgba(255, 255, 255, 235));
    }
    // two hill layers, tangerine trees on the near one
    for (i, &(fx, fy, color, base, radius, spacing, seed)) in
        [(0.25, 0.4, rgb(170, 214, 236), 560.0, 120.0, 190.0, 1), (0.45, 0.6, rgb(146, 206, 150), 590.0, 80.0, 150.0, 2)].iter().enumerate()
    {
        let lx = layer_cam(cam, view_w, level_h, fx, fy);
        for k in span(lx, view_w, spacing) {
            let h = hash(k, seed);
            let r = radius + (h % 50) as f32;
            let cx = k as f32 * spacing + ((h >> 8) % 60) as f32;
            draw_circle(cx, base + r * 0.25, r, color);
            if i == 1 && h % 3 == 0 {
                let tx = cx + ((h >> 4) % 40) as f32 - 20.0;
                let ty = base + r * 0.25 - r * 0.92;
                draw_rectangle(tx - 2.5, ty, 5.0, 18.0, rgb(150, 110, 80));
                draw_circle(tx, ty - 4.0, 13.0, rgb(110, 180, 100));
                for (ox, oy) in [(-6.0, -6.0), (5.0, -2.0), (0.0, -11.0)] {
                    draw_circle(tx + ox, ty + oy, 2.6, rgb(255, 170, 70));
                }
            }
        }
        draw_rectangle(lx - 50.0, base + 20.0, view_w + 100.0, 2000.0, color);
    }
}

fn bg_factory(cam: Vec2, view_w: f32, level_h: f32, t: f32) {
    set_default_camera();
    gradient_rect(0.0, 0.0, screen_width(), screen_height(), rgb(255, 226, 194), rgb(236, 176, 132));
    // far wall: tall arched windows with sky behind them
    let lx = layer_cam(cam, view_w, level_h, 0.15, 0.3);
    for k in span(lx, view_w, 240.0) {
        let x = k as f32 * 240.0 + 60.0;
        let (y, w, h) = (150.0, 90.0, 190.0);
        draw_rectangle(x - 8.0, y - 8.0, w + 16.0, h + 16.0, rgb(232, 168, 124));
        draw_circle(x + w / 2.0, y, w / 2.0 + 8.0, rgb(232, 168, 124));
        gradient_rect(x, y, w, h, rgb(140, 200, 255), rgb(214, 238, 255));
        draw_circle(x + w / 2.0, y, w / 2.0, rgb(140, 200, 255));
        cloud(x + 10.0 + (hash(k, 3) % 30) as f32, y + 60.0, 0.45, rgba(255, 255, 255, 220));
        draw_rectangle(x + w / 2.0 - 3.0, y - w / 2.0, 6.0, h + w / 2.0, rgb(232, 168, 124));
        draw_rectangle(x, y + 70.0, w, 6.0, rgb(232, 168, 124));
    }
    // gears and pipes
    let lx = layer_cam(cam, view_w, level_h, 0.35, 0.45);
    let gear = rgb(220, 150, 108);
    for k in span(lx, view_w, 330.0) {
        let h = hash(k, 11);
        let cx = k as f32 * 330.0 + (h % 80) as f32;
        let cy = 380.0 + ((h >> 8) % 120) as f32;
        let r = 46.0 + ((h >> 16) % 30) as f32;
        let dir = if h % 2 == 0 { 1.0 } else { -1.0 };
        draw_gear(cx, cy, r, t * 0.4 * dir, gear, rgb(240, 186, 142));
        draw_gear(cx + r * 1.55, cy - r * 0.8, r * 0.55, -t * 0.4 * dir * 1.8 + 0.3, gear, rgb(240, 186, 142));
    }
    draw_rectangle(lx - 50.0, 300.0, view_w + 100.0, 14.0, rgb(214, 140, 100));
    for k in span(lx, view_w, 160.0) {
        draw_rectangle(k as f32 * 160.0, 296.0, 14.0, 22.0, rgb(196, 124, 88));
    }
    // near: big cookie jars on shelves
    let lx = layer_cam(cam, view_w, level_h, 0.55, 0.62);
    let shelf = rgb(198, 128, 88);
    for k in span(lx, view_w, 210.0) {
        let h = hash(k, 21);
        if h % 3 == 0 {
            continue;
        }
        let x = k as f32 * 210.0 + (h % 60) as f32;
        let base = 600.0;
        draw_rectangle(x - 14.0, base - 110.0, 76.0, 110.0, rgb(214, 150, 112));
        draw_rectangle(x - 18.0, base - 122.0, 84.0, 14.0, shelf);
        draw_circle(x + 24.0, base - 60.0, 22.0, rgb(232, 178, 120));
        for (ox, oy) in [(-8.0, -6.0), (6.0, 4.0), (10.0, -10.0)] {
            draw_circle(x + 24.0 + ox, base - 60.0 + oy, 3.2, rgb(170, 110, 76));
        }
    }
    draw_rectangle(lx - 50.0, 600.0, view_w + 100.0, 2000.0, shelf);
}

fn bg_sky(cam: Vec2, view_w: f32, level_h: f32, t: f32) {
    set_default_camera();
    let (sw, sh) = (screen_width(), screen_height());
    gradient_rect(0.0, 0.0, sw, sh, rgb(110, 172, 252), rgb(255, 222, 238));
    // sun
    for (r, a) in [(0.2, 40), (0.14, 70), (0.09, 255)] {
        draw_circle(sw * 0.82, sh * 0.2, sh * r, rgba(255, 246, 200, a));
    }
    // far clouds
    let lx = layer_cam(cam, view_w, level_h, 0.08, 0.2);
    for k in span(lx, view_w, 200.0) {
        let h = hash(k, 5);
        let cx = k as f32 * 200.0 + (h % 90) as f32 + (t * 4.0) % 200.0;
        cloud(cx, 90.0 + ((h >> 8) % 220) as f32, 0.5 + ((h >> 16) % 40) as f32 / 100.0, rgba(255, 255, 255, 170));
    }
    // floating islands
    let lx = layer_cam(cam, view_w, level_h, 0.25, 0.35);
    for k in span(lx, view_w, 280.0) {
        let h = hash(k, 9);
        if h % 2 == 0 {
            continue;
        }
        let cx = k as f32 * 280.0 + (h % 100) as f32;
        let cy = 260.0 + ((h >> 8) % 160) as f32 + (t * 0.8 + k as f32).sin() * 4.0;
        let w = 50.0 + ((h >> 16) % 40) as f32;
        draw_triangle(vec2(cx - w, cy), vec2(cx + w, cy), vec2(cx, cy + w * 0.9), rgb(196, 176, 226));
        draw_ellipse(cx, cy, w, 9.0, 0.0, rgb(164, 214, 170));
        draw_circle(cx - w * 0.3, cy - 10.0, 9.0, rgb(150, 200, 160));
        draw_circle(cx - w * 0.3 + 3.0, cy - 13.0, 2.4, rgb(255, 170, 70));
    }
    // cloud sea
    for (fx, fy, base, col, seed) in [(0.4, 0.5, 560.0, rgba(238, 240, 255, 200), 13), (0.6, 0.7, 600.0, rgb(210, 220, 248), 17)] {
        let lx = layer_cam(cam, view_w, level_h, fx, fy);
        for k in span(lx, view_w, 70.0) {
            let h = hash(k, seed);
            let r = 40.0 + (h % 30) as f32;
            draw_circle(k as f32 * 70.0, base + 10.0 + (t * 1.2 + k as f32 * 0.7).sin() * 3.0, r, col);
        }
        draw_rectangle(lx - 50.0, base + 20.0, view_w + 100.0, 2000.0, col);
    }
}

// ------------------------------------------------------------------------------------------ tiles

fn solidish(t: Tile) -> bool {
    matches!(
        t,
        Tile::Ground
            | Tile::Stone
            | Tile::TurretL
            | Tile::TurretR
            | Tile::Fake
            | Tile::Fan
            | Tile::Spring
            | Tile::SlideR
            | Tile::SlideL
            | Tile::Cracked
            | Tile::Pedestal
    )
}

fn groundish(t: Tile) -> bool {
    matches!(t, Tile::Ground | Tile::Fake | Tile::SlideR | Tile::SlideL)
}

fn draw_ground(g: &Game, tx: i32, ty: i32, alpha: f32) {
    let (x, y) = (tx as f32 * TILE, ty as f32 * TILE);
    let lv = &g.lv;
    let p = pal(lv.theme);
    let c = |col: Color| with_a(col, alpha);
    draw_rectangle(x, y, TILE, TILE, c(p.fill));
    let h = hash(tx, ty);
    match lv.theme {
        Theme::Factory => {
            for k in 0..2 {
                let hx = ((h >> (k * 6)) % 22) as f32 + 5.0;
                let hy = ((h >> (k * 6 + 3)) % 16) as f32 + 12.0;
                draw_circle(x + hx, y + hy, 2.8, c(p.speck));
            }
        }
        _ => {
            for k in 0..3 {
                let hx = ((h >> (k * 6)) % 24) as f32 + 4.0;
                let hy = ((h >> (k * 6 + 3)) % 18) as f32 + 10.0;
                draw_circle(x + hx, y + hy, 2.2 + (k as f32) * 0.4, c(p.speck));
            }
        }
    }
    let open_l = !groundish(lv.tile(tx - 1, ty));
    let open_r = !groundish(lv.tile(tx + 1, ty));
    if open_l {
        draw_rectangle(x, y, 3.0, TILE, c(p.fill_dark));
    }
    if open_r {
        draw_rectangle(x + TILE - 3.0, y, 3.0, TILE, c(p.fill_dark));
    }
    if ty + 1 < lv.h && !groundish(lv.tile(tx, ty + 1)) {
        draw_rectangle(x, y + TILE - 4.0, TILE, 4.0, c(p.fill_dark));
    }
    if solidish(lv.tile(tx, ty - 1)) {
        return;
    }
    // exposed top: grass / icing / fluffy cloud
    let bump = if matches!(lv.theme, Theme::Sky | Theme::Storm) { 6.0 } else { 4.5 };
    draw_rectangle(x, y + 9.0, TILE, 4.0, c(p.top_dark));
    if lv.theme == Theme::Factory {
        // icing drips
        for k in 0..2 {
            let dx = 5.0 + ((h >> (k * 4)) % 20) as f32;
            let len = 4.0 + ((h >> (k * 4 + 2)) % 9) as f32;
            draw_rectangle(x + dx - 2.5, y + 9.0, 5.0, len, c(p.top));
            draw_circle(x + dx, y + 9.0 + len, 2.5, c(p.top));
        }
    }
    draw_rectangle(x, y, TILE, 9.0, c(p.top));
    for i in 0..4 {
        draw_circle(x + 4.0 + i as f32 * 8.0, y + 9.0, bump, c(p.top));
    }
    draw_rectangle(x, y, TILE, 3.0, c(p.top_light));
    if open_l {
        draw_circle(x + 1.0, y + 6.0, 5.5, c(p.top_dark));
        draw_circle(x + 2.5, y + 5.0, 5.0, c(p.top));
    }
    if open_r {
        draw_circle(x + TILE - 1.0, y + 6.0, 5.5, c(p.top_dark));
        draw_circle(x + TILE - 2.5, y + 5.0, 5.0, c(p.top));
    }
    if lv.theme == Theme::Factory {
        // sprinkles
        for k in 0..3 {
            let sx = x + 4.0 + ((h >> (k * 5 + 1)) % 24) as f32;
            let sy = y + 2.0 + ((h >> (k * 5 + 3)) % 6) as f32;
            let col = [rgb(255, 255, 255), rgb(120, 200, 255), rgb(255, 220, 90), rgb(140, 220, 140)][((h >> (k * 3)) % 4) as usize];
            draw_rectangle(sx, sy, 4.0, 1.8, c(col));
        }
    }
}

fn draw_decor(g: &Game, tx: i32, ty: i32) {
    let (x, y) = (tx as f32 * TILE, ty as f32 * TILE);
    let p = pal(g.lv.theme);
    let h = hash(tx, ty + 1000);
    match (g.lv.theme, h % 9) {
        (Theme::Meadow, 0) => {
            let fx = x + 8.0 + (h >> 8) as f32 % 16.0;
            draw_line(fx, y, fx, y - 9.0, 1.5, p.top_dark);
            let col = [rgb(255, 120, 150), rgb(255, 220, 90), rgb(255, 255, 255), rgb(190, 140, 255)][(h >> 4) as usize % 4];
            for k in 0..5 {
                let a = k as f32 * 1.2566;
                draw_circle(fx + a.cos() * 3.0, y - 10.0 + a.sin() * 3.0, 2.2, col);
            }
            draw_circle(fx, y - 10.0, 1.6, rgb(255, 190, 60));
        }
        (Theme::Meadow, 1 | 2) => {
            let fx = x + 6.0 + (h >> 8) as f32 % 20.0;
            for k in -1i32..=1 {
                let kx = k as f32 * 3.0;
                draw_triangle(vec2(fx + kx - 2.0, y + 1.0), vec2(fx + kx + 2.0, y + 1.0), vec2(fx + kx * 1.6, y - 7.0 + k.abs() as f32 * 2.0), p.top_dark);
            }
        }
        (Theme::Meadow, 3) => {
            let fx = x + 16.0;
            draw_circle(fx - 7.0, y - 3.0, 7.0, p.top_dark);
            draw_circle(fx + 6.0, y - 3.0, 8.0, p.top_dark);
            draw_circle(fx, y - 7.0, 8.0, rgb(100, 182, 70));
            draw_circle(fx - 2.0, y - 10.0, 3.0, p.top_light);
        }
        (Theme::Factory, 0) => {
            // lollipop
            let fx = x + 10.0 + (h >> 8) as f32 % 12.0;
            draw_line(fx, y, fx, y - 14.0, 2.0, rgb(255, 255, 255));
            draw_circle(fx, y - 17.0, 6.0, [rgb(255, 110, 150), rgb(120, 200, 255), rgb(255, 200, 70)][(h >> 4) as usize % 3]);
            draw_circle(fx - 2.0, y - 19.0, 2.0, rgba(255, 255, 255, 200));
        }
        (Theme::Factory, 1) => {
            // little stack of cookies
            let fx = x + 16.0;
            for k in 0..3 {
                draw_ellipse(fx, y - 3.0 - k as f32 * 4.0, 9.0, 3.0, 0.0, if k % 2 == 0 { COOKIE_DARK } else { COOKIE });
            }
        }
        (Theme::Sky, 0 | 1) => {
            // sparkle
            let fx = x + 8.0 + (h >> 8) as f32 % 16.0;
            let s = 3.0 + ((g.t * 3.0 + h as f32).sin() * 0.5 + 0.5) * 2.0;
            let col = rgba(255, 236, 150, 230);
            draw_triangle(vec2(fx - s, y - 8.0), vec2(fx + s, y - 8.0), vec2(fx, y - 8.0 - s * 1.8), col);
            draw_triangle(vec2(fx - s, y - 8.0), vec2(fx + s, y - 8.0), vec2(fx, y - 8.0 + s * 1.8), col);
        }
        (Theme::Sky, 2) => {
            let fx = x + 10.0 + (h >> 8) as f32 % 12.0;
            draw_line(fx, y, fx, y - 8.0, 1.5, rgb(130, 190, 140));
            draw_circle(fx, y - 9.0, 2.6, rgb(255, 180, 220));
        }
        _ => {}
    }
}

fn draw_stone(theme: Theme, x: f32, y: f32) {
    let p = pal(theme);
    draw_rectangle(x, y, TILE, TILE, p.stone_dark);
    draw_rectangle(x + 2.0, y + 2.0, TILE - 4.0, TILE - 4.0, p.stone);
    draw_rectangle(x + 2.0, y + 2.0, TILE - 4.0, 4.0, p.stone_light);
    draw_rectangle(x + 2.0, y + TILE - 6.0, TILE - 4.0, 4.0, p.stone_mid);
    match theme {
        Theme::Factory => {
            for (ox, oy) in [(6.0, 9.0), (26.0, 9.0), (6.0, 22.0), (26.0, 22.0)] {
                draw_circle(x + ox, y + oy, 1.6, p.stone_dark);
                draw_circle(x + ox - 0.4, y + oy - 0.4, 0.8, p.stone_light);
            }
        }
        Theme::Sky | Theme::Storm => {
            draw_triangle(vec2(x + 6.0, y + 24.0), vec2(x + 12.0, y + 24.0), vec2(x + 22.0, y + 8.0), rgba(255, 255, 255, 120));
        }
        Theme::Meadow => draw_circle(x + 7.0, y + 10.0, 1.5, p.stone_light),
    }
}

/// Cookie floor with big cracks: a ground pound breaks it.
fn draw_cracked(x: f32, y: f32, tx: i32, ty: i32) {
    draw_cookie(x, y, 0.0, 1.0, tx, ty);
    let c = CHOC;
    let h = hash(tx, ty);
    let o = (h % 6) as f32;
    draw_line(x + 4.0, y + 8.0 + o, x + 14.0, y + 14.0, 2.0, c);
    draw_line(x + 14.0, y + 14.0, x + 12.0, y + 24.0, 2.0, c);
    draw_line(x + 14.0, y + 14.0, x + 24.0, y + 10.0, 2.0, c);
    draw_line(x + 24.0, y + 10.0, x + 29.0, y + 18.0 - o, 2.0, c);
    draw_line(x + 12.0, y + 24.0, x + 20.0, y + 29.0, 1.6, c);
}

/// Stone pedestal with a golden arrow: the super-jump spot.
fn draw_pedestal(theme: Theme, x: f32, y: f32, t: f32) {
    let p = pal(theme);
    draw_rectangle(x + 2.0, y + 4.0, TILE - 4.0, TILE - 4.0, p.stone_dark);
    draw_rectangle(x + 4.0, y + 6.0, TILE - 8.0, TILE - 8.0, p.stone);
    draw_rectangle(x, y, TILE, 7.0, p.stone_dark);
    draw_rectangle(x + 1.0, y + 1.0, TILE - 2.0, 4.0, p.stone_light);
    let bob = (t * 4.0).sin() * 1.5;
    let c = vec2(x + TILE / 2.0, y + 19.0 + bob);
    draw_triangle(c + vec2(-7.0, 2.0), c + vec2(7.0, 2.0), c + vec2(0.0, -7.0), GOLD);
    draw_rectangle(c.x - 2.5, c.y + 1.0, 5.0, 6.0, GOLD);
}

fn draw_turret(t: Option<&Turret>, x: f32, y: f32, dir: f32) {
    draw_rectangle(x, y, TILE, TILE, rgb(70, 60, 80));
    draw_rectangle(x + 2.0, y + 2.0, TILE - 4.0, TILE - 4.0, rgb(112, 96, 128));
    draw_rectangle(x + 2.0, y + 2.0, TILE - 4.0, 4.0, rgb(146, 128, 160));
    let mx = if dir < 0.0 { x + 3.0 } else { x + TILE - 3.0 };
    draw_circle(mx, y + 16.0, 9.0, rgb(60, 48, 66));
    draw_circle(mx, y + 16.0, 6.0, rgb(24, 16, 24));
    if let Some(t) = t {
        let charge = 1.0 - (t.timer / 0.8).clamp(0.0, 1.0);
        if charge > 0.0 {
            draw_circle(mx, y + 16.0, 5.0 * charge, rgba(255, 140, 40, 200));
        }
        if t.flash > 0.0 {
            draw_circle(mx + dir * 6.0, y + 16.0, 8.0, rgba(255, 230, 150, 220));
        }
    }
    let ex = x + 16.0 - dir * 5.0;
    draw_circle(ex - 4.0, y + 9.0, 2.6, WHITE);
    draw_circle(ex + 4.0, y + 9.0, 2.6, WHITE);
    draw_circle(ex - 4.0 + dir, y + 9.5, 1.3, BLACK);
    draw_circle(ex + 4.0 + dir, y + 9.5, 1.3, BLACK);
    draw_line(ex - 7.0, y + 5.0, ex - 2.0, y + 7.0, 1.5, BLACK);
    draw_line(ex + 7.0, y + 5.0, ex + 2.0, y + 7.0, 1.5, BLACK);
}

fn draw_oneway(g: &Game, tx: i32, ty: i32, x: f32, y: f32) {
    let p = pal(g.lv.theme);
    let edge_l = g.lv.tile(tx - 1, ty) != Tile::OneWay;
    let edge_r = g.lv.tile(tx + 1, ty) != Tile::OneWay;
    if matches!(g.lv.theme, Theme::Sky | Theme::Storm) {
        // fluffy cloud strip
        draw_rectangle(x, y + 2.0, TILE, 9.0, p.plank_dark);
        draw_rectangle(x, y, TILE, 8.0, p.plank);
        for i in 0..3 {
            draw_circle(x + 5.0 + i as f32 * 11.0, y + 4.0, 5.5, p.plank);
        }
        if edge_l {
            draw_circle(x + 2.0, y + 5.0, 6.0, p.plank);
        }
        if edge_r {
            draw_circle(x + TILE - 2.0, y + 5.0, 6.0, p.plank);
        }
        return;
    }
    draw_rectangle(x, y, TILE, 11.0, p.plank);
    draw_rectangle(x, y, TILE, 3.0, p.plank_light);
    draw_rectangle(x, y + 9.0, TILE, 2.0, p.plank_dark);
    if g.lv.theme == Theme::Factory {
        // wafer grid
        for k in 0..4 {
            draw_line(x + 4.0 + k as f32 * 8.0, y + 3.0, x + 4.0 + k as f32 * 8.0, y + 9.0, 1.0, p.plank_dark);
        }
    } else {
        draw_circle(x + 6.0, y + 6.0, 1.3, p.plank_dark);
        draw_circle(x + 26.0, y + 6.0, 1.3, p.plank_dark);
    }
    if edge_l {
        draw_rectangle(x, y, 2.0, 11.0, p.plank_dark);
    }
    if edge_r {
        draw_rectangle(x + TILE - 2.0, y, 2.0, 11.0, p.plank_dark);
    }
}

fn draw_cookie(x: f32, y: f32, crack: f32, alpha: f32, tx: i32, ty: i32) {
    draw_rectangle(x + 1.0, y + 1.0, TILE - 2.0, TILE - 2.0, with_a(COOKIE_DARK, alpha));
    draw_rectangle(x + 3.0, y + 3.0, TILE - 6.0, TILE - 6.0, with_a(COOKIE, alpha));
    let h = hash(tx, ty);
    for k in 0..4 {
        let cx = x + 7.0 + ((h >> (k * 5)) % 18) as f32;
        let cy = y + 7.0 + ((h >> (k * 5 + 2)) % 18) as f32;
        draw_circle(cx, cy, 2.0, with_a(CHOC, alpha));
    }
    if crack > 0.2 {
        let c = with_a(CHOC, alpha);
        draw_line(x + 16.0, y + 3.0, x + 13.0, y + 14.0, 1.5, c);
        draw_line(x + 13.0, y + 14.0, x + 19.0, y + 21.0, 1.5, c);
        if crack > 0.6 {
            draw_line(x + 19.0, y + 21.0, x + 15.0, y + 29.0, 1.5, c);
            draw_line(x + 13.0, y + 14.0, x + 4.0, y + 17.0, 1.5, c);
        }
    }
}

fn draw_spikes(x: f32, y: f32, up: bool) {
    let w = (TILE - 4.0) / 3.0;
    for i in 0..3 {
        let bx = x + 2.0 + i as f32 * w;
        let (a, b, c) = if up {
            (vec2(bx, y + TILE - 3.0), vec2(bx + w, y + TILE - 3.0), vec2(bx + w / 2.0, y + 12.0))
        } else {
            (vec2(bx, y + 3.0), vec2(bx + w, y + 3.0), vec2(bx + w / 2.0, y + TILE - 12.0))
        };
        draw_triangle(a, b, c, METAL);
        draw_triangle(a, vec2((a.x + b.x) / 2.0, a.y), c, rgb(250, 252, 255));
        draw_triangle_lines(a, b, c, 1.2, METAL_DARK);
    }
    if up {
        draw_rectangle(x, y + TILE - 4.0, TILE, 4.0, METAL_DARK);
    } else {
        draw_rectangle(x, y, TILE, 4.0, METAL_DARK);
    }
}

fn draw_spring(g: &Game, tx: i32, ty: i32, x: f32, y: f32) {
    let p = pal(g.lv.theme);
    let t = g.spring_anims.iter().find(|s| s.0 == (tx, ty)).map(|s| s.1);
    // squash right after launching, then wobble
    let k = t.map_or(0.0, |t| (t * 30.0).sin() * (-t * 7.0).exp());
    let wob = (g.t * 4.0 + tx as f32).sin() * 0.6;
    draw_rectangle(x + 1.0, y + 14.0, TILE - 2.0, TILE - 14.0, p.plank_dark);
    draw_rectangle(x + 3.0, y + 16.0, TILE - 6.0, TILE - 18.0, p.plank);
    draw_rectangle(x + 3.0, y + 16.0, TILE - 6.0, 3.0, p.plank_light);
    let top = y + 2.0 + k * 6.0 + wob;
    let h = y + 16.0 - top;
    draw_rectangle(x + 2.0, top + h * 0.45, TILE - 4.0, h * 0.55 + 1.0, JELLY_PINK);
    draw_ellipse(x + TILE / 2.0, top + h * 0.45, TILE / 2.0 - 2.0, h * 0.45, 0.0, JELLY_PINK);
    draw_ellipse(x + 10.0, top + h * 0.3, 4.5, 2.2, -15.0, JELLY_LIGHT);
    draw_circle(x + 21.0, top + h * 0.5, 1.6, JELLY_LIGHT);
}

fn draw_fan(g: &Game, x: f32, y: f32) {
    let p = pal(g.lv.theme);
    draw_rectangle(x, y, TILE, TILE, p.stone_dark);
    draw_rectangle(x + 2.0, y + 8.0, TILE - 4.0, TILE - 10.0, p.stone);
    for k in 0..3 {
        draw_rectangle(x + 5.0, y + 14.0 + k as f32 * 5.0, TILE - 10.0, 2.0, p.stone_mid);
    }
    // spinning blades seen from the side
    let a = g.t * 22.0;
    draw_rectangle(x + 1.0, y + 2.0, TILE - 2.0, 6.0, rgb(70, 76, 96));
    for k in 0..3 {
        let ph = a + k as f32 * 2.094;
        let bx = x + TILE / 2.0 + ph.cos() * 11.0;
        let w = 3.0 + ph.sin().abs() * 5.0;
        draw_ellipse(bx, y + 4.0, w, 2.2, 0.0, rgb(200, 230, 255));
    }
    draw_circle(x + TILE / 2.0, y + 4.0, 2.4, rgb(255, 214, 90));
}

fn draw_box(g: &Game, tx: i32, ty: i32, x: f32, y: f32, heart: bool) {
    let i = g.lv.idx(tx, ty).unwrap();
    let used = g.box_used[i];
    let bump = g.box_bumps.iter().find(|b| b.0 == i).map_or(0.0, |b| (b.1 / 0.25 * std::f32::consts::PI).sin() * 7.0);
    let y = y - bump;
    if used {
        draw_rectangle(x, y, TILE, TILE, rgb(150, 104, 64));
        draw_rectangle(x + 2.0, y + 2.0, TILE - 4.0, TILE - 4.0, rgb(184, 132, 84));
        for (ox, oy) in [(6.0, 6.0), (26.0, 6.0), (6.0, 26.0), (26.0, 26.0)] {
            draw_circle(x + ox, y + oy, 1.5, rgb(130, 88, 54));
        }
        return;
    }
    draw_rectangle(x, y, TILE, TILE, COOKIE_DARK);
    draw_rectangle(x + 2.0, y + 2.0, TILE - 4.0, TILE - 4.0, COOKIE);
    draw_rectangle(x + 2.0, y + 2.0, TILE - 4.0, 3.0, rgb(255, 228, 170));
    // biscuit holes
    for (ox, oy) in [(6.0, 6.0), (26.0, 6.0), (6.0, 26.0), (26.0, 26.0)] {
        draw_circle(x + ox, y + oy, 1.5, COOKIE_DARK);
    }
    let pulse = 1.0 + (g.t * 5.0).sin() * 0.08;
    let c = vec2(x + TILE / 2.0, y + TILE / 2.0 + 1.0);
    if heart {
        draw_heart(c, 8.0 * pulse, HEART, HEART_DARK);
    } else {
        draw_orange(c, 7.0 * pulse);
    }
}

fn draw_slide(g: &Game, tx: i32, ty: i32, x: f32, y: f32) {
    let p = pal(g.lv.theme);
    let right = g.lv.tile(tx, ty) == Tile::SlideR;
    // ground under the slope
    let (a, b, c) = if right {
        (vec2(x, y), vec2(x, y + TILE), vec2(x + TILE, y + TILE))
    } else {
        (vec2(x + TILE, y), vec2(x + TILE, y + TILE), vec2(x, y + TILE))
    };
    draw_triangle(a, b, c, p.fill);
    // the slide track along the diagonal
    let (s0, s1) = if right { (vec2(x, y), vec2(x + TILE, y + TILE)) } else { (vec2(x, y + TILE), vec2(x + TILE, y)) };
    let n = vec2(0.0, 7.0);
    draw_triangle(s0, s1, s1 + n, rgb(255, 160, 60));
    draw_triangle(s0, s1 + n, s0 + n, rgb(255, 160, 60));
    draw_line(s0.x, s0.y, s1.x, s1.y, 2.5, rgb(255, 236, 190));
    draw_line(s0.x, s0.y + 7.0, s1.x, s1.y + 7.0, 1.5, rgb(214, 104, 20));
    for k in 0..2 {
        let f = 0.25 + k as f32 * 0.5;
        let m = s0 + (s1 - s0) * f + vec2(0.0, 3.5);
        draw_circle(m.x, m.y, 1.6, rgb(255, 255, 255));
    }
}

fn draw_sign(g: &Game, x: f32, y: f32, n: u8) {
    let near = g.sign.is_some_and(|s| s.0 == n);
    let wood = rgb(198, 140, 80);
    let bob = if near { (g.t * 6.0).sin() * 1.0 } else { 0.0 };
    draw_rectangle(x + 14.0, y + 14.0, 4.0, 18.0, rgb(146, 98, 54));
    draw_rectangle(x + 3.0, y + 2.0 + bob, 26.0, 16.0, rgb(146, 98, 54));
    draw_rectangle(x + 4.5, y + 3.5 + bob, 23.0, 13.0, wood);
    draw_rectangle(x + 4.5, y + 3.5 + bob, 23.0, 2.5, rgb(232, 182, 116));
    // "!" mark
    let col = if near { rgb(255, 120, 40) } else { INK };
    draw_rectangle(x + 14.8, y + 5.5 + bob, 2.4, 6.0, col);
    draw_circle(x + 16.0, y + 14.0 + bob, 1.4, col);
}

pub fn draw_tiles(g: &Game) {
    let lv = &g.lv;
    let c0 = (g.cam.x / TILE).floor() as i32 - 1;
    let c1 = ((g.cam.x + g.view_w) / TILE).ceil() as i32 + 1;
    let r0 = (g.cam.y / TILE).floor() as i32 - 1;
    let r1 = ((g.cam.y + VIEW_H) / TILE).ceil() as i32 + 1;
    for ty in r0.max(0)..=r1.min(lv.h - 1) {
        for tx in c0.max(0)..=c1.min(lv.w - 1) {
            let (x, y) = (tx as f32 * TILE, ty as f32 * TILE);
            match lv.tile(tx, ty) {
                Tile::Ground => draw_ground(g, tx, ty, 1.0),
                Tile::Stone => draw_stone(lv.theme, x, y),
                Tile::OneWay => draw_oneway(g, tx, ty, x, y),
                Tile::SpikeUp => draw_spikes(x, y, true),
                Tile::SpikeDown => draw_spikes(x, y, false),
                Tile::TurretL | Tile::TurretR => {
                    let dir = if lv.tile(tx, ty) == Tile::TurretL { -1.0 } else { 1.0 };
                    let t = g.turrets.iter().find(|t| t.tx == tx && t.ty == ty);
                    draw_turret(t, x, y, dir);
                }
                Tile::Crumble => {
                    let i = lv.idx(tx, ty).unwrap();
                    match g.crumbles[i] {
                        Crumble::Solid => draw_cookie(x, y, 0.0, 1.0, tx, ty),
                        Crumble::Shaking(t) => {
                            let k = t / CRUMBLE_DELAY;
                            let j = (g.t * 90.0).sin() * (0.4 + k * 1.6);
                            draw_cookie(x + j, y + k * 1.5, k, 1.0, tx, ty);
                        }
                        Crumble::Gone(t) => {
                            let a = if t > CRUMBLE_RESPAWN - 0.4 { 0.6 } else { 0.18 };
                            draw_rectangle_lines(x + 3.0, y + 3.0, TILE - 6.0, TILE - 6.0, 1.5, with_a(COOKIE_DARK, a));
                        }
                    }
                }
                Tile::Spring => draw_spring(g, tx, ty, x, y),
                Tile::Fan => draw_fan(g, x, y),
                Tile::BoxOrange => draw_box(g, tx, ty, x, y, false),
                Tile::BoxHeart => draw_box(g, tx, ty, x, y, true),
                Tile::SlideR | Tile::SlideL => draw_slide(g, tx, ty, x, y),
                Tile::Sign(n) => draw_sign(g, x, y, n),
                Tile::Cracked => {
                    if !g.cracked_gone[lv.idx(tx, ty).unwrap()] {
                        draw_cracked(x, y, tx, ty);
                    }
                }
                Tile::Pedestal => draw_pedestal(lv.theme, x, y, g.t),
                Tile::Empty | Tile::Bumper | Tile::Fake => {}
            }
        }
    }
    // decorations on top of exposed ground
    for ty in r0.max(1)..=r1.min(lv.h - 1) {
        for tx in c0.max(0)..=c1.min(lv.w - 1) {
            if lv.tile(tx, ty) == Tile::Ground && lv.tile(tx, ty - 1) == Tile::Empty {
                draw_decor(g, tx, ty);
            }
        }
    }
}

/// Fake walls are drawn after the player so they hide the secret room until it is revealed.
fn draw_fake_tiles(g: &Game) {
    let lv = &g.lv;
    if lv.fake_groups == 0 {
        return;
    }
    let c0 = (g.cam.x / TILE).floor() as i32 - 1;
    let c1 = ((g.cam.x + g.view_w) / TILE).ceil() as i32 + 1;
    let r0 = (g.cam.y / TILE).floor() as i32 - 1;
    let r1 = ((g.cam.y + VIEW_H) / TILE).ceil() as i32 + 1;
    for ty in r0.max(0)..=r1.min(lv.h - 1) {
        for tx in c0.max(0)..=c1.min(lv.w - 1) {
            // fake ground, plus springs/spikes sitting in a secret room's floor (they'd give it away)
            let t = lv.tile(tx, ty);
            let owner = if t == Tile::Fake {
                Some((tx, ty))
            } else if matches!(t, Tile::Spring | Tile::SpikeUp) && lv.tile(tx, ty - 1) == Tile::Fake {
                Some((tx, ty - 1))
            } else {
                None
            };
            if let Some((ox, oy)) = owner {
                let grp = lv.fake_group[lv.idx(ox, oy).unwrap()] as usize;
                let a = 1.0 - g.reveal[grp] * 0.75;
                draw_ground(g, tx, ty, a);
            }
        }
    }
}

/// Rising air above fans.
fn draw_wind(g: &Game) {
    let lv = &g.lv;
    let c0 = ((g.cam.x / TILE).floor() as i32 - 1).max(0);
    let c1 = (((g.cam.x + g.view_w) / TILE).ceil() as i32 + 1).min(lv.w - 1);
    let r0 = ((g.cam.y / TILE).floor() as i32 - 1).max(0);
    let r1 = (((g.cam.y + VIEW_H) / TILE).ceil() as i32 + 1).min(lv.h - 1);
    for tx in c0..=c1 {
        for ty in r0..=r1 {
            if !lv.wind_at(tx, ty) {
                continue;
            }
            let (x, y) = (tx as f32 * TILE, ty as f32 * TILE);
            draw_rectangle(x, y, TILE, TILE, rgba(255, 255, 255, 28));
            for k in 0..2 {
                let h = hash(tx, k);
                let sx = x + 6.0 + ((h >> 3) % 20) as f32;
                let off = (g.t * 150.0 + (h % 64) as f32) % (TILE * 2.0);
                let sy = y + TILE * 2.0 - off - TILE / 2.0;
                if sy > y - 2.0 && sy < y + TILE {
                    draw_line(sx, sy, sx, sy + 9.0, 1.5, rgba(255, 255, 255, 140));
                }
            }
        }
    }
}

// ------------------------------------------------------------------------------------------ objects

pub fn draw_orange_rot(c: Vec2, r: f32, rot: f32) {
    draw_circle(c.x, c.y + 1.0, r + 1.2, ORANGE_DARK);
    draw_circle(c.x, c.y, r, ORANGE);
    draw_circle(c.x - r * 0.35, c.y - r * 0.35, r * 0.38, ORANGE_LIGHT);
    let (s, co) = rot.sin_cos();
    let up = vec2(s, -co);
    let stem = c + up * (r + 1.0);
    draw_line(c.x + up.x * (r - 1.0), c.y + up.y * (r - 1.0), stem.x, stem.y, 1.6, rgb(110, 80, 40));
    let side = vec2(co, s);
    let leaf = stem + side * 3.0;
    draw_ellipse(leaf.x, leaf.y, r * 0.5, r * 0.24, rot.to_degrees() - 25.0, LEAF);
}

pub fn draw_orange(c: Vec2, r: f32) {
    draw_orange_rot(c, r, 0.0);
}

pub fn draw_golden(c: Vec2, r: f32, t: f32) {
    let pulse = (t * 4.0).sin() * 0.5 + 0.5;
    draw_circle(c.x, c.y, r + 6.0 + pulse * 3.0, rgba(255, 230, 120, 70));
    for k in 0..4 {
        let a = t * 1.5 + k as f32 * std::f32::consts::FRAC_PI_2;
        let d = r + 7.0;
        draw_line(c.x + a.cos() * (r + 2.0), c.y + a.sin() * (r + 2.0), c.x + a.cos() * d, c.y + a.sin() * d, 2.0, rgba(255, 245, 190, 200));
    }
    draw_circle(c.x, c.y + 1.0, r + 1.5, rgb(214, 160, 20));
    draw_circle(c.x, c.y, r, GOLD);
    draw_circle(c.x - r * 0.35, c.y - r * 0.35, r * 0.4, GOLD_LIGHT);
    draw_rectangle(c.x - 1.0, c.y - r - 3.0, 2.0, 4.0, rgb(110, 80, 40));
    draw_ellipse(c.x + 4.0, c.y - r - 2.0, r * 0.5, r * 0.24, -25.0, LEAF);
}

pub fn draw_bubble(c: Vec2, r: f32, t: f32) {
    let wob = (t * 5.0).sin() * 0.8;
    draw_circle(c.x, c.y, r + wob, rgba(140, 205, 255, 90));
    draw_circle_lines(c.x, c.y, r + wob, 1.8, rgba(215, 240, 255, 230));
    draw_circle(c.x - r * 0.4, c.y - r * 0.4, r * 0.22, rgba(255, 255, 255, 220));
}

fn draw_items(g: &Game) {
    for it in &g.items {
        if it.taken {
            continue;
        }
        let bob = if it.falling { 0.0 } else { (g.t * 3.0 + it.pos.x * 0.05).sin() * 2.0 };
        let p = it.pos + vec2(0.0, bob);
        match it.kind {
            ItemKind::Orange => draw_orange(p, 7.0),
            ItemKind::Golden => draw_golden(p, 10.0, g.t),
            ItemKind::Shield => {
                draw_bubble(p, 11.0, g.t);
                draw_circle(p.x, p.y + 1.0, 4.5, rgba(255, 255, 255, 160));
            }
            ItemKind::Heart => {
                let pulse = 1.0 + (g.t * 6.0).sin() * 0.07;
                draw_circle(p.x, p.y, 15.0, rgba(255, 150, 170, 60));
                draw_heart(p, 10.0 * pulse, HEART, HEART_DARK);
            }
        }
    }
    for t in &g.thrown {
        let c = t.pos + vec2(THROWN_SIZE / 2.0, THROWN_SIZE / 2.0);
        draw_orange_rot(c, 6.0, t.spin);
    }
}

fn draw_checkpoints(g: &Game) {
    let p = pal(g.lv.theme);
    for cp in &g.checkpoints {
        let b = cp.pos;
        draw_rectangle(b.x - 9.0, b.y - 6.0, 18.0, 6.0, p.stone_dark);
        draw_rectangle(b.x - 7.0, b.y - 8.0, 14.0, 3.0, p.stone);
        draw_rectangle(b.x - 2.0, b.y - 60.0, 4.0, 54.0, rgb(240, 236, 228));
        let top = b.y - 58.0;
        let wave = (g.t * 6.0).sin() * 3.0;
        let col = if cp.active { ORANGE } else { rgb(200, 200, 212) };
        draw_triangle(vec2(b.x + 2.0, top), vec2(b.x + 26.0, top + 8.0 + wave), vec2(b.x + 2.0, top + 17.0), col);
        if cp.active {
            draw_circle(b.x + 10.0, top + 8.5, 3.2, ORANGE_LIGHT);
        }
        draw_circle(b.x, b.y - 62.0, 4.0, if cp.active { GOLD } else { rgb(170, 170, 180) });
    }
}

fn draw_goal(g: &Game) {
    // in the boss arena the gate rises out of the floor once the boss is beaten
    let sink = match &g.boss {
        Some(b) if !b.gone => return,
        Some(_) => (1.0 - (g.gate_t / 0.8).min(1.0)) * 140.0,
        None => 0.0,
    };
    draw_gate(g, sink);
}

fn draw_gate(g: &Game, sink: f32) {
    let (gx, gy) = g.goal_tile;
    let x0 = gx as f32 * TILE;
    let bottom = (gy + 1) as f32 * TILE + sink;
    let cream = rgb(250, 244, 230);
    let shimmer = (g.t * 3.0).sin() * 0.5 + 0.5;
    draw_rectangle(x0 + 20.0, bottom - 118.0, 56.0, 118.0, rgba(255, 240, 180, (60.0 + shimmer * 50.0) as u8));
    for px in [x0 + 4.0, x0 + 76.0] {
        draw_rectangle(px, bottom - 122.0, 16.0, 122.0, cream);
        for k in 0..4 {
            draw_rectangle(px, bottom - 110.0 + k as f32 * 28.0, 16.0, 9.0, ORANGE);
        }
        draw_rectangle(px - 2.0, bottom - 8.0, 20.0, 8.0, pal(g.lv.theme).stone_dark);
    }
    draw_rectangle(x0, bottom - 136.0, 96.0, 20.0, ORANGE_DARK);
    draw_rectangle(x0 + 2.0, bottom - 134.0, 92.0, 15.0, ORANGE);
    let params = TextParams { font: font(), font_size: 32, font_scale: 0.42, color: WHITE, ..Default::default() };
    let d = measure_text("终点", font(), 32, 0.42);
    draw_text_ex("终点", x0 + 48.0 - d.width / 2.0, bottom - 121.0, params);
    draw_orange(vec2(x0 + 48.0, bottom - 150.0), 11.0);
}

fn draw_plats(g: &Game) {
    for p in &g.plats {
        let r = p.rect;
        draw_ellipse(r.x + r.w / 2.0, r.y + r.h + 3.0, r.w * 0.4, 3.0, 0.0, rgba(120, 230, 255, 90));
        draw_rectangle(r.x, r.y, r.w, r.h, rgb(60, 120, 140));
        draw_rectangle(r.x + 1.5, r.y + 1.5, r.w - 3.0, r.h - 3.0, rgb(96, 172, 192));
        draw_rectangle(r.x + 1.5, r.y + 1.5, r.w - 3.0, 4.0, rgb(160, 222, 236));
        let n = (r.w / 16.0) as i32;
        for k in 0..n {
            let sx = r.x + 8.0 + k as f32 * 16.0;
            draw_circle(sx, r.y + 10.0, 1.6, rgb(50, 100, 120));
        }
        let c = vec2(r.x + r.w / 2.0, r.y + 10.0);
        let d = if p.vel.length_squared() > 0.0 { p.vel.normalize() } else { vec2(1.0, 0.0) };
        let n2 = vec2(-d.y, d.x);
        draw_triangle(c + d * 4.0, c - d * 3.0 + n2 * 3.5, c - d * 3.0 - n2 * 3.5, rgb(255, 214, 90));
    }
}

fn draw_enemy(e: &Enemy, t: f32) {
    let r = e.rect();
    let cx = r.x + r.w / 2.0;
    let bottom = r.y + r.h;
    let dir = e.dir;
    match e.kind {
        EKind::Jelly => {
            let col = rgba(178, 110, 236, 235);
            if e.dying > 0.0 {
                draw_ellipse(cx, bottom - 3.0, r.w * 0.7, 4.0, 0.0, col);
                return;
            }
            let wob = (t * 9.0 + r.x * 0.1).sin() * 0.08;
            let w = r.w * (1.0 + wob);
            let h = r.h * (1.0 - wob);
            draw_rectangle(cx - w / 2.0, bottom - h * 0.55, w, h * 0.55, col);
            draw_ellipse(cx, bottom - h * 0.55, w / 2.0, h * 0.5, 0.0, col);
            draw_ellipse(cx - w * 0.2, bottom - h * 0.75, 3.5, 2.0, -20.0, rgba(255, 255, 255, 170));
            for s in [-1.0, 1.0] {
                let ex = cx + s * 5.0 + dir * 2.0;
                draw_circle(ex, bottom - h * 0.5, 3.4, WHITE);
                draw_circle(ex + dir * 1.2, bottom - h * 0.48, 1.7, INK);
            }
        }
        EKind::Hedgehog => {
            let step = (t * 14.0 + r.x * 0.2).sin();
            draw_rectangle(cx - 8.0 + step * 1.5, bottom - 4.0, 5.0, 4.0, rgb(70, 44, 30));
            draw_rectangle(cx + 4.0 - step * 1.5, bottom - 4.0, 5.0, 4.0, rgb(70, 44, 30));
            for k in 0..6 {
                let a = std::f32::consts::PI * (0.08 + 0.84 * k as f32 / 5.0);
                let base = vec2(cx - a.cos() * 11.0 * -dir, bottom - 9.0 - a.sin() * 9.0);
                let out = vec2(-a.cos() * -dir, -a.sin());
                let side = vec2(-out.y, out.x) * 4.0;
                draw_triangle(base + side, base - side, base + out * 9.0, rgb(84, 58, 44));
                draw_circle(base.x + out.x * 8.0, base.y + out.y * 8.0, 1.1, rgb(240, 230, 220));
            }
            draw_ellipse(cx, bottom - 9.0, 13.0, 9.0, 0.0, rgb(140, 94, 62));
            draw_circle(cx + dir * 9.0, bottom - 8.0, 5.5, rgb(232, 196, 152));
            draw_circle(cx + dir * 14.0, bottom - 9.0, 1.8, INK);
            draw_circle(cx + dir * 6.0, bottom - 12.0, 1.6, INK);
            draw_circle(cx + dir * 4.0, bottom - 7.0, 1.6, rgba(255, 140, 150, 200));
        }
        EKind::Frog => {
            let col = rgb(96, 196, 96);
            if e.dying > 0.0 {
                draw_ellipse(cx, bottom - 3.0, r.w * 0.7, 4.0, 0.0, col);
                return;
            }
            let (sx, sy) = if e.on_ground {
                let charge = (e.timer / 1.5).clamp(0.0, 1.0);
                (1.0 + charge * 0.12, 1.0 - charge * 0.12)
            } else {
                (0.85, 1.18)
            };
            let w = r.w * sx;
            let h = r.h * sy;
            draw_ellipse(cx, bottom - h * 0.42, w / 2.0, h * 0.42, 0.0, col);
            draw_ellipse(cx, bottom - h * 0.3, w * 0.32, h * 0.22, 0.0, rgb(200, 236, 160));
            for s in [-1.0, 1.0] {
                let ex = cx + s * 6.0;
                let ey = bottom - h * 0.82;
                draw_circle(ex, ey, 5.0, col);
                draw_circle(ex, ey, 3.4, WHITE);
                draw_circle(ex + dir * 1.0, ey, 1.8, INK);
            }
            draw_line(cx - 4.0, bottom - h * 0.48, cx + 4.0, bottom - h * 0.48, 1.2, rgb(40, 100, 40));
        }
        EKind::Cloudlet => {
            let col = rgb(150, 156, 190);
            if e.dying > 0.0 {
                draw_ellipse(cx, bottom - 3.0, r.w * 0.7, 4.0, 0.0, col);
                return;
            }
            let cy = r.y + r.h / 2.0;
            draw_circle(cx - 8.0, cy + 2.0, 7.0, col);
            draw_circle(cx + 8.0, cy + 2.0, 7.0, col);
            draw_circle(cx, cy - 3.0, 9.0, col);
            draw_circle(cx - 3.0, cy - 6.0, 4.0, rgb(196, 200, 228));
            for s in [-1.0, 1.0] {
                let ex = cx + s * 5.0 + dir * 2.0;
                draw_circle(ex, cy, 2.8, WHITE);
                draw_circle(ex + dir, cy + 0.5, 1.4, INK);
                draw_line(ex - 3.0 * s - 1.0, cy - 5.0 + s * dir, ex + 3.0 * s - 1.0, cy - 3.5, 1.4, INK);
            }
            // drizzle
            let k = (t * 6.0 + r.x) % 1.0;
            draw_line(cx - 4.0, bottom + k * 8.0, cx - 5.0, bottom + 3.0 + k * 8.0, 1.2, rgba(150, 200, 255, 200));
        }
        EKind::Seed => {
            for k in 1..4 {
                let lx = cx - dir * (8.0 + k as f32 * 5.0);
                draw_line(lx, r.y + 2.0 + k as f32 * 2.0, lx - dir * 5.0, r.y + 2.0 + k as f32 * 2.0, 1.2, rgba(255, 255, 255, 120));
            }
            draw_ellipse(cx, r.y + r.h / 2.0, r.w / 2.0 + 1.0, r.h / 2.0 + 1.0, 0.0, rgb(50, 30, 20));
            draw_ellipse(cx, r.y + r.h / 2.0, r.w / 2.0, r.h / 2.0, 0.0, rgb(120, 76, 46));
            draw_circle(cx + dir * 2.0, r.y + 3.0, 1.8, rgba(255, 230, 200, 200));
        }
    }
}

/// The orange on Lulu's cap turns into a leaf propeller while gliding.
fn draw_propeller(top: Vec2, a: f32, alpha: f32) {
    let rod = rgba(110, 80, 40, (alpha * 255.0) as u8);
    draw_line(top.x, top.y + 4.0, top.x, top.y - 3.0, 2.0, rod);
    for k in 0..2 {
        let ph = a + k as f32 * std::f32::consts::PI;
        let reach = ph.cos() * 15.0;
        let depth = ph.sin();
        let col = if depth > 0.0 { rgb(96, 190, 72) } else { rgb(70, 150, 56) };
        draw_ellipse(top.x + reach / 2.0, top.y - 3.0, reach.abs() / 2.0 + 1.5, 3.2, 0.0, with_a(col, alpha));
    }
    draw_circle(top.x, top.y - 3.0, 2.6, with_a(ORANGE, alpha));
}

fn draw_player(g: &Game, a: &Assets) {
    let aspect = a.lulu.width() / a.lulu.height();
    if !g.alive {
        let w = SPRITE_H * aspect;
        let p = g.death_pos + vec2(PW / 2.0, PH / 2.0);
        draw_texture_ex(
            &a.lulu,
            p.x - w / 2.0,
            p.y - SPRITE_H / 2.0,
            WHITE,
            DrawTextureParams { dest_size: Some(vec2(w, SPRITE_H)), rotation: g.death_t * 9.0, ..Default::default() },
        );
        return;
    }
    let b = &g.body;
    let an = &g.anim;
    let blink = g.invuln > 0.0 && ((g.t * 16.0) as i32) % 2 == 0;
    let color = if blink { Color::new(1.0, 1.0, 1.0, 0.35) } else { WHITE };
    let feet = vec2(b.pos.x + PW / 2.0, b.pos.y + PH + 1.0 - an.bob);
    let breathe = if b.on_ground && b.vel.x.abs() < 5.0 { (g.t * 3.0).sin() * 0.015 } else { 0.0 };
    let h = SPRITE_H * (an.scale.y + breathe);
    let w = SPRITE_H * aspect * (an.scale.x - breathe * 0.5);
    if b.on_ground {
        draw_ellipse(b.pos.x + PW / 2.0, b.pos.y + PH, 13.0, 3.0, 0.0, rgba(0, 0, 0, 45));
    }
    // super jump charge: a ring that fills up, then a golden glow
    if b.crouch || b.super_grace > 0.0 {
        let c = feet - vec2(0.0, h * 0.5);
        if b.super_ready() {
            let pulse = (g.t * 14.0).sin() * 0.5 + 0.5;
            draw_circle(c.x, c.y, 26.0 + pulse * 4.0, rgba(255, 220, 90, (50.0 + pulse * 50.0) as u8));
        } else {
            let k = (b.charge / SUPER_CHARGE).min(1.0);
            let n = (k * 12.0) as i32;
            for i in 0..n {
                let ang = -std::f32::consts::FRAC_PI_2 + i as f32 / 12.0 * std::f32::consts::TAU;
                draw_circle(c.x + ang.cos() * 24.0, c.y + ang.sin() * 24.0, 2.4, rgba(255, 214, 80, 230));
            }
        }
    }
    if b.pound == Pound::Slam {
        for k in 0..3 {
            let lx = feet.x - 9.0 + k as f32 * 9.0;
            let top = feet.y - h - 26.0 - (k % 2) as f32 * 8.0;
            draw_line(lx, top, lx, top + 18.0, 2.0, rgba(255, 255, 255, 170));
        }
    }
    let (rot, pivot) = if an.spin != 0.0 { (an.spin, feet - vec2(0.0, h * 0.5)) } else { (an.tilt, feet) };
    draw_texture_ex(
        &a.lulu,
        feet.x - w / 2.0,
        feet.y - h,
        color,
        DrawTextureParams { dest_size: Some(vec2(w, h)), rotation: rot, pivot: Some(pivot), flip_x: b.facing < 0.0, ..Default::default() },
    );
    draw_whip(g);
    if an.prop_speed > 1.0 && an.spin == 0.0 {
        let (s, c) = an.tilt.sin_cos();
        let up = vec2(s, -c);
        let top = feet + up * (h - 3.0);
        draw_propeller(top, an.prop, color.a.max(0.6));
    }
    if g.shield {
        let c = b.center() + vec2(0.0, -4.0);
        draw_bubble(c, 30.0, g.t);
    }
}

/// Lulu's pink tie, stretched out as a whip.
fn draw_whip(g: &Game) {
    let Some(w) = &g.whip else { return };
    let e = w.extension();
    if e <= 0.02 {
        return;
    }
    let b = &g.body;
    let c = b.center();
    let start = vec2(c.x + w.dir * 3.0, c.y + 1.0);
    let len = 6.0 + w.reach() * e;
    let arc = match w.stage {
        1 => 9.0,
        2 => -9.0,
        _ => 0.0,
    };
    let n = 12;
    let pts: Vec<Vec2> = (0..=n)
        .map(|i| {
            let f = i as f32 / n as f32;
            let wave = (f * 9.0 - g.t * 40.0).sin() * 3.0 * (1.0 - e) * f;
            vec2(start.x + w.dir * len * f, start.y + arc * (f * std::f32::consts::PI).sin() * (1.0 - e * 0.5) + wave)
        })
        .collect();
    // swoosh trail in front
    if w.t < 0.2 {
        let a0 = if w.stage == 2 { 0.7 } else { -0.7 };
        for k in 0..7 {
            let f = k as f32 / 6.0;
            let ang = a0 * (1.0 - f) * (1.0 - e);
            let p = c + vec2(w.dir * (len + 6.0) * ang.cos(), (len + 6.0) * ang.sin() * 0.6);
            draw_circle(p.x, p.y, 3.0 + f * 2.0, rgba(255, 255, 255, (40.0 + f * 90.0) as u8));
        }
    }
    for (width, col) in [(7.0, TIE_DARK), (4.5, TIE)] {
        for i in 0..n {
            let taper = 1.0 - 0.35 * i as f32 / n as f32;
            draw_line(pts[i].x, pts[i].y, pts[i + 1].x, pts[i + 1].y, width * taper, col);
        }
    }
    for i in (0..n).step_by(3) {
        draw_line(pts[i].x, pts[i].y - 1.0, pts[i + 1].x, pts[i + 1].y - 1.0, 1.2, TIE_LIGHT);
    }
    // the tie's pointy end
    let tip = pts[n];
    let d = w.dir;
    draw_triangle(tip + vec2(d * 10.0, 0.0), tip + vec2(-d * 1.0, -6.5), tip + vec2(-d * 1.0, 6.5), TIE_DARK);
    draw_triangle(tip + vec2(d * 8.0, 0.0), tip + vec2(0.0, -4.5), tip + vec2(0.0, 4.5), TIE);
    if w.stage >= 3 {
        let s = 4.0 + (g.t * 30.0).sin() * 1.5;
        let p = tip + vec2(d * 12.0, 0.0);
        draw_triangle(p - vec2(s, 0.0), p + vec2(s, 0.0), p - vec2(0.0, s * 1.8), GOLD_LIGHT);
        draw_triangle(p - vec2(s, 0.0), p + vec2(s, 0.0), p + vec2(0.0, s * 1.8), GOLD_LIGHT);
    }
}

fn draw_shockwaves(g: &Game) {
    for (c, age) in &g.shockwaves {
        let k = age / 0.4;
        let r = 10.0 + POUND_RADIUS * k;
        draw_ellipse_lines(c.x, c.y - 3.0, r, r * 0.22, 0.0, 4.0 * (1.0 - k) + 1.0, rgba(255, 255, 255, ((1.0 - k) * 220.0) as u8));
    }
}

fn draw_particles(g: &Game) {
    for p in &g.particles {
        let k = (p.life / p.max).clamp(0.0, 1.0);
        draw_circle(p.pos.x, p.pos.y, p.size * k.sqrt(), with_a(p.color, k));
    }
}

/// Draw the whole world (background, tiles, objects, player) from the game's camera.
pub fn draw_world(g: &Game, a: &Assets) {
    let level_h = g.lv.h as f32 * TILE;
    if g.lv.theme == Theme::Storm {
        let clear = g.boss.as_ref().map_or(0.0, |b| if b.gone { (g.gate_t / 2.5).min(1.0) } else { 0.0 });
        bg_storm(g.cam, g.view_w, level_h, g.t, clear);
    } else {
        draw_background(g.lv.theme, g.cam, g.view_w, level_h, g.t);
    }
    let mut cam = g.cam;
    if g.shake > 0.0 {
        cam += vec2((g.t * 97.0).sin(), (g.t * 131.0).cos()) * g.shake * 12.0;
    }
    set_camera(&world_cam(cam.x, cam.y, g.view_w));
    draw_wind(g);
    boss::draw_bolt_warnings(g);
    draw_goal(g);
    draw_checkpoints(g);
    draw_tiles(g);
    draw_plats(g);
    draw_items(g);
    for e in &g.enemies {
        if e.ko {
            let c = e.pos + e.size / 2.0;
            with_rotation(cam, g.view_w, c, e.spin, || draw_enemy(e, g.t));
        } else if e.active || e.kind != EKind::Seed {
            draw_enemy(e, g.t);
        }
    }
    boss::draw_boss_world(g);
    draw_player(g, a);
    draw_fake_tiles(g);
    draw_shockwaves(g);
    boss::draw_bolts(g);
    draw_particles(g);
    if g.screen_flash > 0.0 {
        set_default_camera();
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(1.0, 1.0, 0.95, g.screen_flash / 0.12 * 0.45));
    }
}

// ------------------------------------------------------------------------------------------ UI

pub fn ui_scale() -> f32 {
    screen_height() / VIEW_H
}

pub fn text(s: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_text_ex(s, x + size * 0.06, y + size * 0.06, tp(size, Color::new(0.0, 0.0, 0.0, 0.45 * color.a)));
    draw_text_ex(s, x, y, tp(size, color));
}

pub fn text_center(s: &str, cx: f32, y: f32, size: f32, color: Color) {
    let d = measure(s, size);
    text(s, cx - d.width / 2.0, y, size, color);
}

fn text_right(s: &str, rx: f32, y: f32, size: f32, color: Color) {
    let d = measure(s, size);
    text(s, rx - d.width, y, size, color);
}

pub fn text_outline_center(s: &str, cx: f32, y: f32, size: f32, color: Color, outline: Color) {
    let d = measure(s, size);
    let x = cx - d.width / 2.0;
    let o = (size * 0.06).max(1.5);
    draw_text_ex(s, x + o * 1.6, y + o * 1.9, tp(size, Color::new(0.0, 0.0, 0.0, 0.35 * color.a)));
    for (dx, dy) in [(-o, 0.0), (o, 0.0), (0.0, -o), (0.0, o), (-o, -o), (o, o), (-o, o), (o, -o)] {
        draw_text_ex(s, x + dx, y + dy, tp(size, outline));
    }
    draw_text_ex(s, x, y, tp(size, color));
}

pub fn fmt_time(t: f32) -> String {
    let m = (t / 60.0) as u32;
    let s = t - m as f32 * 60.0;
    format!("{m}:{s:05.2}")
}

/// Rounded rectangle as one triangle fan, so translucent colours have no overlapping seams.
fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    let r = r.min(w / 2.0).min(h / 2.0);
    let mut vertices = vec![Vertex::new(x + w / 2.0, y + h / 2.0, 0.0, 0.0, 0.0, color)];
    let corners = [(x + w - r, y + r, -90.0f32), (x + w - r, y + h - r, 0.0), (x + r, y + h - r, 90.0), (x + r, y + r, 180.0)];
    for (cx, cy, a0) in corners {
        for k in 0..=6 {
            let a = (a0 + k as f32 * 15.0).to_radians();
            vertices.push(Vertex::new(cx + a.cos() * r, cy + a.sin() * r, 0.0, 0.0, 0.0, color));
        }
    }
    let n = vertices.len() as u16 - 1;
    let mut indices = vec![];
    for i in 1..=n {
        indices.extend_from_slice(&[0, i, if i == n { 1 } else { i + 1 }]);
    }
    draw_mesh(&Mesh { vertices, indices, texture: None });
}

/// Restrict drawing to a screen rectangle (logical pixels), or lift the restriction.
fn clip(r: Option<(f32, f32, f32, f32)>) {
    let s = screen_dpi_scale();
    let gl = unsafe { get_internal_gl() }.quad_gl;
    gl.scissor(r.map(|(x, y, w, h)| ((x * s) as i32, (y * s) as i32, (w * s).ceil() as i32, (h * s).ceil() as i32)));
}

fn draw_golden_icon(c: Vec2, r: f32) {
    draw_circle(c.x, c.y, r + 1.0, rgb(214, 160, 20));
    draw_circle(c.x, c.y, r, GOLD);
    draw_circle(c.x - r * 0.35, c.y - r * 0.35, r * 0.4, GOLD_LIGHT);
}

fn golden_slots(c: Vec2, r: f32, gap: f32, mask_or_count: impl Fn(u32) -> bool) {
    for k in 0..3u32 {
        let p = c + vec2(k as f32 * gap, 0.0);
        if mask_or_count(k) {
            draw_golden_icon(p, r);
        } else {
            draw_circle_lines(p.x, p.y, r, r * 0.22, rgba(255, 230, 150, 170));
        }
    }
}

/// Speech bubble above a sign (screen space).
fn draw_sign_bubble(g: &Game, n: u8, world: Vec2) {
    let msg = g.lv.sign_text(n);
    if msg.is_empty() {
        return;
    }
    let u = ui_scale();
    let lines: Vec<&str> = msg.split('|').map(|s| s.trim()).collect();
    let size = 12.0 * u;
    let w = lines.iter().map(|l| measure(l, size).width).fold(0.0, f32::max) + 20.0 * u;
    let h = lines.len() as f32 * 17.0 * u + 10.0 * u;
    let sx = ((world.x - g.cam.x) * u).clamp(w / 2.0 + 6.0 * u, screen_width() - w / 2.0 - 6.0 * u);
    let sy = (world.y - g.cam.y) * u - 22.0 * u;
    let (x, y) = (sx - w / 2.0, sy - h);
    rounded_rect(x + 2.0 * u, y + 3.0 * u, w, h, 7.0 * u, rgba(0, 0, 0, 60));
    rounded_rect(x, y, w, h, 7.0 * u, rgba(255, 251, 238, 245));
    let tip = vec2(((world.x - g.cam.x) * u).clamp(x + 12.0 * u, x + w - 12.0 * u), sy + 8.0 * u);
    draw_triangle(vec2(tip.x - 7.0 * u, sy - 1.0), vec2(tip.x + 7.0 * u, sy - 1.0), tip, rgba(255, 251, 238, 245));
    for (i, l) in lines.iter().enumerate() {
        let d = measure(l, size);
        draw_text_ex(l, sx - d.width / 2.0, y + (18.0 + i as f32 * 17.0) * u, tp(size, INK));
    }
}

/// Comic speech bubble with a thick outline above a world position (screen space).
fn speech_bubble(msg: &str, world: Vec2, g: &Game, size: f32, scale: f32, alpha: f32) {
    let u = ui_scale();
    let d = measure(msg, size);
    let (w, h) = ((d.width + 22.0 * u) * scale, (size + 14.0 * u) * scale);
    let sx = ((world.x - g.cam.x) * u).clamp(w / 2.0 + 6.0 * u, screen_width() - w / 2.0 - 6.0 * u);
    let sy = (world.y - g.cam.y) * u - 14.0 * u;
    let (x, y) = (sx - w / 2.0, sy - h);
    let ink = with_a(INK, alpha);
    let paper = with_a(rgb(255, 253, 244), alpha);
    let tipx = ((world.x - g.cam.x) * u).clamp(x + 14.0 * u, x + w - 14.0 * u);
    let o = 2.5 * u;
    rounded_rect(x - o, y - o, w + 2.0 * o, h + 2.0 * o, 10.0 * u, ink);
    draw_triangle(vec2(tipx - 8.0 * u, sy - 2.0), vec2(tipx + 4.0 * u, sy - 2.0), vec2(tipx - 6.0 * u, sy + 11.0 * u), ink);
    rounded_rect(x, y, w, h, 8.0 * u, paper);
    draw_triangle(vec2(tipx - 5.5 * u, sy - 3.0 * u), vec2(tipx + 1.5 * u, sy - 3.0 * u), vec2(tipx - 5.0 * u, sy + 6.5 * u), paper);
    if scale > 0.6 {
        let fs = size * scale;
        let d = measure(msg, fs);
        draw_text_ex(msg, sx - d.width / 2.0, y + h / 2.0 + fs * 0.36, tp(fs, with_a(rgb(214, 70, 120), alpha)));
    }
}

pub fn draw_hud(g: &Game) {
    set_default_camera();
    let u = ui_scale();
    let panel = rgba(30, 20, 40, 110);
    rounded_rect(8.0 * u, 8.0 * u, 214.0 * u, 26.0 * u, 6.0 * u, panel);
    for k in 0..MAX_HEARTS {
        let c = vec2((22.0 + k as f32 * 19.0) * u, 21.0 * u);
        if k < g.hearts {
            draw_heart(c, 7.0 * u, HEART, HEART_DARK);
        } else {
            draw_heart(c, 7.0 * u, rgba(90, 70, 90, 200), rgba(60, 40, 60, 200));
        }
    }
    draw_orange(vec2(88.0 * u, 21.0 * u), 6.5 * u);
    text(&format!("x{}", g.oranges), 98.0 * u, 27.0 * u, 15.0 * u, WHITE);
    if !g.lv.boss {
        golden_slots(vec2(170.0 * u, 21.0 * u), 6.5 * u, 18.0 * u, |k| g.golden_mask & (1 << k) != 0);
    }
    if g.shield {
        draw_bubble(vec2(236.0 * u, 21.0 * u), 9.0 * u, g.t);
    }

    let sw = screen_width();
    let s = format!("{}   {}", g.lv.name, fmt_time(g.time));
    let d = measure(&s, 14.0 * u);
    rounded_rect(sw - d.width - 26.0 * u, 8.0 * u, d.width + 18.0 * u, 26.0 * u, 6.0 * u, panel);
    text(&s, sw - 17.0 * u - d.width, 26.5 * u, 14.0 * u, WHITE);
    boss::draw_boss_hud(g);

    if let Some((n, pos)) = g.sign {
        if g.alive && !g.cleared {
            draw_sign_bubble(g, n, pos);
        }
    }
    // "LuLu can fly!!!" when the propeller starts
    if g.bubble_t > 0.0 && g.alive && !g.cleared {
        let age = BUBBLE_TIME - g.bubble_t;
        let pop = if age < 0.18 {
            let k = age / 0.18;
            1.0 + (1.0 - k) * (k * 9.0).sin() * 0.25 - (1.0 - k).powi(3)
        } else {
            1.0
        };
        let alpha = (g.bubble_t / 0.3).min(1.0);
        let head = vec2(g.body.center().x, g.body.pos.y - 16.0);
        speech_bubble("LuLu can fly！！！", head, g, 14.0 * u, pop.max(0.05), alpha);
    }
    // combo counter
    if g.combo >= 2 && g.combo_pop.1 < 1.1 {
        let age = g.combo_pop.1;
        let at = g.combo_pop.0;
        let pop = 1.0 + (0.15 - age).max(0.0) * 3.0;
        let a = (1.0 - (age - 0.7).max(0.0) / 0.4).clamp(0.0, 1.0);
        let x = (at.x - g.cam.x) * u;
        let y = (at.y - g.cam.y - 26.0 - age * 24.0) * u;
        let col = Color::new(1.0, 0.9, 0.35, a);
        text_outline_center(&format!("连击 x{}!", g.combo), x, y, 17.0 * u * pop, col, Color::new(0.75, 0.2, 0.35, a));
    }

    if let Some((msg, t)) = g.toast {
        let a = (t / 0.4).min(1.0);
        let rise = (1.6 - t).min(0.25) * 40.0;
        text_outline_center(msg, sw / 2.0, (90.0 - rise) * u, 26.0 * u, Color::new(1.0, 0.95, 0.7, a), Color::new(0.75, 0.35, 0.05, a));
    }
}

pub fn dim(a: f32) {
    set_default_camera();
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.08, 0.05, 0.12, a));
}

/// Title backdrop: slowly scrolling sky + hills and a flat meadow to stand on.
pub fn draw_title_scene(t: f32, view_w: f32) {
    let level_h = TITLE_LEVEL_H;
    let cam = vec2(t * 25.0, level_h - VIEW_H);
    draw_background(Theme::Meadow, cam, view_w, level_h, t);
    set_camera(&world_cam(0.0, cam.y, view_w));
    let p = &MEADOW;
    let top = TITLE_GROUND_Y;
    draw_rectangle(-10.0, top, view_w + 20.0, 400.0, p.fill);
    let cols = (view_w / TILE) as i32 + 2;
    for c in 0..cols {
        let x = c as f32 * TILE;
        for k in 0..6 {
            let h = hash(c, k);
            draw_circle(x + (h % 30) as f32, top + 20.0 + ((h >> 8) % 70) as f32, 2.4, p.speck);
        }
        if hash(c, 99) % 3 == 0 {
            let h = hash(c, 7);
            let fx = x + (h % 24) as f32;
            draw_line(fx, top, fx, top - 9.0, 1.5, p.top_dark);
            draw_circle(fx, top - 10.0, 2.6, [rgb(255, 120, 150), rgb(255, 220, 90), WHITE][(h >> 4) as usize % 3]);
        }
    }
    draw_rectangle(-10.0, top + 9.0, view_w + 20.0, 4.0, p.fill_dark);
    draw_rectangle(-10.0, top, view_w + 20.0, 9.0, p.top);
    for i in 0..(view_w / 8.0) as i32 + 2 {
        draw_circle(i as f32 * 8.0, top + 9.0, 4.5, p.top);
    }
    draw_rectangle(-10.0, top, view_w + 20.0, 3.0, p.top_light);
}

const TITLE_LEVEL_H: f32 = 20.0 * TILE;
const TITLE_GROUND_Y: f32 = 17.0 * TILE;

pub fn draw_title(a: &Assets, t: f32, blink: bool) {
    set_default_camera();
    let u = ui_scale();
    let sw = screen_width();
    let hop = ((t * 2.2).sin().abs()) * 14.0 * u;
    let squash = 1.0 + ((t * 4.4).cos()) * 0.025;
    let h = 230.0 * u * squash;
    let w = h * a.lulu_title.width() / a.lulu_title.height() / squash / squash;
    let cx = sw / 2.0 + 150.0 * u;
    let base = (TITLE_GROUND_Y - (TITLE_LEVEL_H - VIEW_H)) * u + 4.0 * u;
    draw_ellipse(cx, base, 60.0 * u, 9.0 * u, 0.0, rgba(0, 0, 0, 50));
    draw_texture_ex(&a.lulu_title, cx - w / 2.0, base - h - hop, WHITE, DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() });

    let tx = sw / 2.0 - 120.0 * u;
    text_outline_center("SUPER", tx, 84.0 * u, 52.0 * u, rgb(255, 246, 220), rgb(214, 104, 20));
    text_outline_center("LULU", tx, 142.0 * u, 72.0 * u, rgb(255, 196, 60), rgb(150, 70, 20));
    text_center("~ 甜甜的小冒险 ~", tx, 168.0 * u, 13.0 * u, rgb(255, 255, 255));
    if blink {
        text_outline_center("按 Enter 开始", tx, 202.0 * u, 20.0 * u, WHITE, rgb(60, 110, 200));
    }
    text("Esc 退出", 10.0 * u, VIEW_H * u - 10.0 * u, 11.0 * u, rgba(255, 255, 255, 200));
    let rows = [
        ("A  D", "移动"),
        ("W", "跳（按越久跳越高）"),
        ("空中再按住 W", "滑翔"),
        ("按住 S 再按 W", "超级跳"),
        ("Shift", "加速跑"),
        ("J  /  K  /  L", "领带鞭 / 屁股墩 / 扔橘子"),
    ];
    rounded_rect(tx - 152.0 * u, 214.0 * u, 304.0 * u, 128.0 * u, 8.0 * u, rgba(30, 20, 40, 120));
    for (i, (k, v)) in rows.iter().enumerate() {
        let y = (232.0 + i as f32 * 16.5) * u;
        text_right(k, tx - 22.0 * u, y, 12.5 * u, rgb(255, 214, 110));
        text(v, tx - 10.0 * u, y, 12.5 * u, rgb(255, 250, 235));
    }
    text_center("R 重来   Esc 暂停   M 静音   F11 全屏", tx, 335.0 * u, 11.0 * u, rgba(255, 250, 235, 220));
}

/// Little theme picture on a level card.
fn draw_card_art(theme: Theme, x: f32, y: f32, w: f32, h: f32, u: f32, t: f32) {
    match theme {
        Theme::Meadow => {
            gradient_rect(x, y, w, h, rgb(96, 170, 245), rgb(196, 232, 255));
            cloud(x + w * 0.15, y + h * 0.3, 0.35 * u, WHITE);
            draw_circle(x + w * 0.3, y + h * 1.25, h * 0.75, rgb(146, 206, 150));
            draw_circle(x + w * 0.85, y + h * 1.3, h * 0.7, rgb(118, 202, 82));
            draw_rectangle(x + w * 0.62, y + h * 0.48, 4.0 * u, 14.0 * u, rgb(150, 110, 80));
            draw_circle(x + w * 0.62 + 2.0 * u, y + h * 0.45, 10.0 * u, rgb(110, 180, 100));
            draw_circle(x + w * 0.62 - 2.0 * u, y + h * 0.42, 2.4 * u, ORANGE);
            draw_circle(x + w * 0.62 + 6.0 * u, y + h * 0.47, 2.4 * u, ORANGE);
        }
        Theme::Factory => {
            gradient_rect(x, y, w, h, rgb(255, 226, 194), rgb(236, 176, 132));
            draw_gear(x + w * 0.25, y + h * 0.55, h * 0.28, t * 0.6, rgb(220, 150, 108), rgb(245, 200, 160));
            draw_rectangle(x, y + h * 0.78, w, h * 0.22, rgb(206, 148, 90));
            draw_rectangle(x, y + h * 0.74, w, h * 0.07, rgb(255, 172, 204));
            draw_circle(x + w * 0.7, y + h * 0.48, h * 0.2, COOKIE);
            for (ox, oy) in [(-0.05, -0.05), (0.04, 0.04), (0.06, -0.08)] {
                draw_circle(x + w * (0.7 + ox), y + h * (0.48 + oy), 2.2 * u, CHOC);
            }
        }
        Theme::Sky => {
            gradient_rect(x, y, w, h, rgb(110, 172, 252), rgb(255, 222, 238));
            draw_circle(x + w * 0.8, y + h * 0.25, h * 0.14, rgb(255, 246, 200));
            cloud(x + w * 0.05, y + h * 0.7, 0.5 * u, WHITE);
            cloud(x + w * 0.55, y + h * 0.82, 0.6 * u, WHITE);
            cloud(x + w * 0.35, y + h * 0.35, 0.3 * u, rgba(255, 255, 255, 200));
        }
        Theme::Storm => {
            gradient_rect(x, y, w, h, rgb(40, 42, 80), rgb(104, 88, 132));
            let c = vec2(x + w * 0.5, y + h * 0.42);
            boss::draw_storm_face(c, 0.62 * u, t, 0.0, false);
            let bx = x + w * 0.22;
            let pts = [vec2(bx, y + h * 0.55), vec2(bx - 6.0 * u, y + h * 0.75), vec2(bx + 3.0 * u, y + h * 0.75), vec2(bx - 4.0 * u, y + h * 0.98)];
            for k in 0..3 {
                draw_line(pts[k].x, pts[k].y, pts[k + 1].x, pts[k + 1].y, 3.0 * u, GOLD);
            }
        }
    }
}

/// Level select: one card per level, locked until the previous one is cleared.
pub fn draw_level_select(names: &[(String, Theme)], progress: &Progress, sel: usize, t: f32) {
    set_default_camera();
    let u = ui_scale();
    let sw = screen_width();
    text_outline_center("选择关卡", sw / 2.0, 60.0 * u, 32.0 * u, rgb(255, 246, 220), rgb(214, 104, 20));
    let n = names.len() as f32;
    let (cw, ch, gap) = if names.len() > 3 { (136.0 * u, 190.0 * u, 12.0 * u) } else { (150.0 * u, 190.0 * u, 18.0 * u) };
    let x0 = sw / 2.0 - (n * cw + (n - 1.0) * gap) / 2.0;
    for (i, (name, theme)) in names.iter().enumerate() {
        let unlocked = progress.unlocked(i);
        let save = &progress.levels[i];
        let chosen = i == sel;
        let lift = if chosen { 6.0 * u + (t * 4.0).sin() * 2.0 * u } else { 0.0 };
        let x = x0 + i as f32 * (cw + gap);
        let y = 90.0 * u - lift;
        if chosen {
            rounded_rect(x - 4.0 * u, y - 4.0 * u, cw + 8.0 * u, ch + 8.0 * u, 12.0 * u, rgb(255, 196, 60));
        }
        rounded_rect(x + 3.0 * u, y + 5.0 * u, cw, ch, 10.0 * u, rgba(0, 0, 0, 60));
        rounded_rect(x, y, cw, ch, 10.0 * u, rgb(255, 250, 238));
        let (ax, ay, aw, ah) = (x + 8.0 * u, y + 8.0 * u, cw - 16.0 * u, 92.0 * u);
        clip(Some((ax, ay, aw, ah)));
        draw_card_art(*theme, ax, ay, aw, ah, u, t);
        clip(None);
        let boss_card = *theme == Theme::Storm;
        let label = if boss_card { "BOSS".to_string() } else { format!("{}", i + 1) };
        text_center(&label, x + cw / 2.0, ay + ah + 26.0 * u, 24.0 * u, if boss_card { rgb(150, 70, 190) } else { ORANGE_DARK });
        text_center(name, x + cw / 2.0, ay + ah + 46.0 * u, 13.0 * u, INK);
        if unlocked {
            if !boss_card {
                golden_slots(vec2(x + cw / 2.0 - 18.0 * u, ay + ah + 62.0 * u), 6.0 * u, 18.0 * u, |k| save.goldens & (1 << k) != 0);
            } else if save.cleared {
                text_center("已击败！", x + cw / 2.0, ay + ah + 66.0 * u, 12.0 * u, rgb(150, 70, 190));
            }
            let best = save.best_time.map(|b| format!("最佳 {}", fmt_time(b))).unwrap_or_else(|| "新关卡！".into());
            text_center(&best, x + cw / 2.0, ay + ah + 84.0 * u, 12.0 * u, rgb(120, 90, 70));
        } else {
            rounded_rect(x, y, cw, ch, 10.0 * u, rgba(40, 30, 50, 150));
            // padlock
            let c = vec2(x + cw / 2.0, y + 66.0 * u);
            draw_circle_lines(c.x, c.y - 10.0 * u, 10.0 * u, 4.0 * u, rgb(230, 230, 240));
            rounded_rect(c.x - 15.0 * u, c.y - 6.0 * u, 30.0 * u, 24.0 * u, 4.0 * u, rgb(255, 214, 90));
            draw_circle(c.x, c.y + 4.0 * u, 3.5 * u, rgb(150, 100, 40));
            text_center("先通过上一关", x + cw / 2.0, ay + ah + 84.0 * u, 11.0 * u, rgb(240, 236, 250));
        }
    }
    if ((t * 2.0) as i32) % 2 == 0 {
        text_center("A D 选择      Enter 开始      Esc 返回", sw / 2.0, 318.0 * u, 14.0 * u, WHITE);
    }
}

pub fn draw_pause() {
    dim(0.55);
    let u = ui_scale();
    let cx = screen_width() / 2.0;
    text_outline_center("暂停", cx, 150.0 * u, 44.0 * u, WHITE, rgb(60, 110, 200));
    text_center("Esc 继续      R 从存档点重来      Q 回到选关", cx, 200.0 * u, 15.0 * u, WHITE);
}

pub fn rank(g: &Game) -> &'static str {
    if g.lv.boss {
        return match g.deaths {
            0 => "S",
            1 => "A",
            2..=3 => "B",
            _ => "C",
        };
    }
    match g.deaths {
        0 if g.goldens >= 3 => "S",
        0 | 1 => "A",
        2..=4 => "B",
        _ => "C",
    }
}

pub fn draw_clear(g: &Game, new_best: bool, has_next: bool) {
    let k = ((g.clear_t - 1.0) / 0.5).clamp(0.0, 1.0);
    if k <= 0.0 {
        return;
    }
    dim(0.5 * k);
    let u = ui_scale();
    let cx = screen_width() / 2.0;
    let a = k;
    let title = if g.lv.boss { "打败了雷雨云大王！" } else { "过关啦！" };
    text_outline_center(title, cx, 86.0 * u, 40.0 * u, Color::new(1.0, 0.85, 0.3, a), Color::new(0.6, 0.25, 0.05, a));
    let mut rows = vec![("用时", fmt_time(g.time))];
    if !g.lv.boss {
        rows.push(("橘子", format!("{}/{}", g.collected, g.orange_total)));
        rows.push(("金橘子", format!("{}/3", g.goldens)));
    }
    rows.push(("丢命", format!("{}", g.deaths)));
    let white = Color::new(1.0, 1.0, 1.0, a);
    for (i, (k, v)) in rows.iter().enumerate() {
        let y = (132.0 + i as f32 * 23.0) * u;
        text(k, cx - 100.0 * u, y, 17.0 * u, Color::new(1.0, 0.85, 0.5, a));
        text_right(v, cx + 100.0 * u, y, 17.0 * u, white);
    }
    if new_best {
        let wob = (g.t * 6.0).sin() * 2.0 * u;
        text_outline_center("新纪录！", cx + 158.0 * u, 132.0 * u + wob, 16.0 * u, Color::new(1.0, 0.95, 0.6, a), Color::new(0.85, 0.2, 0.3, a));
    }
    text_outline_center(&format!("评级  {}", rank(g)), cx, 244.0 * u, 32.0 * u, Color::new(1.0, 0.95, 0.7, a), Color::new(0.75, 0.35, 0.05, a));
    if g.clear_t > 2.0 && ((g.clear_t * 2.0) as i32) % 2 == 0 {
        let msg = if has_next { "Enter 下一关      Esc 回到选关" } else { "Enter 回到选关      R 再玩一次" };
        text_center(msg, cx, 296.0 * u, 15.0 * u, WHITE);
    }
}
