//! Lulu's attacks: the tie whip (J, three-hit combo), the ground pound (K) and knocking critters out.

use super::*;

/// One swing of the tie whip.
#[derive(Clone, Debug)]
pub struct Whip {
    /// Time since the swing started.
    pub t: f32,
    /// 1, 2 or 3 (the third hit of a combo reaches further and hits harder).
    pub stage: u8,
    pub dir: f32,
    hit_boss: bool,
    boxes: Vec<(i32, i32)>,
}

pub const WHIP_TIME: f32 = 0.24;
/// The whip only hurts in this part of the swing.
const WHIP_ACTIVE: (f32, f32) = (0.03, 0.17);
/// Time after a swing in which the next press continues the combo.
const WHIP_CHAIN: f32 = 0.34;
const WHIP_RECOVER: f32 = 0.28;
/// Ground pound shockwave reach (each side) and height band.
pub const POUND_RADIUS: f32 = 2.6 * TILE;
const POUND_BAND: f32 = 40.0;
/// Two knock-outs within this time count as a combo.
const COMBO_TIME: f32 = 2.0;

impl Whip {
    /// How far the tie is stretched out (0..1) at time `t`.
    pub fn extension(&self) -> f32 {
        let k = self.t / WHIP_TIME;
        if k < 0.35 {
            (k / 0.35).powf(0.6)
        } else {
            1.0 - ((k - 0.35) / 0.65).powi(2)
        }
    }
    pub fn reach(&self) -> f32 {
        if self.stage >= 3 {
            70.0
        } else {
            54.0
        }
    }
    fn active(&self) -> bool {
        self.t >= WHIP_ACTIVE.0 && self.t <= WHIP_ACTIVE.1
    }
}

/// Damage area of a whip swing, measured from the player's body.
pub fn whip_rect(b: &Body, w: &Whip) -> Rect {
    let c = b.center();
    let len = PW / 2.0 + w.reach() * w.extension().max(0.6);
    let y = c.y - 14.0;
    if w.dir > 0.0 {
        Rect::new(c.x, y, len, 26.0)
    } else {
        Rect::new(c.x - len, y, len, 26.0)
    }
}

impl Game {
    /// J: start a swing, or queue the next combo hit if we're mid-swing.
    pub(super) fn update_whip(&mut self, pressed: bool, dt: f32) {
        self.whip_cd = (self.whip_cd - dt).max(0.0);
        self.whip_chain = (self.whip_chain - dt).max(0.0);
        if pressed {
            if self.whip.is_some() {
                self.whip_queue = true;
            } else if self.whip_cd <= 0.0 && !self.body.pounding() {
                self.start_whip();
            }
        }
        let Some(mut w) = self.whip.take() else { return };
        w.t += dt;
        if w.active() {
            self.whip_hits(&mut w);
        }
        if w.t < WHIP_TIME {
            self.whip = Some(w);
            return;
        }
        if w.stage >= 3 {
            self.whip_cd = WHIP_RECOVER;
            self.whip_chain = 0.0;
            self.whip_queue = false;
        } else {
            self.whip_chain = WHIP_CHAIN;
            self.whip_next = w.stage + 1;
            if self.whip_queue {
                self.whip_queue = false;
                self.start_whip();
            }
        }
    }

    fn start_whip(&mut self) {
        let stage = if self.whip_chain > 0.0 { self.whip_next } else { 1 };
        self.whip_chain = 0.0;
        self.whip = Some(Whip { t: 0.0, stage, dir: self.body.facing, hit_boss: false, boxes: vec![] });
        self.sfx.push(Sfx::Whip);
        self.anim.scale = vec2(1.1, 0.92);
    }

    fn whip_hits(&mut self, w: &mut Whip) {
        let r = whip_rect(&self.body, w);
        let push = if w.stage >= 3 { 1.45 } else { 1.0 };
        let mut hit_any = false;
        let mut i = 0;
        while i < self.enemies.len() {
            let e = &self.enemies[i];
            if e.hittable() && e.rect().overlaps(&r) {
                self.knock_out(i, vec2(w.dir * 240.0 * push, -300.0 * push));
                hit_any = true;
            }
            i += 1;
        }
        if !w.hit_boss && self.boss_overlaps(r) {
            w.hit_boss = true;
            hit_any |= self.boss_attacked(r.center(), false);
        }
        // cookie boxes in reach
        let (c0, c1) = ((r.x / TILE).floor() as i32, ((r.x + r.w) / TILE).floor() as i32);
        let (r0, r1) = ((r.y / TILE).floor() as i32, ((r.y + r.h) / TILE).floor() as i32);
        for ty in r0..=r1 {
            for tx in c0..=c1 {
                if self.lv.tile(tx, ty).is_box() && !w.boxes.contains(&(tx, ty)) {
                    w.boxes.push((tx, ty));
                    self.hit_box(tx, ty);
                    hit_any = true;
                }
            }
        }
        if hit_any {
            self.hitstop = self.hitstop.max(if w.stage >= 3 { 0.08 } else { 0.05 });
            self.shake = self.shake.max(if w.stage >= 3 { 0.14 } else { 0.07 });
            let tip = vec2(if w.dir > 0.0 { r.x + r.w } else { r.x }, r.y + r.h / 2.0);
            self.stars(tip, if w.stage >= 3 { 8 } else { 5 });
        }
    }

    /// Knock a critter flying (whip, thrown orange, pound, slam). Seeds just burst.
    pub(super) fn knock_out(&mut self, i: usize, push: Vec2) {
        let c = {
            let e = &mut self.enemies[i];
            let c = e.pos + e.size / 2.0;
            if e.kind == EKind::Seed {
                e.alive = false;
            } else {
                e.ko = true;
                e.vel = push;
                e.timer = 0.0;
                e.spin_v = push.x.signum() * 14.0 + push.x * 0.01;
            }
            c
        };
        self.sfx.push(Sfx::WhipHit);
        self.puff(c, 8, Color::new(1.0, 1.0, 1.0, 0.95));
        self.count_combo(c);
    }

    pub(super) fn count_combo(&mut self, at: Vec2) {
        self.combo = if self.combo_t > 0.0 { self.combo + 1 } else { 1 };
        self.combo_t = COMBO_TIME;
        if self.combo >= 2 {
            self.combo_pop = (at, 0.0);
            self.sfx.push(Sfx::Combo);
        }
    }

    pub(super) fn stars(&mut self, at: Vec2, n: usize) {
        for k in 0..n {
            let a = k as f32 / n as f32 * std::f32::consts::TAU + self.rand() * 0.5;
            let s = 90.0 + self.rand() * 90.0;
            let life = 0.25 + self.rand() * 0.15;
            let col = if k % 2 == 0 { Color::new(1.0, 0.95, 0.5, 1.0) } else { Color::new(1.0, 0.6, 0.8, 1.0) };
            self.particles.push(Particle { pos: at, vel: vec2(a.cos() * s, a.sin() * s), life, max: life, size: 3.0, color: col, grav: 0.0 });
        }
    }

    /// Landed from a ground pound: shockwave, break cracked floors, open boxes, stun critters.
    pub(super) fn pound_land(&mut self) {
        let b = &self.body;
        let feet = vec2(b.pos.x + PW / 2.0, b.pos.y + PH);
        let row = ((b.pos.y + PH + 1.0) / TILE).floor() as i32;
        let (c0, c1) = ((b.pos.x / TILE).floor() as i32, ((b.pos.x + PW - 0.01) / TILE).floor() as i32);
        self.sfx.push(Sfx::Pound);
        self.shake = self.shake.max(0.22);
        self.hitstop = self.hitstop.max(0.05);
        self.anim.scale = vec2(1.45, 0.62);
        self.shockwaves.push((feet, 0.0));
        for k in 0..14 {
            let side = if k % 2 == 0 { -1.0 } else { 1.0 };
            let life = 0.35 + self.rand() * 0.25;
            let vel = vec2(side * (80.0 + self.rand() * 160.0), -20.0 - self.rand() * 70.0);
            let size = 3.0 + self.rand() * 3.0;
            self.particles.push(Particle { pos: feet, vel, life, max: life, size, color: Color::new(1.0, 0.97, 0.9, 0.85), grav: 60.0 });
        }
        // what we landed on
        let mut broke = false;
        for tx in c0..=c1 {
            match self.lv.tile(tx, row) {
                Tile::Cracked => broke |= self.break_cracked(tx, row),
                t if t.is_box() => self.hit_box(tx, row),
                Tile::Crumble => {
                    let i = self.lv.idx(tx, row).unwrap();
                    if !matches!(self.crumbles[i], Crumble::Gone(_)) {
                        self.crumbles[i] = Crumble::Gone(0.0);
                        let c = tile_center(tx, row);
                        self.puff(c, 8, Color::new(0.85, 0.65, 0.4, 1.0));
                    }
                }
                _ => {}
            }
        }
        if broke {
            self.sfx.push(Sfx::Break);
        }
        // shockwave
        let mut i = 0;
        while i < self.enemies.len() {
            let e = &self.enemies[i];
            let ec = e.pos + e.size / 2.0;
            let bottom = e.pos.y + e.size.y;
            if e.hittable() && (ec.x - feet.x).abs() < POUND_RADIUS && (bottom - feet.y).abs() < POUND_BAND {
                let dir = if ec.x >= feet.x { 1.0 } else { -1.0 };
                self.knock_out(i, vec2(dir * 150.0, -420.0));
            }
            i += 1;
        }
        let wave = Rect::new(feet.x - POUND_RADIUS, feet.y - POUND_BAND, POUND_RADIUS * 2.0, POUND_BAND + 8.0);
        if self.boss_overlaps(wave) {
            self.boss_attacked(feet, false);
        }
    }

    /// Break a cracked floor tile and every cracked tile connected to it. Returns true if anything broke.
    fn break_cracked(&mut self, tx: i32, ty: i32) -> bool {
        let mut stack = vec![(tx, ty)];
        let mut any = false;
        while let Some((x, y)) = stack.pop() {
            let Some(i) = self.lv.idx(x, y) else { continue };
            if self.lv.tiles[i] != Tile::Cracked || self.cracked_gone[i] {
                continue;
            }
            self.cracked_gone[i] = true;
            any = true;
            let c = tile_center(x, y);
            for k in 0..6 {
                let vel = vec2((self.rand() - 0.5) * 220.0, -80.0 - self.rand() * 200.0);
                let life = 0.6 + self.rand() * 0.3;
                let col = if k % 2 == 0 { Color::new(0.94, 0.78, 0.52, 1.0) } else { Color::new(0.44, 0.27, 0.17, 1.0) };
                self.particles.push(Particle { pos: c, vel, life, max: life, size: 3.5, color: col, grav: 900.0 });
            }
            stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
        }
        any
    }
}
