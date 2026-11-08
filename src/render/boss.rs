//! Drawing for the storm-cloud boss: the cloud itself, its lightning and rain, and its health bar.

use super::*;

#[derive(Clone, Copy, PartialEq)]
enum Mood {
    Angry,
    Charging,
    Tired,
    Hurt,
}

/// The cloud's puffs: (x, y, radius) in a 104 x 68 box centred on (0, 0).
const PUFFS: [(f32, f32, f32); 7] = [
    (-35.0, 8.0, 18.0),
    (35.0, 9.0, 17.0),
    (-17.0, -8.0, 22.0),
    (16.0, -10.0, 22.0),
    (0.0, 8.0, 24.0),
    (-21.0, 15.0, 16.0),
    (21.0, 15.0, 16.0),
];

fn body_colors(mood: Mood, phase: usize) -> (Color, Color, Color) {
    if mood == Mood::Tired {
        return (rgb(156, 162, 200), rgb(196, 202, 232), rgb(84, 88, 128));
    }
    match phase {
        0 => (rgb(78, 80, 122), rgb(116, 118, 162), rgb(32, 32, 60)),
        1 => (rgb(90, 72, 118), rgb(128, 108, 160), rgb(38, 28, 58)),
        _ => (rgb(106, 62, 104), rgb(146, 96, 144), rgb(46, 22, 50)),
    }
}

/// The storm cloud at `c`, `s` = scale (1 = 104 x 68 world pixels).
fn draw_cloud(c: Vec2, s: f32, t: f32, look: f32, mood: Mood, phase: usize, white: bool, alpha: f32) {
    let (mut fill, mut light, mut edge) = body_colors(mood, phase);
    if mood == Mood::Charging {
        let k = (t * 18.0).sin() * 0.5 + 0.5;
        fill = Color::new(fill.r + 0.12 * k, fill.g + 0.1 * k, fill.b, 1.0);
    }
    if white {
        (fill, light, edge) = (rgb(255, 255, 255), rgb(255, 255, 255), rgb(200, 200, 230));
    }
    let (fill, light, edge) = (with_a(fill, alpha), with_a(light, alpha), with_a(edge, alpha));
    let p = |x: f32, y: f32| c + vec2(x, y) * s;
    for (x, y, r) in PUFFS {
        let q = p(x, y);
        draw_circle(q.x, q.y, (r + 3.0) * s, edge);
    }
    for (x, y, r) in PUFFS {
        let q = p(x, y);
        draw_circle(q.x, q.y, r * s, fill);
    }
    for (x, y, r) in [(-23.0, -16.0, 8.0), (10.0, -20.0, 7.0), (-40.0, 3.0, 5.0)] {
        let q = p(x, y);
        draw_circle(q.x, q.y, r * s, light);
    }
    draw_face(c, s, t, look, mood, alpha);
    if mood == Mood::Charging {
        // crackling sparks around the edge
        for k in 0..4 {
            let h = hash(k, (t * 14.0) as i32);
            let ang = (h % 628) as f32 / 100.0;
            let q = c + vec2(ang.cos() * 54.0, ang.sin() * 36.0) * s;
            let d = vec2(ang.cos(), ang.sin()) * 7.0 * s;
            let n = vec2(-d.y, d.x) * 0.5;
            let pts = [q, q + d + n, q + d * 1.4 - n, q + d * 2.4];
            for i in 0..3 {
                draw_line(pts[i].x, pts[i].y, pts[i + 1].x, pts[i + 1].y, 2.5 * s, with_a(GOLD, alpha));
            }
        }
    }
    if mood == Mood::Tired {
        // sweat drops
        for (i, sx) in [-1.0f32, 1.0].iter().enumerate() {
            let k = (t * 1.6 + i as f32 * 0.5) % 1.0;
            let q = p(sx * 46.0, -18.0 + k * 14.0);
            let col = with_a(rgb(150, 210, 255), alpha * (1.0 - k));
            draw_circle(q.x, q.y, 3.5 * s, col);
            draw_triangle(q + vec2(-3.2, -1.0) * s, q + vec2(3.2, -1.0) * s, q + vec2(0.0, -9.0) * s, col);
        }
    }
}

fn draw_face(c: Vec2, s: f32, t: f32, look: f32, mood: Mood, alpha: f32) {
    let ink = with_a(rgb(30, 20, 34), alpha);
    let white = with_a(WHITE, alpha);
    let p = |x: f32, y: f32| c + vec2(x, y) * s;
    for side in [-1.0f32, 1.0] {
        let e = p(side * 15.0, 2.0);
        match mood {
            Mood::Tired => {
                // swirly eyes
                draw_circle(e.x, e.y, 8.5 * s, white);
                let mut prev = e;
                for i in 1..=24 {
                    let th = i as f32 * 0.42 + t * 5.0 * side;
                    let r = i as f32 / 24.0 * 7.0 * s;
                    let q = e + vec2(th.cos(), th.sin()) * r;
                    draw_line(prev.x, prev.y, q.x, q.y, 1.6 * s, ink);
                    prev = q;
                }
            }
            Mood::Hurt => {
                // squeezed shut: > <
                let a = e + vec2(-side * 6.0, -5.0) * s;
                let m = e + vec2(side * 5.0, 0.0) * s;
                let b = e + vec2(-side * 6.0, 5.0) * s;
                draw_line(a.x, a.y, m.x, m.y, 3.0 * s, ink);
                draw_line(m.x, m.y, b.x, b.y, 3.0 * s, ink);
            }
            Mood::Angry | Mood::Charging => {
                draw_ellipse(e.x, e.y, 8.0 * s, 9.0 * s, 0.0, white);
                let pupil = if mood == Mood::Charging { with_a(GOLD, alpha) } else { ink };
                draw_circle(e.x + look * 3.0 * s, e.y + 1.5 * s, 4.2 * s, pupil);
                draw_circle(e.x + look * 3.0 * s - 1.2 * s, e.y, 1.3 * s, white);
                // angry brow
                let a = e + vec2(side * 11.0, -12.0) * s;
                let b = e + vec2(-side * 8.0, -5.0) * s;
                draw_line(a.x, a.y, b.x, b.y, 4.5 * s, ink);
            }
        }
    }
    let m = p(0.0, 19.0);
    match mood {
        Mood::Tired => {
            // wobbly mouth
            for i in 0..6 {
                let x0 = -9.0 + i as f32 * 3.0;
                let a = p(x0, 19.0 + if i % 2 == 0 { 0.0 } else { 2.0 });
                let b = p(x0 + 3.0, 19.0 + if i % 2 == 0 { 2.0 } else { 0.0 });
                draw_line(a.x, a.y, b.x, b.y, 1.8 * s, ink);
            }
        }
        Mood::Hurt => {
            draw_ellipse(m.x, m.y, 7.0 * s, 6.0 * s, 0.0, ink);
            draw_ellipse(m.x, m.y + 2.0 * s, 4.0 * s, 2.5 * s, 0.0, with_a(rgb(255, 110, 140), alpha));
        }
        Mood::Angry | Mood::Charging => {
            draw_ellipse(m.x, m.y + 1.0 * s, 12.0 * s, 5.5 * s, 0.0, ink);
            for i in 0..4 {
                let x0 = -9.0 + i as f32 * 5.0;
                draw_triangle(p(x0, 15.5), p(x0 + 4.0, 15.5), p(x0 + 2.0, 19.5), white);
            }
        }
    }
}

/// The boss face used on the level-select card (screen space).
pub fn draw_storm_face(c: Vec2, s: f32, t: f32, look: f32, tired: bool) {
    let mood = if tired { Mood::Tired } else { Mood::Angry };
    draw_cloud(c, s, t, look, mood, 0, false, 1.0);
}

fn draw_drop(p: Vec2) {
    let edge = rgb(36, 64, 130);
    let fill = rgb(150, 214, 255);
    draw_circle(p.x, p.y, DROP_R, edge);
    draw_triangle(p + vec2(-5.4, -2.0), p + vec2(5.4, -2.0), p + vec2(0.0, -14.0), edge);
    draw_circle(p.x, p.y, DROP_R - 1.6, fill);
    draw_triangle(p + vec2(-3.8, -2.0), p + vec2(3.8, -2.0), p + vec2(0.0, -11.0), fill);
    draw_circle(p.x - 1.5, p.y - 1.0, 1.5, rgba(255, 255, 255, 220));
}

/// The boss and its rain (world space).
pub fn draw_boss_world(g: &Game) {
    let Some(b) = &g.boss else { return };
    for d in &b.drops {
        draw_drop(d.pos);
    }
    if b.gone {
        return;
    }
    let mood = match b.state {
        BState::Tired => Mood::Tired,
        BState::Hurt | BState::Defeated => Mood::Hurt,
        BState::Windup(_) => Mood::Charging,
        _ => Mood::Angry,
    };
    let white = b.flash > 0.0 && ((b.flash * 20.0) as i32) % 2 == 0;
    let mut c = b.pos;
    let mut s = b.scale;
    let mut alpha = 1.0;
    match b.state {
        BState::Defeated => {
            c += vec2((b.defeat_t * 60.0).sin() * 4.0, (b.defeat_t * 47.0).cos() * 3.0);
            s *= 1.0 + b.defeat_t * 0.14;
            alpha = if ((b.defeat_t * 14.0) as i32) % 2 == 0 { 1.0 } else { 0.55 };
        }
        BState::PhaseChange if b.t < 1.4 => c += vec2((b.t * 70.0).sin() * 3.0, 0.0),
        BState::Hurt => c += vec2((b.t * 50.0).sin() * 4.0 * (1.0 - b.t).max(0.0), 0.0),
        _ => {}
    }
    // shadow on the floor
    let k = (1.0 - (b.floor_y - c.y) / 320.0).clamp(0.25, 1.0);
    draw_ellipse(c.x, b.floor_y, 42.0 * s * k, 5.0, 0.0, rgba(0, 0, 0, (60.0 * k) as u8));
    draw_cloud(c, s, g.t, b.look, mood, b.phase, white, alpha);
}

fn zigzag(x: f32, top: f32, bottom: f32, seed: i32, wiggle: f32) -> Vec<Vec2> {
    let n = 10;
    (0..=n)
        .map(|i| {
            let f = i as f32 / n as f32;
            let off = if i == 0 || i == n { 0.0 } else { (hash(seed, i) % 1000) as f32 / 1000.0 * 2.0 - 1.0 };
            vec2(x + off * wiggle, top + (bottom - top) * f)
        })
        .collect()
}

/// Lightning warnings: a glowing column where a bolt will strike (drawn behind the tiles).
pub fn draw_bolt_warnings(g: &Game) {
    let Some(b) = &g.boss else { return };
    let top = g.cam.y - 20.0;
    for bolt in b.bolts.iter().filter(|bl| bl.warn > 0.0) {
        let k = 1.0 - bolt.warn / bolt.warn_total;
        let a = 0.08 + 0.22 * k;
        draw_rectangle(bolt.x - BOLT_HALF_W, top, BOLT_HALF_W * 2.0, b.floor_y - top, Color::new(1.0, 0.9, 0.45, a));
        let mut y = top + ((g.t * 160.0) % 22.0);
        while y < b.floor_y {
            draw_line(bolt.x, y, bolt.x, (y + 11.0).min(b.floor_y), 2.0, Color::new(1.0, 0.95, 0.6, 0.35 + 0.5 * k));
            y += 22.0;
        }
    }
}

/// Ground markers for coming bolts, and the bolts themselves (drawn in front).
pub fn draw_bolts(g: &Game) {
    let Some(b) = &g.boss else { return };
    let top = g.cam.y - 20.0;
    for bolt in &b.bolts {
        if bolt.warn > 0.0 {
            let k = 1.0 - bolt.warn / bolt.warn_total;
            let blink = ((g.t * (5.0 + k * 16.0)) as i32) % 2 == 0;
            let col = if blink { rgba(255, 80, 90, 230) } else { rgba(255, 220, 80, 230) };
            draw_ellipse(bolt.x, b.floor_y - 2.0, BOLT_HALF_W + 5.0, 5.0, 0.0, col);
            continue;
        }
        let k = (bolt.strike / BOLT_STRIKE).clamp(0.0, 1.0);
        let seed = bolt.x as i32 + (g.t * 25.0) as i32 * 7;
        let pts = zigzag(bolt.x, top, b.floor_y, seed, BOLT_HALF_W * 0.8);
        draw_rectangle(bolt.x - BOLT_HALF_W * 1.4, top, BOLT_HALF_W * 2.8, b.floor_y - top, Color::new(1.0, 0.95, 0.6, 0.25 * k));
        for (w, col) in [(12.0, Color::new(1.0, 0.85, 0.3, 0.55 * k + 0.2)), (5.0, Color::new(1.0, 1.0, 0.92, 0.6 + 0.4 * k))] {
            for i in 0..pts.len() - 1 {
                draw_line(pts[i].x, pts[i].y, pts[i + 1].x, pts[i + 1].y, w, col);
            }
            for p in &pts {
                draw_circle(p.x, p.y, w / 2.0, col);
            }
        }
        // a side branch
        let i = 3 + (hash(seed, 99) % 4) as usize;
        let side = if hash(seed, 7) % 2 == 0 { 1.0 } else { -1.0 };
        let q = pts[i] + vec2(side * 18.0, 26.0);
        draw_line(pts[i].x, pts[i].y, q.x, q.y, 3.0, Color::new(1.0, 1.0, 0.9, 0.8 * k));
        // impact
        draw_ellipse(bolt.x, b.floor_y - 2.0, 26.0 * (1.2 - k * 0.4), 7.0, 0.0, Color::new(1.0, 0.95, 0.7, 0.8 * k));
    }
}

/// Boss name, health bar, phase banners and the "hit it now" hint (screen space).
pub fn draw_boss_hud(g: &Game) {
    let Some(b) = &g.boss else { return };
    if b.gone {
        return;
    }
    let u = ui_scale();
    let sw = screen_width();
    let cx = sw / 2.0;

    // big name card while it arrives
    if b.name_t < 3.4 {
        let a = (b.name_t / 0.4).min(1.0) * ((3.4 - b.name_t) / 0.6).min(1.0);
        let slide = (1.0 - (b.name_t / 0.45).min(1.0)).powi(3) * 160.0 * u;
        draw_rectangle(0.0, 132.0 * u, sw, 66.0 * u, Color::new(0.08, 0.04, 0.14, 0.6 * a));
        draw_rectangle(0.0, 132.0 * u, sw, 2.0 * u, Color::new(1.0, 0.84, 0.25, a));
        draw_rectangle(0.0, 196.0 * u, sw, 2.0 * u, Color::new(1.0, 0.84, 0.25, a));
        text_outline_center("BOSS", cx - slide, 154.0 * u, 14.0 * u, Color::new(1.0, 0.84, 0.25, a), Color::new(0.3, 0.1, 0.3, a));
        text_outline_center(BOSS_NAME, cx + slide, 186.0 * u, 30.0 * u, Color::new(1.0, 1.0, 1.0, a), Color::new(0.45, 0.2, 0.6, a));
    }

    // health bar, at the bottom of the screen (the boss floats near the top)
    let appear = ((b.name_t - 2.6) / 0.5).clamp(0.0, 1.0);
    if appear > 0.0 {
        let (bw, bh) = (196.0 * u, 11.0 * u);
        let x = cx - bw / 2.0;
        let y = VIEW_H * u - 26.0 * u + (1.0 - appear) * 50.0 * u;
        rounded_rect(x - 10.0 * u, y - 25.0 * u, bw + 20.0 * u, bh + 33.0 * u, 8.0 * u, rgba(40, 24, 56, 150));
        text_center(BOSS_NAME, cx, y - 7.0 * u, 13.0 * u, rgb(255, 236, 250));
        let gap = 3.0 * u;
        let seg = (bw - gap * (BOSS_HP - 1) as f32) / BOSS_HP as f32;
        let flash = b.flash > 0.0 && ((b.flash * 20.0) as i32) % 2 == 0;
        for i in 0..BOSS_HP {
            let sx = x + i as f32 * (seg + gap);
            if i < b.hp {
                rounded_rect(sx, y, seg, bh, 3.0 * u, if flash { WHITE } else { HEART_DARK });
                rounded_rect(sx, y, seg, bh * 0.6, 3.0 * u, if flash { WHITE } else { HEART });
            } else {
                rounded_rect(sx, y, seg, bh, 3.0 * u, rgba(0, 0, 0, 110));
            }
        }
        // phase marks: between the hits of each phase
        let mut left = BOSS_HP;
        for n in PHASE_HP.iter().take(PHASE_HP.len() - 1) {
            left -= n;
            let mx = x + left as f32 * (seg + gap) - gap / 2.0;
            let my = y + bh + 4.0 * u;
            draw_triangle(vec2(mx, my - 4.0 * u), vec2(mx - 3.5 * u, my + 1.0 * u), vec2(mx + 3.5 * u, my + 1.0 * u), GOLD);
        }
    }

    // phase banner
    if b.state == BState::PhaseChange && b.t < 2.0 {
        let a = (b.t / 0.2).min(1.0) * ((2.0 - b.t) / 0.4).min(1.0);
        let msg = if b.phase >= 2 { "最终阶段！" } else { "第二阶段！" };
        let pop = 1.0 + (0.25 - b.t).max(0.0) * 1.6;
        text_outline_center(msg, cx, 120.0 * u, 26.0 * u * pop, Color::new(1.0, 0.9, 0.4, a), Color::new(0.55, 0.15, 0.4, a));
    }

    // "hit it now!" over the tired cloud
    if b.tired() && g.alive && ((g.t * 3.0) as i32) % 3 != 0 {
        let top = b.pos.y - BOSS_H * b.scale / 2.0 - 12.0 - (g.t * 6.0).sin().abs() * 5.0;
        let sx = (b.pos.x - g.cam.x) * u;
        let sy = (top - g.cam.y) * u;
        text_outline_center("快打它！", sx, sy, 15.0 * u, GOLD_LIGHT, rgb(150, 70, 190));
    }
}
