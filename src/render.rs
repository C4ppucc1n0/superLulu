//! All drawing: procedural world art (soft, rounded cartoon style) + the Lulu sprite.

use macroquad::prelude::*;

use crate::game::*;
use crate::level::Tile;
use crate::physics::*;

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

const SKY_TOP: Color = rgb(96, 170, 245);
const SKY_BOTTOM: Color = rgb(196, 232, 255);
const DIRT: Color = rgb(214, 146, 86);
const DIRT_DARK: Color = rgb(181, 113, 62);
const DIRT_SPECK: Color = rgb(194, 126, 70);
const GRASS: Color = rgb(118, 202, 82);
const GRASS_LIGHT: Color = rgb(168, 230, 118);
const GRASS_DARK: Color = rgb(84, 160, 62);
const STONE: Color = rgb(156, 166, 196);
const STONE_LIGHT: Color = rgb(192, 201, 226);
const STONE_MID: Color = rgb(128, 137, 168);
const STONE_DARK: Color = rgb(96, 104, 134);
const WOOD: Color = rgb(198, 140, 80);
const WOOD_LIGHT: Color = rgb(232, 182, 116);
const WOOD_DARK: Color = rgb(146, 98, 54);
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

// ------------------------------------------------------------------------------------------ background

pub fn draw_background(cam: Vec2, view_w: f32, level_h: f32, t: f32) {
    set_default_camera();
    gradient_rect(0.0, 0.0, screen_width(), screen_height(), SKY_TOP, SKY_BOTTOM);

    let max_y = level_h - VIEW_H;
    // (x parallax, y parallax, colour, baseline, radius, spacing, seed)
    let layers = [
        (0.12, 0.25, rgba(255, 255, 255, 235), 0.0, 0.0, 260.0, 7),
        (0.25, 0.4, rgb(170, 214, 236), 560.0, 120.0, 190.0, 1),
        (0.45, 0.6, rgb(146, 206, 150), 590.0, 80.0, 150.0, 2),
    ];
    for (i, &(fx, fy, color, base, radius, spacing, seed)) in layers.iter().enumerate() {
        let lx = cam.x * fx;
        let ly = max_y + (cam.y - max_y) * fy;
        set_camera(&world_cam(lx, ly, view_w));
        let first = (lx / spacing).floor() as i32 - 2;
        let last = ((lx + view_w) / spacing).ceil() as i32 + 2;
        if i == 0 {
            // clouds
            for k in first..=last {
                let h = hash(k, seed);
                let cx = k as f32 * spacing + (h % 120) as f32 + (t * 6.0) % spacing;
                let cy = 120.0 + ((h >> 8) % 140) as f32;
                let s = 0.7 + ((h >> 16) % 50) as f32 / 100.0;
                draw_circle(cx, cy, 18.0 * s, color);
                draw_circle(cx + 20.0 * s, cy - 8.0 * s, 22.0 * s, color);
                draw_circle(cx + 44.0 * s, cy, 17.0 * s, color);
                draw_rectangle(cx, cy, 44.0 * s, 17.0 * s, color);
            }
            continue;
        }
        for k in first..=last {
            let h = hash(k, seed);
            let r = radius + (h % 50) as f32;
            let cx = k as f32 * spacing + ((h >> 8) % 60) as f32;
            draw_circle(cx, base + r * 0.25, r, color);
            if i == 2 && h % 3 == 0 {
                // little tangerine trees on the near hills
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

// ------------------------------------------------------------------------------------------ tiles

fn solidish(t: Tile) -> bool {
    matches!(t, Tile::Ground | Tile::Stone | Tile::TurretL | Tile::TurretR)
}

fn draw_ground(g: &Game, tx: i32, ty: i32, x: f32, y: f32) {
    let lv = &g.lv;
    let up = lv.tile(tx, ty - 1);
    let open = |t: Tile| t != Tile::Ground;
    draw_rectangle(x, y, TILE, TILE, DIRT);
    let h = hash(tx, ty);
    for k in 0..3 {
        let hx = ((h >> (k * 6)) % 24) as f32 + 4.0;
        let hy = ((h >> (k * 6 + 3)) % 18) as f32 + 10.0;
        draw_circle(x + hx, y + hy, 2.2 + (k as f32) * 0.4, DIRT_SPECK);
    }
    if open(lv.tile(tx - 1, ty)) {
        draw_rectangle(x, y, 3.0, TILE, DIRT_DARK);
    }
    if open(lv.tile(tx + 1, ty)) {
        draw_rectangle(x + TILE - 3.0, y, 3.0, TILE, DIRT_DARK);
    }
    if ty + 1 < lv.h && open(lv.tile(tx, ty + 1)) {
        draw_rectangle(x, y + TILE - 4.0, TILE, 4.0, DIRT_DARK);
    }
    if !solidish(up) {
        draw_rectangle(x, y + 9.0, TILE, 4.0, DIRT_DARK);
        draw_rectangle(x, y, TILE, 9.0, GRASS);
        for i in 0..4 {
            draw_circle(x + 4.0 + i as f32 * 8.0, y + 9.0, 4.5, GRASS);
        }
        draw_rectangle(x, y, TILE, 3.0, GRASS_LIGHT);
        if open(lv.tile(tx - 1, ty)) {
            draw_circle(x + 1.0, y + 6.0, 5.5, GRASS_DARK);
            draw_circle(x + 2.5, y + 5.0, 5.0, GRASS);
        }
        if open(lv.tile(tx + 1, ty)) {
            draw_circle(x + TILE - 1.0, y + 6.0, 5.5, GRASS_DARK);
            draw_circle(x + TILE - 2.5, y + 5.0, 5.0, GRASS);
        }
    }
}

fn draw_decor(g: &Game, tx: i32, ty: i32, x: f32, y: f32) {
    if g.lv.tile(tx, ty - 1) != Tile::Empty {
        return;
    }
    let h = hash(tx, ty + 1000);
    match h % 9 {
        0 => {
            // flower
            let fx = x + 8.0 + (h >> 8) as f32 % 16.0;
            draw_line(fx, y, fx, y - 9.0, 1.5, GRASS_DARK);
            let col = [rgb(255, 120, 150), rgb(255, 220, 90), rgb(255, 255, 255), rgb(190, 140, 255)][(h >> 4) as usize % 4];
            for k in 0..5 {
                let a = k as f32 * 1.2566;
                draw_circle(fx + a.cos() * 3.0, y - 10.0 + a.sin() * 3.0, 2.2, col);
            }
            draw_circle(fx, y - 10.0, 1.6, rgb(255, 190, 60));
        }
        1 | 2 => {
            // grass tuft
            let fx = x + 6.0 + (h >> 8) as f32 % 20.0;
            for k in -1i32..=1 {
                let kx = k as f32 * 3.0;
                draw_triangle(vec2(fx + kx - 2.0, y + 1.0), vec2(fx + kx + 2.0, y + 1.0), vec2(fx + kx * 1.6, y - 7.0 + k.abs() as f32 * 2.0), GRASS_DARK);
            }
        }
        3 => {
            // round bush
            let fx = x + 16.0;
            draw_circle(fx - 7.0, y - 3.0, 7.0, GRASS_DARK);
            draw_circle(fx + 6.0, y - 3.0, 8.0, GRASS_DARK);
            draw_circle(fx, y - 7.0, 8.0, rgb(100, 182, 70));
            draw_circle(fx - 2.0, y - 10.0, 3.0, GRASS_LIGHT);
        }
        _ => {}
    }
}

fn draw_stone(x: f32, y: f32) {
    draw_rectangle(x, y, TILE, TILE, STONE_DARK);
    draw_rectangle(x + 2.0, y + 2.0, TILE - 4.0, TILE - 4.0, STONE);
    draw_rectangle(x + 2.0, y + 2.0, TILE - 4.0, 4.0, STONE_LIGHT);
    draw_rectangle(x + 2.0, y + TILE - 6.0, TILE - 4.0, 4.0, STONE_MID);
    draw_circle(x + 7.0, y + 10.0, 1.5, STONE_LIGHT);
}

fn draw_turret(t: Option<&Turret>, x: f32, y: f32, dir: f32) {
    draw_rectangle(x, y, TILE, TILE, rgb(70, 60, 80));
    draw_rectangle(x + 2.0, y + 2.0, TILE - 4.0, TILE - 4.0, rgb(112, 96, 128));
    draw_rectangle(x + 2.0, y + 2.0, TILE - 4.0, 4.0, rgb(146, 128, 160));
    let mx = if dir < 0.0 { x + 3.0 } else { x + TILE - 3.0 };
    draw_circle(mx, y + 16.0, 9.0, rgb(60, 48, 66));
    draw_circle(mx, y + 16.0, 6.0, rgb(24, 16, 24));
    if let Some(t) = t {
        let charge = 1.0 - (t.timer / 0.6).clamp(0.0, 1.0);
        if charge > 0.0 {
            draw_circle(mx, y + 16.0, 5.0 * charge, rgba(255, 140, 40, 200));
        }
        if t.flash > 0.0 {
            draw_circle(mx + dir * 6.0, y + 16.0, 8.0, rgba(255, 230, 150, 220));
        }
    }
    // angry little eyes
    let ex = x + 16.0 - dir * 5.0;
    draw_circle(ex - 4.0, y + 9.0, 2.6, WHITE);
    draw_circle(ex + 4.0, y + 9.0, 2.6, WHITE);
    draw_circle(ex - 4.0 + dir, y + 9.5, 1.3, BLACK);
    draw_circle(ex + 4.0 + dir, y + 9.5, 1.3, BLACK);
    draw_line(ex - 7.0, y + 5.0, ex - 2.0, y + 7.0, 1.5, BLACK);
    draw_line(ex + 7.0, y + 5.0, ex + 2.0, y + 7.0, 1.5, BLACK);
}

fn draw_oneway(g: &Game, tx: i32, ty: i32, x: f32, y: f32) {
    draw_rectangle(x, y, TILE, 11.0, WOOD);
    draw_rectangle(x, y, TILE, 3.0, WOOD_LIGHT);
    draw_rectangle(x, y + 9.0, TILE, 2.0, WOOD_DARK);
    draw_circle(x + 6.0, y + 6.0, 1.3, WOOD_DARK);
    draw_circle(x + 26.0, y + 6.0, 1.3, WOOD_DARK);
    if g.lv.tile(tx - 1, ty) != Tile::OneWay {
        draw_rectangle(x, y, 2.0, 11.0, WOOD_DARK);
    }
    if g.lv.tile(tx + 1, ty) != Tile::OneWay {
        draw_rectangle(x + TILE - 2.0, y, 2.0, 11.0, WOOD_DARK);
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

pub fn draw_tiles(g: &Game) {
    let lv = &g.lv;
    let c0 = (g.cam.x / TILE).floor() as i32 - 1;
    let c1 = ((g.cam.x + g.view_w) / TILE).ceil() as i32 + 1;
    let r0 = (g.cam.y / TILE).floor() as i32 - 1;
    let r1 = ((g.cam.y + VIEW_H) / TILE).ceil() as i32 + 1;
    for ty in r0..=r1.min(lv.h - 1) {
        for tx in c0.max(0)..=c1.min(lv.w - 1) {
            let (x, y) = (tx as f32 * TILE, ty as f32 * TILE);
            match lv.tile(tx, ty) {
                Tile::Ground => draw_ground(g, tx, ty, x, y),
                Tile::Stone => draw_stone(x, y),
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
                            let j = (g.t * 90.0).sin() * (0.6 + k * 1.6);
                            draw_cookie(x + j, y + k * 1.5, k, 1.0, tx, ty);
                        }
                        Crumble::Gone(t) => {
                            let a = if t > CRUMBLE_RESPAWN - 0.4 { 0.6 } else { 0.18 };
                            draw_rectangle_lines(x + 3.0, y + 3.0, TILE - 6.0, TILE - 6.0, 1.5, with_a(COOKIE_DARK, a));
                        }
                    }
                }
                Tile::Empty | Tile::Bumper => {}
            }
        }
    }
    // decorations on top of grass
    for ty in r0.max(1)..=r1.min(lv.h - 1) {
        for tx in c0.max(0)..=c1.min(lv.w - 1) {
            if lv.tile(tx, ty) == Tile::Ground && lv.tile(tx, ty - 1) == Tile::Empty {
                draw_decor(g, tx, ty, tx as f32 * TILE, ty as f32 * TILE);
            }
        }
    }
}

// ------------------------------------------------------------------------------------------ objects

pub fn draw_orange(c: Vec2, r: f32) {
    draw_circle(c.x, c.y + 1.0, r + 1.2, ORANGE_DARK);
    draw_circle(c.x, c.y, r, ORANGE);
    draw_circle(c.x - r * 0.35, c.y - r * 0.35, r * 0.38, ORANGE_LIGHT);
    draw_rectangle(c.x - 0.8, c.y - r - 2.5, 1.6, 3.5, rgb(110, 80, 40));
    draw_ellipse(c.x + 3.0, c.y - r - 1.5, r * 0.5, r * 0.24, -25.0, LEAF);
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
        let bob = (g.t * 3.0 + it.pos.x * 0.05).sin() * 2.0;
        let p = it.pos + vec2(0.0, bob);
        match it.kind {
            ItemKind::Orange => draw_orange(p, 7.0),
            ItemKind::Golden => draw_golden(p, 10.0, g.t),
            ItemKind::Shield => {
                draw_bubble(p, 11.0, g.t);
                draw_circle(p.x, p.y + 1.0, 4.5, rgba(255, 255, 255, 160));
            }
        }
    }
}

fn draw_checkpoints(g: &Game) {
    for cp in &g.checkpoints {
        let b = cp.pos;
        draw_rectangle(b.x - 9.0, b.y - 6.0, 18.0, 6.0, STONE_DARK);
        draw_rectangle(b.x - 7.0, b.y - 8.0, 14.0, 3.0, STONE);
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
    let (gx, gy) = g.goal_tile;
    let x0 = gx as f32 * TILE;
    let bottom = (gy + 1) as f32 * TILE;
    let cream = rgb(250, 244, 230);
    // shimmering doorway
    let shimmer = (g.t * 3.0).sin() * 0.5 + 0.5;
    draw_rectangle(x0 + 20.0, bottom - 118.0, 56.0, 118.0, rgba(255, 240, 180, (60.0 + shimmer * 50.0) as u8));
    for (px, _) in [(x0 + 4.0, 0), (x0 + 76.0, 1)] {
        draw_rectangle(px, bottom - 122.0, 16.0, 122.0, cream);
        for k in 0..4 {
            draw_rectangle(px, bottom - 110.0 + k as f32 * 28.0, 16.0, 9.0, ORANGE);
        }
        draw_rectangle(px - 2.0, bottom - 8.0, 20.0, 8.0, STONE_DARK);
    }
    draw_rectangle(x0, bottom - 136.0, 96.0, 20.0, ORANGE_DARK);
    draw_rectangle(x0 + 2.0, bottom - 134.0, 92.0, 15.0, ORANGE);
    let params = TextParams { font_size: 64, font_scale: 0.22, color: WHITE, ..Default::default() };
    let d = measure_text("GOAL", None, 64, 0.22);
    draw_text_ex("GOAL", x0 + 48.0 - d.width / 2.0, bottom - 121.0, params);
    draw_orange(vec2(x0 + 48.0, bottom - 150.0), 11.0);
}

fn draw_plats(g: &Game) {
    for p in &g.plats {
        let r = p.rect;
        // little hover glow
        draw_ellipse(r.x + r.w / 2.0, r.y + r.h + 3.0, r.w * 0.4, 3.0, 0.0, rgba(120, 230, 255, 90));
        draw_rectangle(r.x, r.y, r.w, r.h, rgb(60, 120, 140));
        draw_rectangle(r.x + 1.5, r.y + 1.5, r.w - 3.0, r.h - 3.0, rgb(96, 172, 192));
        draw_rectangle(r.x + 1.5, r.y + 1.5, r.w - 3.0, 4.0, rgb(160, 222, 236));
        let n = (r.w / 16.0) as i32;
        for k in 0..n {
            let sx = r.x + 8.0 + k as f32 * 16.0;
            draw_circle(sx, r.y + 10.0, 1.6, rgb(50, 100, 120));
        }
        // direction arrow
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
            // feet
            draw_rectangle(cx - 8.0 + step * 1.5, bottom - 4.0, 5.0, 4.0, rgb(70, 44, 30));
            draw_rectangle(cx + 4.0 - step * 1.5, bottom - 4.0, 5.0, 4.0, rgb(70, 44, 30));
            // spikes along the back
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
                let charge = (e.timer / 1.25).clamp(0.0, 1.0);
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
    // soft shadow
    if b.on_ground {
        draw_ellipse(b.pos.x + PW / 2.0, b.pos.y + PH, 13.0, 3.0, 0.0, rgba(0, 0, 0, 45));
    }
    draw_texture_ex(
        &a.lulu,
        feet.x - w / 2.0,
        feet.y - h,
        color,
        DrawTextureParams { dest_size: Some(vec2(w, h)), rotation: an.tilt, pivot: Some(feet), flip_x: b.facing < 0.0, ..Default::default() },
    );
    if g.shield {
        let c = b.center() + vec2(0.0, -4.0);
        draw_bubble(c, 30.0, g.t);
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
    draw_background(g.cam, g.view_w, level_h, g.t);
    let mut cam = g.cam;
    if g.shake > 0.0 {
        cam += vec2((g.t * 97.0).sin(), (g.t * 131.0).cos()) * g.shake * 12.0;
    }
    set_camera(&world_cam(cam.x, cam.y, g.view_w));
    draw_goal(g);
    draw_checkpoints(g);
    draw_tiles(g);
    draw_plats(g);
    draw_items(g);
    for e in &g.enemies {
        if e.active || e.kind != EKind::Seed {
            draw_enemy(e, g.t);
        }
    }
    draw_player(g, a);
    draw_particles(g);
}

// ------------------------------------------------------------------------------------------ UI

pub fn ui_scale() -> f32 {
    screen_height() / VIEW_H
}

pub fn text(s: &str, x: f32, y: f32, size: f32, color: Color) {
    let shadow = TextParams { font_size: size as u16, color: Color::new(0.0, 0.0, 0.0, 0.45 * color.a), ..Default::default() };
    draw_text_ex(s, x + size * 0.07, y + size * 0.07, shadow);
    draw_text_ex(s, x, y, TextParams { font_size: size as u16, color, ..Default::default() });
}

pub fn text_center(s: &str, cx: f32, y: f32, size: f32, color: Color) {
    let d = measure_text(s, None, size as u16, 1.0);
    text(s, cx - d.width / 2.0, y, size, color);
}

pub fn text_outline_center(s: &str, cx: f32, y: f32, size: f32, color: Color, outline: Color) {
    let d = measure_text(s, None, size as u16, 1.0);
    let x = cx - d.width / 2.0;
    let o = (size * 0.06).max(1.5);
    let p = |c: Color| TextParams { font_size: size as u16, color: c, ..Default::default() };
    draw_text_ex(s, x + o * 1.6, y + o * 1.9, p(Color::new(0.0, 0.0, 0.0, 0.35)));
    for (dx, dy) in [(-o, 0.0), (o, 0.0), (0.0, -o), (0.0, o), (-o, -o), (o, o), (-o, o), (o, -o)] {
        draw_text_ex(s, x + dx, y + dy, p(outline));
    }
    draw_text_ex(s, x, y, p(color));
}

pub fn fmt_time(t: f32) -> String {
    let m = (t / 60.0) as u32;
    let s = t - m as f32 * 60.0;
    format!("{m}:{s:05.2}")
}

pub fn draw_hud(g: &Game) {
    set_default_camera();
    let u = ui_scale();
    let panel = rgba(30, 20, 40, 110);
    draw_rectangle(8.0 * u, 8.0 * u, 150.0 * u, 26.0 * u, panel);
    draw_orange(vec2(21.0 * u, 21.0 * u), 6.5 * u);
    text(&format!("{}/{}", g.oranges, g.orange_total), 32.0 * u, 26.0 * u, 15.0 * u, WHITE);
    for k in 0..3 {
        let c = vec2((100.0 + k as f32 * 18.0) * u, 21.0 * u);
        if (k as u32) < g.goldens {
            draw_golden_icon(c, 6.5 * u);
        } else {
            draw_circle_lines(c.x, c.y, 6.5 * u, 1.5 * u, rgba(255, 230, 150, 160));
        }
    }
    if g.shield {
        draw_bubble(vec2(170.0 * u, 21.0 * u), 9.0 * u, g.t);
    }

    let sw = screen_width();
    draw_rectangle(sw - 168.0 * u, 8.0 * u, 160.0 * u, 26.0 * u, panel);
    let s = format!("DEATHS {:<3}  {}", g.deaths, fmt_time(g.time));
    let d = measure_text(&s, None, (14.0 * u) as u16, 1.0);
    text(&s, sw - 16.0 * u - d.width, 26.0 * u, 14.0 * u, WHITE);

    if let Some((msg, t)) = g.toast {
        let a = (t / 0.4).min(1.0);
        let rise = (1.6 - t).min(0.25) * 40.0;
        text_outline_center(msg, sw / 2.0, (90.0 - rise) * u, 26.0 * u, Color::new(1.0, 0.95, 0.7, a), Color::new(0.75, 0.35, 0.05, a));
    }
}

fn draw_golden_icon(c: Vec2, r: f32) {
    draw_circle(c.x, c.y, r + 1.0, rgb(214, 160, 20));
    draw_circle(c.x, c.y, r, GOLD);
    draw_circle(c.x - r * 0.35, c.y - r * 0.35, r * 0.4, GOLD_LIGHT);
}

pub fn dim(a: f32) {
    set_default_camera();
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.08, 0.05, 0.12, a));
}

/// Title backdrop: slowly scrolling sky + hills and a flat meadow to stand on.
pub fn draw_title_scene(t: f32, view_w: f32, level_h: f32) {
    let cam = vec2(t * 25.0, level_h - VIEW_H);
    draw_background(cam, view_w, level_h, t);
    set_camera(&world_cam(0.0, cam.y, view_w));
    let top = TITLE_GROUND_Y;
    draw_rectangle(-10.0, top, view_w + 20.0, 400.0, DIRT);
    let cols = (view_w / TILE) as i32 + 2;
    for c in 0..cols {
        let x = c as f32 * TILE;
        for k in 0..6 {
            let h = hash(c, k);
            draw_circle(x + (h % 30) as f32, top + 20.0 + ((h >> 8) % 70) as f32, 2.4, DIRT_SPECK);
        }
        if hash(c, 99) % 3 == 0 {
            let h = hash(c, 7);
            let fx = x + (h % 24) as f32;
            draw_line(fx, top, fx, top - 9.0, 1.5, GRASS_DARK);
            draw_circle(fx, top - 10.0, 2.6, [rgb(255, 120, 150), rgb(255, 220, 90), WHITE][(h >> 4) as usize % 3]);
        }
    }
    draw_rectangle(-10.0, top + 9.0, view_w + 20.0, 4.0, DIRT_DARK);
    draw_rectangle(-10.0, top, view_w + 20.0, 9.0, GRASS);
    for i in 0..(view_w / 8.0) as i32 + 2 {
        draw_circle(i as f32 * 8.0, top + 9.0, 4.5, GRASS);
    }
    draw_rectangle(-10.0, top, view_w + 20.0, 3.0, GRASS_LIGHT);
}

const TITLE_GROUND_Y: f32 = 17.0 * TILE;

pub fn draw_title(a: &Assets, t: f32, blink: bool, level_h: f32) {
    set_default_camera();
    let u = ui_scale();
    let sw = screen_width();
    // big Lulu, bouncing on the meadow
    let hop = ((t * 2.2).sin().abs()) * 14.0 * u;
    let squash = 1.0 + ((t * 4.4).cos()) * 0.025;
    let h = 230.0 * u * squash;
    let w = h * a.lulu_title.width() / a.lulu_title.height() / squash / squash;
    let cx = sw / 2.0 + 150.0 * u;
    let base = (TITLE_GROUND_Y - (level_h - VIEW_H)) * u + 4.0 * u;
    draw_ellipse(cx, base, 60.0 * u, 9.0 * u, 0.0, rgba(0, 0, 0, 50));
    draw_texture_ex(&a.lulu_title, cx - w / 2.0, base - h - hop, WHITE, DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() });

    let tx = sw / 2.0 - 120.0 * u;
    text_outline_center("SUPER", tx, 92.0 * u, 52.0 * u, rgb(255, 246, 220), rgb(214, 104, 20));
    text_outline_center("LULU", tx, 150.0 * u, 72.0 * u, rgb(255, 196, 60), rgb(150, 70, 20));
    text_center("~ a tough little adventure ~", tx, 176.0 * u, 13.0 * u, rgb(255, 255, 255));
    if blink {
        text_outline_center("PRESS ENTER", tx, 222.0 * u, 22.0 * u, WHITE, rgb(60, 110, 200));
    }
    text("ESC quit", 10.0 * u, VIEW_H * u - 10.0 * u, 11.0 * u, rgba(255, 255, 255, 200));
    let lines = [
        "ARROWS / A D ...... move",
        "Z / SPACE / K ..... jump (hold = higher)",
        "X / SHIFT / J ..... run",
        "R retry   ESC pause   M mute   F11 full",
    ];
    draw_rectangle(tx - 150.0 * u, 238.0 * u, 300.0 * u, 74.0 * u, rgba(30, 20, 40, 120));
    for (i, l) in lines.iter().enumerate() {
        text(l, tx - 138.0 * u, (256.0 + i as f32 * 16.0) * u, 13.0 * u, rgb(255, 250, 235));
    }
}

pub fn draw_pause() {
    dim(0.55);
    let u = ui_scale();
    let cx = screen_width() / 2.0;
    text_outline_center("PAUSED", cx, 150.0 * u, 48.0 * u, WHITE, rgb(60, 110, 200));
    text_center("ESC resume      R retry      Q title", cx, 200.0 * u, 15.0 * u, WHITE);
}

pub fn draw_clear(g: &Game) {
    let k = ((g.clear_t - 1.0) / 0.5).clamp(0.0, 1.0);
    if k <= 0.0 {
        return;
    }
    dim(0.5 * k);
    let u = ui_scale();
    let cx = screen_width() / 2.0;
    let a = k;
    text_outline_center("COURSE CLEAR!", cx, 92.0 * u, 46.0 * u, Color::new(1.0, 0.85, 0.3, a), Color::new(0.6, 0.25, 0.05, a));
    let rank = match g.deaths {
        0 => "S",
        1..=5 => "A",
        6..=15 => "B",
        _ => "C",
    };
    let rows = [
        format!("TIME          {}", fmt_time(g.time)),
        format!("DEATHS        {}", g.deaths),
        format!("ORANGES       {}/{}", g.oranges, g.orange_total),
        format!("GOLDEN        {}/3", g.goldens),
    ];
    for (i, r) in rows.iter().enumerate() {
        text(r, cx - 110.0 * u, (140.0 + i as f32 * 22.0) * u, 17.0 * u, Color::new(1.0, 1.0, 1.0, a));
    }
    text_outline_center(&format!("RANK {rank}"), cx, 252.0 * u, 34.0 * u, Color::new(1.0, 0.95, 0.7, a), Color::new(0.75, 0.35, 0.05, a));
    if g.clear_t > 2.0 && ((g.clear_t * 2.0) as i32) % 2 == 0 {
        text_center("ENTER play again      ESC title", cx, 300.0 * u, 15.0 * u, WHITE);
    }
}
