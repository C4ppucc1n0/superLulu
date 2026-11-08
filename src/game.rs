//! Game state and fixed-timestep simulation.

use macroquad::math::{vec2, Rect, Vec2};
use macroquad::color::Color;

use crate::level::{Level, SpawnKind, Tile};
use crate::physics::*;

pub const VIEW_H: f32 = 360.0;
pub const CRUMBLE_DELAY: f32 = 0.45;
pub const CRUMBLE_RESPAWN: f32 = 3.0;
const PLAT_H_SPEED: f32 = 60.0;
const PLAT_V_SPEED: f32 = 50.0;
const PLAT_THICK: f32 = 16.0;
const TURRET_INTERVAL: f32 = 2.2;
const SEED_SPEED: f32 = 150.0;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Sfx {
    Jump,
    Land,
    Stomp,
    Coin,
    Golden,
    Shield,
    ShieldPop,
    Die,
    Checkpoint,
    Crumble,
    Shoot,
    Win,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Crumble {
    Solid,
    Shaking(f32),
    Gone(f32),
}

#[derive(Clone, Debug)]
pub struct Plat {
    pub rect: Rect,
    pub vel: Vec2,
    pub delta: Vec2,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EKind {
    Jelly,
    Hedgehog,
    Frog,
    Seed,
}

impl EKind {
    pub fn stompable(self) -> bool {
        self != EKind::Hedgehog
    }
    fn size(self) -> Vec2 {
        match self {
            EKind::Jelly => vec2(26.0, 20.0),
            EKind::Hedgehog => vec2(28.0, 22.0),
            EKind::Frog => vec2(24.0, 20.0),
            EKind::Seed => vec2(14.0, 10.0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Enemy {
    pub kind: EKind,
    pub pos: Vec2,
    pub vel: Vec2,
    pub size: Vec2,
    pub dir: f32,
    pub alive: bool,
    /// >0 while playing the squashed animation after a stomp.
    pub dying: f32,
    pub active: bool,
    pub timer: f32,
    pub on_ground: bool,
}

impl Enemy {
    fn new(kind: EKind, pos: Vec2) -> Self {
        Enemy { kind, pos, vel: Vec2::ZERO, size: kind.size(), dir: -1.0, alive: true, dying: 0.0, active: false, timer: 0.0, on_ground: false }
    }
    pub fn rect(&self) -> Rect {
        Rect::new(self.pos.x, self.pos.y, self.size.x, self.size.y)
    }
}

#[derive(Clone, Debug)]
pub struct Turret {
    pub tx: i32,
    pub ty: i32,
    pub dir: f32,
    pub timer: f32,
    pub flash: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ItemKind {
    Orange,
    Golden,
    Shield,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub kind: ItemKind,
    pub pos: Vec2,
    pub taken: bool,
}

impl Item {
    fn rect(&self) -> Rect {
        let r = match self.kind {
            ItemKind::Orange => 9.0,
            _ => 12.0,
        };
        Rect::new(self.pos.x - r, self.pos.y - r, r * 2.0, r * 2.0)
    }
}

#[derive(Clone, Debug)]
pub struct Checkpoint {
    /// Bottom-center (where the player respawns).
    pub pos: Vec2,
    pub active: bool,
}

#[derive(Clone, Debug)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub max: f32,
    pub size: f32,
    pub color: Color,
    pub grav: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Anim {
    pub scale: Vec2,
    pub tilt: f32,
    pub run_phase: f32,
    pub bob: f32,
}

/// Read-only view of the world used for collision during a step.
pub struct WorldView<'a> {
    lv: &'a Level,
    crumbles: &'a [Crumble],
    plats: &'a [Rect],
}

impl Collider for WorldView<'_> {
    fn coll(&self, tx: i32, ty: i32) -> Coll {
        let t = self.lv.tile(tx, ty);
        if t == Tile::Crumble {
            let i = self.lv.idx(tx, ty).unwrap();
            if let Crumble::Gone(_) = self.crumbles[i] {
                return Coll::Empty;
            }
        }
        t.coll()
    }
    fn plats(&self) -> &[Rect] {
        self.plats
    }
}

pub fn spike_rect(t: Tile, tx: i32, ty: i32) -> Option<Rect> {
    let (x, y) = (tx as f32 * TILE, ty as f32 * TILE);
    match t {
        Tile::SpikeUp => Some(Rect::new(x + 4.0, y + 14.0, TILE - 8.0, TILE - 14.0)),
        Tile::SpikeDown => Some(Rect::new(x + 4.0, y, TILE - 8.0, TILE - 14.0)),
        _ => None,
    }
}

/// Smaller-than-hitbox rectangle used for damage, so grazes don't kill.
pub fn hurt_rect(b: &Body) -> Rect {
    Rect::new(b.pos.x + 3.0, b.pos.y + 4.0, PW - 6.0, PH - 4.0)
}

pub struct Game {
    pub lv: Level,
    pub crumbles: Vec<Crumble>,
    pub plats: Vec<Plat>,
    plat_rects: Vec<Rect>,
    pub body: Body,
    pub alive: bool,
    pub death_t: f32,
    pub death_pos: Vec2,
    death_vel: Vec2,
    pub shield: bool,
    pub invuln: f32,
    pub enemies: Vec<Enemy>,
    pub turrets: Vec<Turret>,
    pub items: Vec<Item>,
    pub checkpoints: Vec<Checkpoint>,
    pub goal: Rect,
    pub goal_tile: (i32, i32),
    respawn: Vec2,
    pub deaths: u32,
    pub time: f32,
    pub oranges: u32,
    pub orange_total: u32,
    pub goldens: u32,
    pub cleared: bool,
    pub clear_t: f32,
    pub particles: Vec<Particle>,
    pub sfx: Vec<Sfx>,
    pub cam: Vec2,
    pub view_w: f32,
    look: f32,
    pub anim: Anim,
    pub toast: Option<(&'static str, f32)>,
    pub t: f32,
    pub shake: f32,
    rng: u32,
}

fn tile_center(tx: i32, ty: i32) -> Vec2 {
    vec2((tx as f32 + 0.5) * TILE, (ty as f32 + 0.5) * TILE)
}

impl Game {
    pub fn new(lv: Level) -> Game {
        let start = vec2((lv.start.0 as f32 + 0.5) * TILE, (lv.start.1 + 1) as f32 * TILE);
        let mut items = vec![];
        let mut checkpoints = vec![];
        let mut goal = Rect::new(-1000.0, 0.0, 0.0, 0.0);
        let mut goal_tile = (0, 0);
        for s in &lv.spawns {
            match s.kind {
                SpawnKind::Orange => items.push(Item { kind: ItemKind::Orange, pos: tile_center(s.tx, s.ty), taken: false }),
                SpawnKind::Golden => items.push(Item { kind: ItemKind::Golden, pos: tile_center(s.tx, s.ty), taken: false }),
                SpawnKind::Shield => items.push(Item { kind: ItemKind::Shield, pos: tile_center(s.tx, s.ty), taken: false }),
                SpawnKind::Checkpoint => {
                    checkpoints.push(Checkpoint { pos: vec2((s.tx as f32 + 0.5) * TILE, (s.ty + 1) as f32 * TILE), active: false })
                }
                SpawnKind::Goal => {
                    goal_tile = (s.tx, s.ty);
                    goal = Rect::new(s.tx as f32 * TILE + 30.0, (s.ty - 3) as f32 * TILE, 36.0, 4.0 * TILE);
                }
                _ => {}
            }
        }
        let orange_total = items.iter().filter(|i| i.kind == ItemKind::Orange).count() as u32;
        let n = lv.tiles.len();
        let mut g = Game {
            crumbles: vec![Crumble::Solid; n],
            plats: vec![],
            plat_rects: vec![],
            body: Body::new(start - vec2(PW / 2.0, PH)),
            alive: true,
            death_t: 0.0,
            death_pos: Vec2::ZERO,
            death_vel: Vec2::ZERO,
            shield: false,
            invuln: 0.0,
            enemies: vec![],
            turrets: vec![],
            items,
            checkpoints,
            goal,
            goal_tile,
            respawn: start,
            deaths: 0,
            time: 0.0,
            oranges: 0,
            orange_total,
            goldens: 0,
            cleared: false,
            clear_t: 0.0,
            particles: vec![],
            sfx: vec![],
            cam: Vec2::ZERO,
            view_w: 640.0,
            look: 0.0,
            anim: Anim { scale: Vec2::ONE, ..Default::default() },
            toast: None,
            t: 0.0,
            shake: 0.0,
            rng: 0x1234_5678,
            lv,
        };
        g.reset_world();
        g.update_camera(0.0, true);
        g
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng % 10_000) as f32 / 10_000.0
    }

    /// Reset everything that comes back when you die (enemies, platforms, crumbling blocks, shields).
    fn reset_world(&mut self) {
        self.enemies.clear();
        self.plats.clear();
        self.turrets.clear();
        for c in self.crumbles.iter_mut() {
            *c = Crumble::Solid;
        }
        for s in &self.lv.spawns {
            let bottom = |k: EKind| {
                let sz = k.size();
                vec2(s.tx as f32 * TILE + (TILE - sz.x) / 2.0, (s.ty + 1) as f32 * TILE - sz.y)
            };
            match s.kind {
                SpawnKind::Jelly => self.enemies.push(Enemy::new(EKind::Jelly, bottom(EKind::Jelly))),
                SpawnKind::Hedgehog => self.enemies.push(Enemy::new(EKind::Hedgehog, bottom(EKind::Hedgehog))),
                SpawnKind::Frog => self.enemies.push(Enemy::new(EKind::Frog, bottom(EKind::Frog))),
                SpawnKind::PlatH | SpawnKind::PlatV => {
                    let rect = Rect::new(s.tx as f32 * TILE, s.ty as f32 * TILE, s.len as f32 * TILE, PLAT_THICK);
                    let vel = if s.kind == SpawnKind::PlatH { vec2(PLAT_H_SPEED, 0.0) } else { vec2(0.0, -PLAT_V_SPEED) };
                    self.plats.push(Plat { rect, vel, delta: Vec2::ZERO });
                }
                _ => {}
            }
        }
        for y in 0..self.lv.h {
            for x in 0..self.lv.w {
                let t = self.lv.tile(x, y);
                if t == Tile::TurretL || t == Tile::TurretR {
                    let dir = if t == Tile::TurretL { -1.0 } else { 1.0 };
                    let timer = TURRET_INTERVAL * (0.5 + 0.5 * (y % 2) as f32);
                    self.turrets.push(Turret { tx: x, ty: y, dir, timer, flash: 0.0 });
                }
            }
        }
        for it in self.items.iter_mut() {
            if it.kind == ItemKind::Shield {
                it.taken = false;
            }
        }
        self.plat_rects = self.plats.iter().map(|p| p.rect).collect();
    }

    fn respawn_player(&mut self) {
        self.reset_world();
        self.body = Body::new(self.respawn - vec2(PW / 2.0, PH));
        self.alive = true;
        self.shield = false;
        self.invuln = 0.0;
        self.anim = Anim { scale: vec2(1.3, 0.7), ..Default::default() };
        self.particles.clear();
        self.update_camera(0.0, true);
        self.puff(self.body.center(), 10, Color::new(1.0, 1.0, 1.0, 0.9));
    }

    /// R key: give up and go back to the last checkpoint (counts as a death).
    pub fn retry(&mut self) {
        if self.alive && !self.cleared {
            self.deaths += 1;
            self.respawn_player();
        }
    }

    fn die(&mut self) {
        if !self.alive || self.cleared {
            return;
        }
        self.alive = false;
        self.death_t = 0.0;
        self.deaths += 1;
        self.shield = false;
        let bottom = (self.lv.h as f32) * TILE;
        self.death_pos = vec2(self.body.pos.x, self.body.pos.y.min(bottom));
        self.death_vel = vec2(0.0, if self.body.pos.y > bottom - 10.0 { -620.0 } else { -430.0 });
        self.shake = 0.25;
        self.sfx.push(Sfx::Die);
    }

    fn hurt(&mut self) {
        if self.invuln > 0.0 || !self.alive {
            return;
        }
        if self.shield {
            self.shield = false;
            self.invuln = 1.5;
            self.body.vel.y = self.body.vel.y.min(-260.0);
            self.body.jumping = false;
            self.sfx.push(Sfx::ShieldPop);
            let c = self.body.center();
            self.puff(c, 16, Color::new(0.6, 0.85, 1.0, 0.9));
        } else {
            self.die();
        }
    }

    pub fn puff(&mut self, at: Vec2, n: usize, color: Color) {
        for _ in 0..n {
            let a = self.rand() * std::f32::consts::TAU;
            let s = 40.0 + self.rand() * 120.0;
            let life = 0.3 + self.rand() * 0.3;
            let size = 2.0 + self.rand() * 3.0;
            self.particles.push(Particle { pos: at, vel: vec2(a.cos() * s, a.sin() * s), life, max: life, size, color, grav: 200.0 });
        }
    }

    fn dust(&mut self, at: Vec2, n: usize) {
        for i in 0..n {
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            let life = 0.25 + self.rand() * 0.2;
            let vel = vec2(side * (30.0 + self.rand() * 60.0), -10.0 - self.rand() * 30.0);
            let size = 2.5 + self.rand() * 2.5;
            self.particles.push(Particle { pos: at, vel, life, max: life, size, color: Color::new(1.0, 0.97, 0.9, 0.8), grav: -20.0 });
        }
    }

    pub fn update(&mut self, inp: &Input, dt: f32) {
        self.t += dt;
        self.shake = (self.shake - dt).max(0.0);
        if let Some((_, t)) = &mut self.toast {
            *t -= dt;
            if *t <= 0.0 {
                self.toast = None;
            }
        }
        self.update_particles(dt);

        if self.cleared {
            self.clear_t += dt;
            // walk into the gate
            self.body.vel.x = approach(self.body.vel.x, 0.0, FRICTION * dt);
            let empty = Input::default();
            let wv = WorldView { lv: &self.lv, crumbles: &self.crumbles, plats: &self.plat_rects };
            step_player(&mut self.body, &empty, dt, &wv);
            if self.clear_t < 2.5 && (self.t * 10.0) as i32 % 2 == 0 {
                let x = self.goal.x + self.rand() * self.goal.w;
                let y = self.goal.y + self.rand() * 40.0;
                let col = [Color::new(1.0, 0.6, 0.15, 1.0), Color::new(1.0, 0.85, 0.2, 1.0), Color::new(0.5, 0.85, 1.0, 1.0), Color::new(1.0, 0.5, 0.7, 1.0)]
                    [(self.rand() * 3.99) as usize];
                let vel = vec2((self.rand() - 0.5) * 160.0, -120.0 - self.rand() * 160.0);
                self.particles.push(Particle { pos: vec2(x, y), vel, life: 1.4, max: 1.4, size: 3.0, color: col, grav: 260.0 });
            }
            self.update_anim(dt);
            return;
        }

        if !self.alive {
            self.death_t += dt;
            self.death_vel.y += 1500.0 * dt;
            self.death_pos += self.death_vel * dt;
            if self.death_t > 1.3 || (self.death_t > 0.45 && inp.jump_pressed) {
                self.respawn_player();
            }
            return;
        }

        self.time += dt;
        self.invuln = (self.invuln - dt).max(0.0);

        self.update_plats(dt);

        // ride moving platforms
        if let Some(i) = self.body.plat {
            if let Some(p) = self.plats.get(i) {
                let d = p.delta;
                let wv = WorldView { lv: &self.lv, crumbles: &self.crumbles, plats: &[] };
                sweep_x(&mut self.body.pos, vec2(PW, PH), d.x, &wv, false);
                if d.y < 0.0 {
                    sweep_y(&mut self.body.pos, vec2(PW, PH), d.y, &wv, false, false);
                } else {
                    self.body.pos.y += d.y;
                }
            }
        }

        // player
        let prev_bottom = self.body.pos.y + PH;
        let ev = {
            let wv = WorldView { lv: &self.lv, crumbles: &self.crumbles, plats: &self.plat_rects };
            step_player(&mut self.body, inp, dt, &wv)
        };
        let feet = vec2(self.body.pos.x + PW / 2.0, self.body.pos.y + PH);
        if ev.jumped {
            self.sfx.push(Sfx::Jump);
            self.anim.scale = vec2(0.78, 1.22);
            self.dust(feet, 4);
        }
        if ev.landed {
            self.anim.scale = vec2(1.25, 0.8);
            self.sfx.push(Sfx::Land);
            self.dust(feet, 6);
        }
        if self.body.on_ground && self.body.vel.x.abs() > RUN_MAX * 0.8 && self.rand() < 0.12 {
            self.dust(feet, 1);
        }

        self.update_crumbles(dt);
        self.update_enemies(dt);
        self.update_turrets(dt);
        self.interact(prev_bottom, inp.jump_held);

        if self.body.pos.y > self.lv.h as f32 * TILE + 40.0 {
            self.die();
        }
        self.update_camera(dt, false);
        self.update_anim(dt);
    }

    fn update_plats(&mut self, dt: f32) {
        let lv = &self.lv;
        let blocked = |tx: i32, ty: i32| matches!(lv.tile(tx, ty), Tile::Bumper) || lv.tile(tx, ty).coll() == Coll::Solid;
        for p in self.plats.iter_mut() {
            let mut next = p.rect;
            next.x += p.vel.x * dt;
            next.y += p.vel.y * dt;
            let mut bounce = false;
            if p.vel.x != 0.0 {
                let ex = if p.vel.x > 0.0 { next.x + next.w - 0.01 } else { next.x };
                bounce = blocked((ex / TILE).floor() as i32, (next.y / TILE).floor() as i32);
            }
            if p.vel.y != 0.0 {
                let ey = if p.vel.y > 0.0 { next.y + next.h - 0.01 } else { next.y };
                let ty = (ey / TILE).floor() as i32;
                let c0 = (next.x / TILE).floor() as i32;
                let c1 = ((next.x + next.w - 0.01) / TILE).floor() as i32;
                bounce = (c0..=c1).any(|c| blocked(c, ty));
            }
            if bounce {
                p.vel = -p.vel;
                p.delta = Vec2::ZERO;
            } else {
                p.delta = vec2(next.x - p.rect.x, next.y - p.rect.y);
                p.rect = next;
            }
        }
        self.plat_rects.clear();
        self.plat_rects.extend(self.plats.iter().map(|p| p.rect));
    }

    fn update_crumbles(&mut self, dt: f32) {
        // trigger the blocks under the player's feet
        if self.body.on_ground && self.body.plat.is_none() {
            let ty = ((self.body.pos.y + PH + 1.0) / TILE).floor() as i32;
            let c0 = (self.body.pos.x / TILE).floor() as i32;
            let c1 = ((self.body.pos.x + PW - 0.01) / TILE).floor() as i32;
            for tx in c0..=c1 {
                if self.lv.tile(tx, ty) == Tile::Crumble {
                    let i = self.lv.idx(tx, ty).unwrap();
                    if self.crumbles[i] == Crumble::Solid {
                        self.crumbles[i] = Crumble::Shaking(0.0);
                        self.sfx.push(Sfx::Crumble);
                    }
                }
            }
        }
        let pr = self.body.rect();
        let w = self.lv.w;
        let mut broke = vec![];
        for (i, c) in self.crumbles.iter_mut().enumerate() {
            match c {
                Crumble::Solid => {}
                Crumble::Shaking(t) => {
                    *t += dt;
                    if *t >= CRUMBLE_DELAY {
                        *c = Crumble::Gone(0.0);
                        broke.push(i);
                    }
                }
                Crumble::Gone(t) => {
                    *t += dt;
                    if *t >= CRUMBLE_RESPAWN {
                        let (tx, ty) = (i as i32 % w, i as i32 / w);
                        let r = Rect::new(tx as f32 * TILE, ty as f32 * TILE, TILE, TILE);
                        if !r.overlaps(&pr) {
                            *c = Crumble::Solid;
                        }
                    }
                }
            }
        }
        for i in broke {
            let c = tile_center(i as i32 % w, i as i32 / w);
            self.puff(c, 8, Color::new(0.85, 0.65, 0.4, 1.0));
        }
    }

    fn update_enemies(&mut self, dt: f32) {
        let pc = self.body.center();
        let cam_cx = self.cam.x + self.view_w / 2.0;
        let range = self.view_w / 2.0 + 48.0;
        let bottom = self.lv.h as f32 * TILE + 100.0;
        let mut poofs = vec![];
        {
            let wv = WorldView { lv: &self.lv, crumbles: &self.crumbles, plats: &[] };
            for e in self.enemies.iter_mut() {
                if !e.alive {
                    continue;
                }
                if e.dying > 0.0 {
                    e.dying -= dt;
                    if e.dying <= 0.0 {
                        e.alive = false;
                    }
                    continue;
                }
                if !e.active {
                    if (e.pos.x - cam_cx).abs() < range {
                        e.active = true;
                    } else {
                        continue;
                    }
                }
                match e.kind {
                    EKind::Jelly | EKind::Hedgehog => {
                        let speed = if e.kind == EKind::Jelly { 42.0 } else { 62.0 };
                        e.vel.x = e.dir * speed;
                        e.vel.y = (e.vel.y + G_FALL * dt).min(MAX_FALL);
                        if sweep_x(&mut e.pos, e.size, e.vel.x * dt, &wv, true) {
                            e.dir = -e.dir;
                        }
                        let hit = sweep_y(&mut e.pos, e.size, e.vel.y * dt, &wv, false, false);
                        e.on_ground = hit.landed;
                        if hit.landed {
                            e.vel.y = 0.0;
                            let fx = if e.dir > 0.0 { e.pos.x + e.size.x + 1.0 } else { e.pos.x - 1.0 };
                            let fy = e.pos.y + e.size.y + 2.0;
                            let c = wv.coll((fx / TILE).floor() as i32, (fy / TILE).floor() as i32);
                            if !matches!(c, Coll::Solid | Coll::OneWay) {
                                e.dir = -e.dir;
                            }
                        }
                    }
                    EKind::Frog => {
                        e.vel.y = (e.vel.y + G_FALL * dt).min(MAX_FALL);
                        if e.on_ground {
                            e.vel.x = approach(e.vel.x, 0.0, 900.0 * dt);
                            e.timer += dt;
                            let dx = pc.x - (e.pos.x + e.size.x / 2.0);
                            if dx.abs() > 4.0 {
                                e.dir = dx.signum();
                            }
                            if e.timer > 1.25 && dx.abs() < 8.0 * TILE {
                                e.vel.y = -470.0;
                                e.vel.x = e.dir * 95.0;
                                e.timer = 0.0;
                                e.on_ground = false;
                            }
                        }
                        if sweep_x(&mut e.pos, e.size, e.vel.x * dt, &wv, true) {
                            e.vel.x = 0.0;
                        }
                        let hit = sweep_y(&mut e.pos, e.size, e.vel.y * dt, &wv, false, false);
                        if hit.landed {
                            e.vel.y = 0.0;
                            e.on_ground = true;
                        }
                        if hit.bumped {
                            e.vel.y = 0.0;
                        }
                    }
                    EKind::Seed => {
                        e.pos += e.vel * dt;
                        e.timer += dt;
                        if overlaps_solid(e.rect(), &wv) || e.timer > 8.0 {
                            e.alive = false;
                            poofs.push(e.pos + e.size / 2.0);
                        }
                    }
                }
                if e.pos.y > bottom {
                    e.alive = false;
                }
            }
        }
        for p in poofs {
            self.puff(p, 5, Color::new(0.45, 0.3, 0.2, 1.0));
        }
        self.enemies.retain(|e| e.alive);
    }

    fn update_turrets(&mut self, dt: f32) {
        let cam_cx = self.cam.x + self.view_w / 2.0;
        let range = self.view_w / 2.0 + TILE * 2.0;
        let cam_cy = self.cam.y + VIEW_H / 2.0;
        let mut shots = vec![];
        for t in self.turrets.iter_mut() {
            t.flash = (t.flash - dt).max(0.0);
            let x = (t.tx as f32 + 0.5) * TILE;
            let y = (t.ty as f32 + 0.5) * TILE;
            if (x - cam_cx).abs() > range || (y - cam_cy).abs() > VIEW_H * 0.75 {
                continue;
            }
            t.timer -= dt;
            if t.timer <= 0.0 {
                t.timer += TURRET_INTERVAL;
                t.flash = 0.15;
                let sx = if t.dir < 0.0 { t.tx as f32 * TILE - 14.0 } else { (t.tx + 1) as f32 * TILE };
                shots.push((vec2(sx, t.ty as f32 * TILE + 11.0), t.dir));
            }
        }
        for (pos, dir) in shots {
            let mut e = Enemy::new(EKind::Seed, pos);
            e.vel = vec2(dir * SEED_SPEED, 0.0);
            e.dir = dir;
            e.active = true;
            self.enemies.push(e);
            self.sfx.push(Sfx::Shoot);
        }
    }

    fn interact(&mut self, prev_bottom: f32, jump_held: bool) {
        // enemies
        let pr = self.body.rect();
        let mut stomped = None;
        let mut hurt = false;
        for (i, e) in self.enemies.iter().enumerate() {
            if !e.alive || e.dying > 0.0 || !e.active {
                continue;
            }
            let er = e.rect();
            if !pr.overlaps(&er) {
                continue;
            }
            let from_above = prev_bottom <= er.y + 10.0 && self.body.vel.y >= 0.0;
            if from_above && e.kind.stompable() {
                stomped = Some(i);
                break;
            } else {
                let hr = hurt_rect(&self.body);
                if hr.overlaps(&Rect::new(er.x + 2.0, er.y + 2.0, er.w - 4.0, er.h - 2.0)) {
                    hurt = true;
                }
            }
        }
        if let Some(i) = stomped {
            let e = &mut self.enemies[i];
            let c = e.pos + e.size / 2.0;
            if e.kind == EKind::Seed {
                e.alive = false;
            } else {
                e.dying = 0.35;
            }
            // holding jump while stomping gives a big bounce
            self.body.vel.y = -if jump_held { STOMP_BOUNCE_HELD } else { STOMP_BOUNCE };
            self.body.jumping = true;
            self.body.on_ground = false;
            self.sfx.push(Sfx::Stomp);
            self.puff(c, 8, Color::new(1.0, 1.0, 1.0, 0.9));
        } else if hurt {
            self.hurt();
        }

        // spikes
        let hr = hurt_rect(&self.body);
        let (c0, c1) = ((hr.x / TILE).floor() as i32, ((hr.x + hr.w) / TILE).floor() as i32);
        let (r0, r1) = ((hr.y / TILE).floor() as i32, ((hr.y + hr.h) / TILE).floor() as i32);
        'spikes: for ty in r0..=r1 {
            for tx in c0..=c1 {
                if let Some(sr) = spike_rect(self.lv.tile(tx, ty), tx, ty) {
                    if sr.overlaps(&hr) {
                        self.hurt();
                        break 'spikes;
                    }
                }
            }
        }

        // items
        let pr = self.body.rect();
        let mut got = vec![];
        for (i, it) in self.items.iter_mut().enumerate() {
            if !it.taken && it.rect().overlaps(&pr) {
                it.taken = true;
                got.push(i);
            }
        }
        for i in got {
            let (kind, pos) = (self.items[i].kind, self.items[i].pos);
            match kind {
                ItemKind::Orange => {
                    self.oranges += 1;
                    self.sfx.push(Sfx::Coin);
                    self.puff(pos, 5, Color::new(1.0, 0.75, 0.3, 1.0));
                }
                ItemKind::Golden => {
                    self.goldens += 1;
                    self.sfx.push(Sfx::Golden);
                    self.toast = Some(("GOLDEN ORANGE!", 1.6));
                    self.puff(pos, 18, Color::new(1.0, 0.9, 0.3, 1.0));
                }
                ItemKind::Shield => {
                    self.shield = true;
                    self.sfx.push(Sfx::Shield);
                    self.puff(pos, 10, Color::new(0.6, 0.85, 1.0, 1.0));
                }
            }
        }

        // checkpoints
        for i in 0..self.checkpoints.len() {
            let cp = &self.checkpoints[i];
            let r = Rect::new(cp.pos.x - 14.0, cp.pos.y - 2.0 * TILE, 28.0, 2.0 * TILE);
            if !cp.active && r.overlaps(&pr) {
                self.checkpoints[i].active = true;
                self.respawn = self.checkpoints[i].pos;
                self.sfx.push(Sfx::Checkpoint);
                self.toast = Some(("CHECKPOINT!", 1.6));
                let p = self.checkpoints[i].pos - vec2(0.0, 50.0);
                self.puff(p, 14, Color::new(1.0, 0.65, 0.2, 1.0));
            }
        }

        // goal
        if self.goal.overlaps(&pr) {
            self.cleared = true;
            self.clear_t = 0.0;
            self.sfx.push(Sfx::Win);
        }
    }

    fn update_particles(&mut self, dt: f32) {
        for p in self.particles.iter_mut() {
            p.pos += p.vel * dt;
            p.vel.y += p.grav * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
    }

    pub fn update_camera(&mut self, dt: f32, snap: bool) {
        if !self.alive && !snap {
            return;
        }
        let c = self.body.center();
        let look_target = if self.body.vel.x.abs() > 20.0 { self.body.vel.x.signum() * 40.0 } else { self.look };
        self.look = if snap { look_target } else { approach(self.look, look_target, 90.0 * dt) };
        let max_x = (self.lv.w as f32 * TILE - self.view_w).max(0.0);
        let max_y = (self.lv.h as f32 * TILE - VIEW_H).max(0.0);
        let tx = (c.x + self.look - self.view_w * 0.5).clamp(0.0, max_x);
        let ty = (c.y - VIEW_H * 0.55).clamp(0.0, max_y);
        if snap {
            self.cam = vec2(tx, ty);
        } else {
            let kx = 1.0 - (-dt * 10.0).exp();
            let ky = 1.0 - (-dt * 6.0).exp();
            self.cam.x += (tx - self.cam.x) * kx;
            self.cam.y += (ty - self.cam.y) * ky;
        }
    }

    fn update_anim(&mut self, dt: f32) {
        let b = &self.body;
        let a = &mut self.anim;
        let k = 1.0 - (-dt * 12.0).exp();
        a.scale += (Vec2::ONE - a.scale) * k;
        let speed = b.vel.x.abs();
        let moving = b.on_ground && speed > 10.0;
        if moving {
            a.run_phase += speed * dt * 0.075;
        }
        let tilt_target = if moving {
            a.run_phase.sin() * 0.10 + b.vel.x / RUN_MAX * 0.05
        } else if !b.on_ground {
            b.vel.x / RUN_MAX * 0.12
        } else {
            0.0
        };
        let kt = 1.0 - (-dt * 18.0).exp();
        a.tilt += (tilt_target - a.tilt) * kt;
        let bob_target = if moving { a.run_phase.sin().abs() * 3.0 * (speed / RUN_MAX).max(0.6) } else { 0.0 };
        a.bob += (bob_target - a.bob) * kt;
    }

    /// Debug helper: place the player at a tile column (dropping onto whatever is below).
    pub fn teleport(&mut self, tx: i32, ty: i32) {
        self.body = Body::new(vec2((tx as f32 + 0.5) * TILE - PW / 2.0, (ty + 1) as f32 * TILE - PH));
        self.update_camera(0.0, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 120.0;

    fn game(rows: &[&str]) -> Game {
        let mut g = Game::new(Level::parse(&rows.join("\n")));
        g.view_w = 640.0;
        g.update_camera(0.0, true);
        g
    }

    fn run(g: &mut Game, secs: f32, f: impl Fn(f32) -> Input) {
        let mut t = 0.0;
        while t < secs {
            g.update(&f(t), DT);
            t += DT;
        }
    }

    fn idle(_: f32) -> Input {
        Input::default()
    }
    fn walk_right(_: f32) -> Input {
        Input { dir: 1.0, ..Default::default() }
    }

    #[test]
    fn stands_still_safely() {
        let mut g = game(&["  P        ", "###########"]);
        run(&mut g, 2.0, idle);
        assert!(g.alive && g.body.on_ground && g.deaths == 0);
    }

    #[test]
    fn falling_in_a_pit_kills_and_respawns() {
        let mut g = game(&["  P                 ", "#####     ##########"]);
        run(&mut g, 1.6, walk_right);
        assert_eq!(g.deaths, 1);
        run(&mut g, 2.0, idle);
        assert!(g.alive, "respawned after the death animation");
        assert!((g.body.pos.x - (2.5 * TILE - PW / 2.0)).abs() < 1.0, "back at the start");
    }

    #[test]
    fn stomping_a_jelly_bounces() {
        let mut g = game(&["       P    ", "            ", "            ", "       e    ", "############"]);
        let mut bounced = false;
        for _ in 0..120 {
            g.update(&Input::default(), DT);
            bounced |= g.body.vel.y < -300.0;
        }
        assert!(bounced, "player bounced off the jelly");
        assert!(g.enemies.iter().all(|e| e.kind != EKind::Jelly || e.dying > 0.0));
        assert_eq!(g.deaths, 0);
    }

    #[test]
    fn hedgehogs_cannot_be_stomped() {
        let mut g = game(&["       P    ", "            ", "            ", "       h    ", "############"]);
        run(&mut g, 1.0, idle);
        assert_eq!(g.deaths, 1);
    }

    #[test]
    fn moving_platform_carries_the_player() {
        let mut g = game(&["      P             ", "                    ", "   |HHH        |    ", "                    "]);
        run(&mut g, 0.6, idle);
        assert!(g.body.on_ground && g.body.plat.is_some(), "landed on the platform");
        let x0 = g.body.pos.x;
        run(&mut g, 1.0, idle);
        assert!(g.alive);
        assert!((g.body.pos.x - x0 - PLAT_H_SPEED).abs() < 3.0, "carried ~60px in 1s, moved {}", g.body.pos.x - x0);
    }

    #[test]
    fn crumbling_blocks_drop_you() {
        let mut g = game(&["  P   ", "      ", " CCC  ", "      "]);
        run(&mut g, 0.3, idle);
        assert!(g.body.on_ground);
        run(&mut g, 1.2, idle);
        assert_eq!(g.deaths, 1, "fell after the blocks crumbled");
    }

    #[test]
    fn shield_absorbs_one_hit() {
        let mut g = game(&["  PS     ^^^       ", "###################"]);
        run(&mut g, 2.0, walk_right);
        assert_eq!(g.deaths, 0, "shield saved us from the spikes");
        assert!(!g.shield);
        let mut g2 = game(&["  P      ^^^       ", "###################"]);
        run(&mut g2, 2.0, walk_right);
        assert_eq!(g2.deaths, 1, "no shield -> spikes kill");
    }

    #[test]
    fn checkpoint_then_goal() {
        let mut g = game(&["                        ", "                        ", "                        ", " P    K         G       ", "########################"]);
        run(&mut g, 1.0, walk_right);
        assert!(g.checkpoints[0].active);
        g.retry();
        assert!((g.body.center().x - 6.5 * TILE).abs() < 1.0, "respawn at checkpoint");
        run(&mut g, 4.0, walk_right);
        assert!(g.cleared);
    }

    #[test]
    fn turret_seeds_hurt_and_can_be_jumped() {
        let mut g = game(&["  P        <", "############"]);
        run(&mut g, 4.0, idle);
        assert!(g.deaths >= 1, "standing still in front of a turret is deadly");
    }
}
