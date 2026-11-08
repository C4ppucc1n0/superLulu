//! Game state and fixed-timestep simulation.

mod boss;
mod combat;

pub use boss::*;
pub use combat::*;

use macroquad::color::Color;
use macroquad::math::{vec2, Rect, Vec2};

use crate::level::{Level, SpawnKind, Tile};
use crate::physics::*;

pub const VIEW_H: f32 = 360.0;
pub const CRUMBLE_DELAY: f32 = 0.8;
pub const CRUMBLE_RESPAWN: f32 = 3.0;
pub const MAX_HEARTS: i32 = 3;
const PLAT_H_SPEED: f32 = 60.0;
const PLAT_V_SPEED: f32 = 50.0;
const PLAT_THICK: f32 = 16.0;
const TURRET_INTERVAL: f32 = 3.0;
const SEED_SPEED: f32 = 110.0;
/// Blinking safe time after taking damage.
const INVULN: f32 = 1.6;
const THROW_SPEED: f32 = 340.0;
const THROW_COOLDOWN: f32 = 0.22;
const MAX_THROWN: usize = 3;
pub const THROWN_SIZE: f32 = 12.0;
const BOX_ORANGES: u32 = 3;
/// "LuLu can fly!!!" bubble: how long it shows, and the minimum time between two of them.
pub const BUBBLE_TIME: f32 = 1.6;
const BUBBLE_COOLDOWN: f32 = 6.0;

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
    Hurt,
    Heart,
    Spring,
    Glide,
    Throw,
    NoAmmo,
    Pop,
    BoxHit,
    BoxEmpty,
    Secret,
    Whip,
    WhipHit,
    Combo,
    Spin,
    Pound,
    Break,
    Charge,
    SuperJump,
    Clink,
    Zap,
    Thunder,
    BossRoar,
    BossHurt,
    BossDefeat,
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
    /// Little storm cloud summoned by the boss: floats after you.
    Cloudlet,
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
            EKind::Cloudlet => vec2(28.0, 20.0),
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
    /// Knocked out: flying off the screen, spinning.
    pub ko: bool,
    pub spin: f32,
    spin_v: f32,
}

impl Enemy {
    fn new(kind: EKind, pos: Vec2) -> Self {
        Enemy {
            kind,
            pos,
            vel: Vec2::ZERO,
            size: kind.size(),
            dir: -1.0,
            alive: true,
            dying: 0.0,
            active: false,
            timer: 0.0,
            on_ground: false,
            ko: false,
            spin: 0.0,
            spin_v: 0.0,
        }
    }
    pub fn rect(&self) -> Rect {
        Rect::new(self.pos.x, self.pos.y, self.size.x, self.size.y)
    }
    fn hittable(&self) -> bool {
        self.alive && self.dying <= 0.0 && self.active && !self.ko
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
    Heart,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub kind: ItemKind,
    /// Centre.
    pub pos: Vec2,
    pub vel: Vec2,
    /// Popped out of a box and still bouncing around.
    pub falling: bool,
    pub taken: bool,
    /// Index among the level's golden oranges (for the save file).
    pub id: u8,
}

impl Item {
    fn new(kind: ItemKind, pos: Vec2) -> Item {
        Item { kind, pos, vel: Vec2::ZERO, falling: false, taken: false, id: 0 }
    }
    fn radius(&self) -> f32 {
        match self.kind {
            ItemKind::Orange => 9.0,
            _ => 12.0,
        }
    }
    fn rect(&self) -> Rect {
        let r = self.radius();
        Rect::new(self.pos.x - r, self.pos.y - r, r * 2.0, r * 2.0)
    }
}

/// An orange flying through the air after being thrown.
#[derive(Clone, Debug)]
pub struct Thrown {
    /// Top-left of a THROWN_SIZE square.
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub bounces: u32,
    pub spin: f32,
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
    /// Orange propeller angle and spin speed.
    pub prop: f32,
    pub prop_speed: f32,
    pub throw_t: f32,
    /// Somersault while winding up a ground pound.
    pub spin: f32,
}

/// Read-only view of the world used for collision during a step.
pub struct WorldView<'a> {
    lv: &'a Level,
    crumbles: &'a [Crumble],
    cracked: &'a [bool],
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
        if t == Tile::Cracked && self.cracked[self.lv.idx(tx, ty).unwrap()] {
            return Coll::Empty;
        }
        t.coll()
    }
    fn plats(&self) -> &[Rect] {
        self.plats
    }
    fn fx(&self, tx: i32, ty: i32) -> Fx {
        self.lv.tile(tx, ty).fx()
    }
    fn wind(&self, tx: i32, ty: i32) -> bool {
        self.lv.wind_at(tx, ty)
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

/// Smaller-than-hitbox rectangle used for damage, so grazes don't count.
pub fn hurt_rect(b: &Body) -> Rect {
    Rect::new(b.pos.x + 3.0, b.pos.y + 4.0, PW - 6.0, PH - 4.0)
}

pub fn goal_rect(tx: i32, ty: i32) -> Rect {
    Rect::new(tx as f32 * TILE + 30.0, (ty - 3) as f32 * TILE, 36.0, 4.0 * TILE)
}

pub fn checkpoint_rect(pos: Vec2) -> Rect {
    Rect::new(pos.x - 14.0, pos.y - 2.0 * TILE, 28.0, 2.0 * TILE)
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
    pub hearts: i32,
    pub shield: bool,
    pub invuln: f32,
    /// Last spot of solid ground; a fall into a pit brings you back here.
    safe_pos: Vec2,
    pub enemies: Vec<Enemy>,
    pub turrets: Vec<Turret>,
    pub items: Vec<Item>,
    pub thrown: Vec<Thrown>,
    throw_cd: f32,
    pub checkpoints: Vec<Checkpoint>,
    pub goal: Rect,
    pub goal_tile: (i32, i32),
    respawn: Vec2,
    pub box_used: Vec<bool>,
    /// (tile index, time since bumped)
    pub box_bumps: Vec<(usize, f32)>,
    /// (tile, time since launched)
    pub spring_anims: Vec<((i32, i32), f32)>,
    /// Per hidden room: 0 = hidden .. 1 = fully revealed.
    pub reveal: Vec<f32>,
    revealed: Vec<bool>,
    /// Sign the player is standing at: (number, top-centre of the sign in world space).
    pub sign: Option<(u8, Vec2)>,
    pub deaths: u32,
    pub time: f32,
    /// Oranges in the pocket (thrown ones are spent).
    pub oranges: u32,
    /// Oranges picked up in total.
    pub collected: u32,
    pub orange_total: u32,
    pub goldens: u32,
    pub golden_mask: u8,
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
    pub whip: Option<Whip>,
    whip_cd: f32,
    whip_chain: f32,
    whip_next: u8,
    whip_queue: bool,
    /// Freeze-frame after a big hit.
    pub hitstop: f32,
    pub combo: u32,
    combo_t: f32,
    /// Where the last combo happened and how long ago.
    pub combo_pop: (Vec2, f32),
    /// "LuLu can fly!!!" bubble time left.
    pub bubble_t: f32,
    bubble_cd: f32,
    bubble_used: bool,
    /// Ground pound rings: (centre, age).
    pub shockwaves: Vec<(Vec2, f32)>,
    /// Cracked floor tiles that have been pounded open.
    pub cracked_gone: Vec<bool>,
    pub boss: Option<Boss>,
    /// White flash when lightning strikes.
    pub screen_flash: f32,
    /// Time since the boss gate opened.
    pub gate_t: f32,
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
        let mut golden_id = 0;
        for s in &lv.spawns {
            let at = tile_center(s.tx, s.ty);
            match s.kind {
                SpawnKind::Orange => items.push(Item::new(ItemKind::Orange, at)),
                SpawnKind::Golden => {
                    let mut it = Item::new(ItemKind::Golden, at);
                    it.id = golden_id;
                    golden_id += 1;
                    items.push(it);
                }
                SpawnKind::Shield => items.push(Item::new(ItemKind::Shield, at)),
                SpawnKind::Heart => items.push(Item::new(ItemKind::Heart, at)),
                SpawnKind::Checkpoint => {
                    checkpoints.push(Checkpoint { pos: vec2((s.tx as f32 + 0.5) * TILE, (s.ty + 1) as f32 * TILE), active: false })
                }
                SpawnKind::Goal => {
                    goal_tile = (s.tx, s.ty);
                    goal = goal_rect(s.tx, s.ty);
                }
                _ => {}
            }
        }
        let boxes = lv.tiles.iter().filter(|t| **t == Tile::BoxOrange).count() as u32;
        let orange_total = items.iter().filter(|i| i.kind == ItemKind::Orange).count() as u32 + boxes * BOX_ORANGES;
        let n = lv.tiles.len();
        let groups = lv.fake_groups;
        let body = Body::new(start - vec2(PW / 2.0, PH));
        let boss = lv.boss.then(|| Boss::new(&lv));
        if boss.is_some() {
            goal = Rect::new(-1000.0, 0.0, 0.0, 0.0); // the gate opens when the boss is beaten
        }
        let mut g = Game {
            crumbles: vec![Crumble::Solid; n],
            plats: vec![],
            plat_rects: vec![],
            safe_pos: body.pos,
            body,
            alive: true,
            death_t: 0.0,
            death_pos: Vec2::ZERO,
            death_vel: Vec2::ZERO,
            hearts: MAX_HEARTS,
            shield: false,
            invuln: 0.0,
            enemies: vec![],
            turrets: vec![],
            items,
            thrown: vec![],
            throw_cd: 0.0,
            checkpoints,
            goal,
            goal_tile,
            respawn: start,
            box_used: vec![false; n],
            box_bumps: vec![],
            spring_anims: vec![],
            reveal: vec![0.0; groups],
            revealed: vec![false; groups],
            sign: None,
            deaths: 0,
            time: 0.0,
            oranges: 0,
            collected: 0,
            orange_total,
            goldens: 0,
            golden_mask: 0,
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
            whip: None,
            whip_cd: 0.0,
            whip_chain: 0.0,
            whip_next: 1,
            whip_queue: false,
            hitstop: 0.0,
            combo: 0,
            combo_t: 0.0,
            combo_pop: (Vec2::ZERO, 9.0),
            bubble_t: 0.0,
            bubble_cd: 0.0,
            bubble_used: false,
            shockwaves: vec![],
            cracked_gone: vec![false; n],
            boss,
            screen_flash: 0.0,
            gate_t: 0.0,
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

    /// Reset everything that comes back when you lose a life (enemies, platforms, crumbling blocks, shields, hearts).
    fn reset_world(&mut self) {
        self.enemies.clear();
        self.plats.clear();
        self.turrets.clear();
        self.thrown.clear();
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
            if matches!(it.kind, ItemKind::Shield | ItemKind::Heart) && !it.falling {
                it.taken = false;
            }
        }
        self.plat_rects = self.plats.iter().map(|p| p.rect).collect();
    }

    fn respawn_player(&mut self) {
        self.reset_world();
        self.body = Body::new(self.respawn - vec2(PW / 2.0, PH));
        self.safe_pos = self.body.pos;
        self.alive = true;
        self.hearts = MAX_HEARTS;
        self.shield = false;
        self.invuln = 0.0;
        self.anim = Anim { scale: vec2(1.3, 0.7), ..Default::default() };
        self.particles.clear();
        self.whip = None;
        self.hitstop = 0.0;
        self.boss_reset_phase();
        self.update_camera(0.0, true);
        self.puff(self.body.center(), 10, Color::new(1.0, 1.0, 1.0, 0.9));
    }

    /// R key: give up and go back to the last checkpoint (counts as a lost life).
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
        self.hearts = 0;
        self.death_t = 0.0;
        self.deaths += 1;
        self.shield = false;
        let bottom = (self.lv.h as f32) * TILE;
        self.death_pos = vec2(self.body.pos.x, self.body.pos.y.min(bottom));
        self.death_vel = vec2(0.0, if self.body.pos.y > bottom - 10.0 { -620.0 } else { -430.0 });
        self.shake = 0.25;
        self.sfx.push(Sfx::Die);
    }

    /// Take a hit from something at `from` (used for the knock-back direction).
    fn hurt(&mut self, from: Vec2) {
        if self.invuln > 0.0 || !self.alive || self.cleared {
            return;
        }
        let c = self.body.center();
        if self.shield {
            self.shield = false;
            self.invuln = 1.2;
            self.body.vel.y = self.body.vel.y.min(-260.0);
            self.body.jumping = false;
            self.sfx.push(Sfx::ShieldPop);
            self.puff(c, 16, Color::new(0.6, 0.85, 1.0, 0.9));
            return;
        }
        self.hearts -= 1;
        if self.hearts <= 0 {
            self.die();
            return;
        }
        self.invuln = INVULN;
        let dir = if (c.x - from.x).abs() > 2.0 { (c.x - from.x).signum() } else { -self.body.facing };
        let b = &mut self.body;
        b.vel = vec2(dir * 170.0, -330.0);
        b.pound = Pound::None;
        b.crouch = false;
        b.charge = 0.0;
        b.super_jumping = false;
        b.jumping = false;
        b.boost = false;
        b.gliding = false;
        b.glide_armed = false;
        b.on_ground = false;
        b.plat = None;
        self.shake = 0.15;
        self.anim.scale = vec2(1.25, 0.75);
        self.sfx.push(Sfx::Hurt);
        self.puff(c, 8, Color::new(1.0, 0.5, 0.55, 0.9));
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

    fn confetti(&mut self, at: Vec2, n: usize) {
        let cols = [Color::new(1.0, 0.6, 0.15, 1.0), Color::new(1.0, 0.85, 0.2, 1.0), Color::new(0.5, 0.85, 1.0, 1.0), Color::new(1.0, 0.5, 0.7, 1.0)];
        for _ in 0..n {
            let col = cols[(self.rand() * 3.99) as usize];
            let vel = vec2((self.rand() - 0.5) * 220.0, -120.0 - self.rand() * 200.0);
            self.particles.push(Particle { pos: at, vel, life: 1.2, max: 1.2, size: 3.0, color: col, grav: 300.0 });
        }
    }

    pub fn update(&mut self, inp: &Input, dt: f32) {
        self.t += dt;
        self.shake = (self.shake - dt).max(0.0);
        self.screen_flash = (self.screen_flash - dt).max(0.0);
        if let Some((_, t)) = &mut self.toast {
            *t -= dt;
            if *t <= 0.0 {
                self.toast = None;
            }
        }
        if self.hitstop > 0.0 {
            self.hitstop -= dt;
            return;
        }
        self.update_particles(dt);
        for s in self.shockwaves.iter_mut() {
            s.1 += dt;
        }
        self.shockwaves.retain(|s| s.1 < 0.4);
        self.combo_t = (self.combo_t - dt).max(0.0);
        self.combo_pop.1 += dt;
        self.bubble_t = (self.bubble_t - dt).max(0.0);
        self.bubble_cd = (self.bubble_cd - dt).max(0.0);
        if self.boss.as_ref().is_some_and(|b| b.gone) {
            self.gate_t += dt;
        }
        for b in self.box_bumps.iter_mut() {
            b.1 += dt;
        }
        self.box_bumps.retain(|b| b.1 < 0.25);
        for s in self.spring_anims.iter_mut() {
            s.1 += dt;
        }
        self.spring_anims.retain(|s| s.1 < 0.5);
        for (r, on) in self.reveal.iter_mut().zip(&self.revealed) {
            if *on {
                *r = (*r + dt * 2.5).min(1.0);
            }
        }

        if self.cleared {
            self.clear_t += dt;
            // walk into the gate
            self.body.vel.x = approach(self.body.vel.x, 0.0, FRICTION * dt);
            let empty = Input::default();
            let wv = WorldView { lv: &self.lv, crumbles: &self.crumbles, cracked: &self.cracked_gone, plats: &self.plat_rects };
            step_player(&mut self.body, &empty, dt, &wv);
            if self.clear_t < 2.5 && (self.t * 10.0) as i32 % 2 == 0 {
                let at = vec2(self.goal.x + self.rand() * self.goal.w, self.goal.y + self.rand() * 40.0);
                self.confetti(at, 1);
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
        self.throw_cd = (self.throw_cd - dt).max(0.0);

        self.update_plats(dt);

        // ride moving platforms
        if let Some(i) = self.body.plat {
            if let Some(p) = self.plats.get(i) {
                let d = p.delta;
                let wv = WorldView { lv: &self.lv, crumbles: &self.crumbles, cracked: &self.cracked_gone, plats: &[] };
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
            let wv = WorldView { lv: &self.lv, crumbles: &self.crumbles, cracked: &self.cracked_gone, plats: &self.plat_rects };
            step_player(&mut self.body, inp, dt, &wv)
        };
        let feet = vec2(self.body.pos.x + PW / 2.0, self.body.pos.y + PH);
        if ev.super_jump {
            self.sfx.push(Sfx::SuperJump);
            self.anim.scale = vec2(0.66, 1.38);
            self.dust(feet, 10);
            self.stars(feet, 8);
        } else if ev.jumped {
            self.sfx.push(Sfx::Jump);
            self.anim.scale = vec2(0.78, 1.22);
            self.dust(feet, 4);
        }
        if ev.charged {
            self.sfx.push(Sfx::Charge);
            let c = self.body.center();
            self.stars(c, 6);
        }
        if ev.pound_start {
            self.sfx.push(Sfx::Spin);
        }
        if ev.pound_land {
            self.pound_land();
            self.bubble_used = false;
        } else if ev.landed {
            self.anim.scale = vec2(1.25, 0.8);
            self.sfx.push(Sfx::Land);
            self.dust(feet, 6);
            self.bubble_used = false;
        }
        if self.body.super_jumping && self.rand() < 0.7 {
            let p = feet + vec2((self.rand() - 0.5) * 14.0, -self.rand() * 10.0);
            let life = 0.35;
            self.particles.push(Particle { pos: p, vel: vec2(0.0, 30.0), life, max: life, size: 2.5, color: Color::new(1.0, 0.9, 0.4, 1.0), grav: 0.0 });
        }
        if self.body.super_ready() && self.rand() < 0.25 {
            let c = self.body.center() + vec2((self.rand() - 0.5) * 30.0, (self.rand() - 0.2) * 24.0);
            let life = 0.4;
            self.particles.push(Particle { pos: c, vel: vec2(0.0, -50.0), life, max: life, size: 2.2, color: Color::new(1.0, 0.92, 0.45, 1.0), grav: 0.0 });
        }
        if let Some(tile) = ev.sprung {
            self.sfx.push(Sfx::Spring);
            self.anim.scale = vec2(0.7, 1.3);
            self.spring_anims.push((tile, 0.0));
            self.dust(feet, 6);
        }
        if ev.glide_start {
            self.sfx.push(Sfx::Glide);
            if !self.bubble_used && self.bubble_cd <= 0.0 {
                self.bubble_t = BUBBLE_TIME;
                self.bubble_cd = BUBBLE_COOLDOWN;
                self.bubble_used = true;
            }
        }
        if let Some((tx, ty)) = ev.bump_tile {
            self.hit_box(tx, ty);
        }
        let fast = self.body.vel.x.abs() > RUN_MAX * 0.8 || self.body.slope != 0;
        if self.body.on_ground && fast && self.rand() < 0.15 {
            self.dust(feet, 1);
        }

        self.track_safe_ground();
        if inp.throw_pressed {
            self.throw();
        }
        self.update_whip(inp.whip_pressed, dt);
        self.update_crumbles(dt);
        self.update_enemies(dt);
        self.update_turrets(dt);
        self.update_thrown(dt);
        self.update_items(dt);
        self.update_boss(dt);
        self.interact(prev_bottom, inp.jump_held);
        self.update_secrets();

        if self.body.pos.y > self.lv.h as f32 * TILE + 40.0 {
            self.fell_in_pit();
        }
        self.update_camera(dt, false);
        self.update_anim(dt);
    }

    /// Remember the last place where the player stood on firm ground (not next to spikes).
    fn track_safe_ground(&mut self) {
        let b = &self.body;
        if !b.on_ground || b.plat.is_some() || b.slope != 0 {
            return;
        }
        let row = ((b.pos.y + PH + 0.5) / TILE).floor() as i32;
        let l = (b.pos.x / TILE).floor() as i32;
        let r = ((b.pos.x + PW - 0.01) / TILE).floor() as i32;
        let firm = |t: Tile| {
            matches!(
                t,
                Tile::Ground | Tile::Stone | Tile::OneWay | Tile::TurretL | Tile::TurretR | Tile::Fan | Tile::BoxOrange | Tile::BoxHeart | Tile::Pedestal
            )
        };
        if !firm(self.lv.tile(l, row)) || !firm(self.lv.tile(r, row)) {
            return;
        }
        for tx in l - 1..=r + 1 {
            for ty in row - 2..=row - 1 {
                if matches!(self.lv.tile(tx, ty), Tile::SpikeUp | Tile::SpikeDown) {
                    return;
                }
            }
        }
        self.safe_pos = b.pos;
    }

    /// Falling into a pit costs a heart and puts you back on the last safe ground.
    fn fell_in_pit(&mut self) {
        if self.hearts <= 1 {
            self.die();
            return;
        }
        self.hearts -= 1;
        let facing = self.body.facing;
        self.body = Body::new(self.safe_pos);
        self.body.facing = facing;
        self.invuln = INVULN;
        self.shield = false;
        self.anim.scale = vec2(1.3, 0.7);
        self.sfx.push(Sfx::Hurt);
        let c = self.body.center();
        self.puff(c, 12, Color::new(1.0, 1.0, 1.0, 0.9));
    }

    fn throw(&mut self) {
        if self.throw_cd > 0.0 {
            return;
        }
        if self.oranges == 0 {
            self.sfx.push(Sfx::NoAmmo);
            self.toast = Some(("没有橘子了！", 0.9));
            self.throw_cd = 0.4;
            return;
        }
        if self.thrown.len() >= MAX_THROWN {
            return;
        }
        self.oranges -= 1;
        let f = self.body.facing;
        let c = self.body.center();
        let pos = vec2(c.x - THROWN_SIZE / 2.0 + f * 4.0, c.y - THROWN_SIZE / 2.0 - 6.0);
        let vel = vec2(f * THROW_SPEED + self.body.vel.x * 0.35, -170.0);
        self.thrown.push(Thrown { pos, vel, life: 1.8, bounces: 0, spin: 0.0 });
        self.throw_cd = THROW_COOLDOWN;
        self.anim.throw_t = 0.2;
        self.anim.scale = vec2(1.12, 0.9);
        self.sfx.push(Sfx::Throw);
    }

    /// A cookie box was hit (head bump or thrown orange).
    fn hit_box(&mut self, tx: i32, ty: i32) {
        let t = self.lv.tile(tx, ty);
        if !t.is_box() {
            return;
        }
        let i = self.lv.idx(tx, ty).unwrap();
        self.box_bumps.retain(|b| b.0 != i);
        self.box_bumps.push((i, 0.0));
        if self.box_used[i] {
            self.sfx.push(Sfx::BoxEmpty);
            return;
        }
        self.box_used[i] = true;
        self.sfx.push(Sfx::BoxHit);
        let top = vec2((tx as f32 + 0.5) * TILE, ty as f32 * TILE - 13.0);
        match t {
            Tile::BoxOrange => {
                for k in 0..BOX_ORANGES as i32 {
                    let mut it = Item::new(ItemKind::Orange, top);
                    it.vel = vec2((k - 1) as f32 * 55.0, if k == 1 { -380.0 } else { -320.0 });
                    it.falling = true;
                    self.items.push(it);
                }
            }
            _ => {
                let mut it = Item::new(ItemKind::Heart, top);
                it.vel = vec2(0.0, -300.0);
                it.falling = true;
                self.items.push(it);
            }
        }
        self.puff(top, 8, Color::new(1.0, 0.85, 0.55, 1.0));
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
            let wv = WorldView { lv: &self.lv, crumbles: &self.crumbles, cracked: &self.cracked_gone, plats: &[] };
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
                if e.ko {
                    e.vel.y += 1400.0 * dt;
                    e.pos += e.vel * dt;
                    e.spin += e.spin_v * dt;
                    e.timer += dt;
                    if e.timer > 1.6 {
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
                        let speed = if e.kind == EKind::Jelly { 40.0 } else { 55.0 };
                        e.vel.x = e.dir * speed;
                        e.vel.y = (e.vel.y + G_FALL * dt).min(MAX_FALL);
                        if sweep_x(&mut e.pos, e.size, e.vel.x * dt, &wv, true).is_some() {
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
                            if e.timer > 1.5 && dx.abs() < 8.0 * TILE {
                                e.vel.y = -450.0;
                                e.vel.x = e.dir * 85.0;
                                e.timer = 0.0;
                                e.on_ground = false;
                            }
                        }
                        if sweep_x(&mut e.pos, e.size, e.vel.x * dt, &wv, true).is_some() {
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
                    EKind::Cloudlet => {
                        // drift after Lulu, bobbing
                        let to = pc - (e.pos + e.size / 2.0);
                        let want = to.normalize_or_zero() * 55.0 + vec2(0.0, (e.timer * 4.0).sin() * 25.0);
                        e.vel.x = approach(e.vel.x, want.x, 120.0 * dt);
                        e.vel.y = approach(e.vel.y, want.y, 120.0 * dt);
                        if to.x.abs() > 4.0 {
                            e.dir = to.x.signum();
                        }
                        e.pos += e.vel * dt;
                        e.timer += dt;
                        if e.timer > 9.0 {
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

    fn update_thrown(&mut self, dt: f32) {
        let mut list = std::mem::take(&mut self.thrown);
        let size = vec2(THROWN_SIZE, THROWN_SIZE);
        let bottom = self.lv.h as f32 * TILE + 40.0;
        let mut boxes = vec![];
        let mut splats = vec![];
        let mut pops: Vec<(usize, f32)> = vec![];
        let mut boss_hits = vec![];
        for p in list.iter_mut() {
            p.life -= dt;
            p.vel.y = (p.vel.y + 1100.0 * dt).min(600.0);
            p.spin += p.vel.x.signum() * 14.0 * dt;
            let wv = WorldView { lv: &self.lv, crumbles: &self.crumbles, cracked: &self.cracked_gone, plats: &self.plat_rects };
            if let Some(t) = sweep_x(&mut p.pos, size, p.vel.x * dt, &wv, false) {
                boxes.push(t);
                p.life = 0.0;
            }
            let hit = sweep_y(&mut p.pos, size, p.vel.y * dt, &wv, true, false);
            if hit.landed {
                p.bounces += 1;
                p.vel.y = -260.0 * 0.75f32.powi(p.bounces as i32);
                if p.bounces > 3 {
                    p.life = 0.0;
                }
            }
            if hit.bumped {
                if let Some(t) = hit.tile {
                    boxes.push(t);
                }
                p.life = 0.0;
            }
            let r = Rect::new(p.pos.x, p.pos.y, THROWN_SIZE, THROWN_SIZE);
            if p.life > 0.0 {
                if let Some(i) = self.enemies.iter().position(|e| e.hittable() && e.rect().overlaps(&r)) {
                    pops.push((i, p.vel.x.signum()));
                    p.life = 0.0;
                } else if self.boss_overlaps(r) {
                    boss_hits.push(r.center());
                    p.life = 0.0;
                }
            }
            if p.life <= 0.0 && p.pos.y < bottom {
                splats.push(p.pos + size / 2.0);
            }
            if p.pos.y > bottom {
                p.life = 0.0;
            }
        }
        list.retain(|p| p.life > 0.0);
        self.thrown = list;
        pops.sort_unstable_by_key(|p| p.0);
        pops.dedup_by_key(|p| p.0);
        for (i, dir) in pops {
            self.knock_out(i, vec2(dir * 200.0, -320.0));
        }
        for at in boss_hits {
            self.boss_attacked(at, false);
        }
        for at in splats {
            self.puff(at, 6, Color::new(1.0, 0.65, 0.2, 1.0));
        }
        for (tx, ty) in boxes {
            self.hit_box(tx, ty);
        }
    }

    /// Items popped out of boxes bounce and settle.
    fn update_items(&mut self, dt: f32) {
        let bottom = self.lv.h as f32 * TILE + 40.0;
        let wv = WorldView { lv: &self.lv, crumbles: &self.crumbles, cracked: &self.cracked_gone, plats: &self.plat_rects };
        for it in self.items.iter_mut().filter(|i| i.falling && !i.taken) {
            let r = it.radius();
            let size = vec2(r * 2.0, r * 2.0);
            let mut p = it.pos - vec2(r, r);
            it.vel.y = (it.vel.y + 1200.0 * dt).min(500.0);
            if sweep_x(&mut p, size, it.vel.x * dt, &wv, false).is_some() {
                it.vel.x = -it.vel.x * 0.5;
            }
            let hit = sweep_y(&mut p, size, it.vel.y * dt, &wv, true, false);
            if hit.landed {
                it.vel.x *= 0.5;
                if it.vel.y > 160.0 {
                    it.vel.y = -it.vel.y * 0.35;
                } else {
                    it.vel = Vec2::ZERO;
                    it.falling = false;
                }
            }
            if hit.bumped {
                it.vel.y = 0.0;
            }
            it.pos = p + vec2(r, r);
            if it.pos.y > bottom {
                it.taken = true;
            }
        }
    }

    fn interact(&mut self, prev_bottom: f32, jump_held: bool) {
        // a ground pound slam knocks out anything in its way (even hedgehogs)
        let pr = self.body.rect();
        if self.body.pound == Pound::Slam {
            for i in 0..self.enemies.len() {
                let e = &self.enemies[i];
                if e.hittable() && pr.overlaps(&e.rect()) {
                    let dir = if e.pos.x + e.size.x / 2.0 >= pr.x + pr.w / 2.0 { 1.0 } else { -1.0 };
                    self.knock_out(i, vec2(dir * 160.0, -360.0));
                }
            }
        }
        self.boss_contact(prev_bottom, jump_held);
        // enemies
        let mut stomped = None;
        let mut hurt_by = None;
        for (i, e) in self.enemies.iter().enumerate() {
            if !e.hittable() {
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
                    hurt_by = Some(e.pos + e.size / 2.0);
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
            self.count_combo(c);
        } else if let Some(from) = hurt_by {
            self.hurt(from);
        }

        // spikes
        let hr = hurt_rect(&self.body);
        let (c0, c1) = ((hr.x / TILE).floor() as i32, ((hr.x + hr.w) / TILE).floor() as i32);
        let (r0, r1) = ((hr.y / TILE).floor() as i32, ((hr.y + hr.h) / TILE).floor() as i32);
        let mut spiked = None;
        for ty in r0..=r1 {
            for tx in c0..=c1 {
                if let Some(sr) = spike_rect(self.lv.tile(tx, ty), tx, ty) {
                    if sr.overlaps(&hr) {
                        spiked = Some(vec2(sr.x + sr.w / 2.0, sr.y + sr.h));
                    }
                }
            }
        }
        if let Some(from) = spiked {
            self.hurt(from);
        }

        // items
        let pr = self.body.rect();
        let mut got = vec![];
        for (i, it) in self.items.iter_mut().enumerate() {
            if !it.taken && it.rect().overlaps(&pr) {
                if it.kind == ItemKind::Heart && self.hearts >= MAX_HEARTS && !it.falling {
                    continue; // leave hearts for later when you're at full health
                }
                it.taken = true;
                got.push(i);
            }
        }
        for i in got {
            let (kind, pos, id) = (self.items[i].kind, self.items[i].pos, self.items[i].id);
            match kind {
                ItemKind::Orange => {
                    self.oranges += 1;
                    self.collected += 1;
                    self.sfx.push(Sfx::Coin);
                    self.puff(pos, 5, Color::new(1.0, 0.75, 0.3, 1.0));
                }
                ItemKind::Golden => {
                    self.goldens += 1;
                    self.golden_mask |= 1 << id;
                    self.sfx.push(Sfx::Golden);
                    self.toast = Some(("金橘子！", 1.6));
                    self.puff(pos, 18, Color::new(1.0, 0.9, 0.3, 1.0));
                }
                ItemKind::Shield => {
                    self.shield = true;
                    self.sfx.push(Sfx::Shield);
                    self.puff(pos, 10, Color::new(0.6, 0.85, 1.0, 1.0));
                }
                ItemKind::Heart => {
                    self.hearts = (self.hearts + 1).min(MAX_HEARTS);
                    self.sfx.push(Sfx::Heart);
                    self.puff(pos, 10, Color::new(1.0, 0.45, 0.55, 1.0));
                }
            }
        }

        // checkpoints (they also heal you)
        for i in 0..self.checkpoints.len() {
            let cp = &self.checkpoints[i];
            if !cp.active && checkpoint_rect(cp.pos).overlaps(&pr) {
                self.checkpoints[i].active = true;
                self.respawn = self.checkpoints[i].pos;
                self.hearts = MAX_HEARTS;
                self.sfx.push(Sfx::Checkpoint);
                self.toast = Some(("存档点！", 1.6));
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

    /// Hidden rooms open up when you walk into them; signs show their text when you stand next to them.
    fn update_secrets(&mut self) {
        let pr = self.body.rect();
        // rooms open when you step into them; signs are read from a few pixels further away
        let col = |x: f32| (x / TILE).floor() as i32;
        let (c0, c1) = (col(pr.x - 12.0), col(pr.x + pr.w + 12.0));
        let (r0, r1) = ((pr.y / TILE).floor() as i32, ((pr.y + pr.h - 0.01) / TILE).floor() as i32);
        self.sign = None;
        for ty in r0..=r1 {
            for tx in c0..=c1 {
                let touching = tx >= col(pr.x - 2.0) && tx <= col(pr.x + pr.w + 2.0);
                match self.lv.tile(tx, ty) {
                    Tile::Fake if touching => {
                        let g = self.lv.fake_group[self.lv.idx(tx, ty).unwrap()] as usize;
                        if !self.revealed[g] {
                            self.revealed[g] = true;
                            self.sfx.push(Sfx::Secret);
                            self.toast = Some(("发现隐藏房间！", 1.6));
                        }
                    }
                    Tile::Sign(n) => self.sign = Some((n, vec2((tx as f32 + 0.5) * TILE, ty as f32 * TILE))),
                    _ => {}
                }
            }
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
        let rest = if b.crouch {
            vec2(1.16, 0.78)
        } else if b.pound == Pound::Slam {
            vec2(0.8, 1.22)
        } else {
            Vec2::ONE
        };
        a.scale += (rest - a.scale) * k;
        a.throw_t = (a.throw_t - dt).max(0.0);
        a.spin = match b.pound {
            Pound::Windup(t) => (t / POUND_WINDUP).min(1.0) * std::f32::consts::TAU * b.facing,
            _ => 0.0,
        };
        let speed = b.vel.x.abs();
        let moving = b.on_ground && speed > 10.0 && b.slope == 0;
        if moving {
            a.run_phase += speed * dt * 0.075;
        }
        let tilt_target = if let Some(w) = &self.whip {
            w.dir * 0.16 * w.extension()
        } else if b.slope != 0 {
            b.slope as f32 * 0.5
        } else if moving {
            a.run_phase.sin() * 0.10 + b.vel.x / RUN_MAX * 0.05
        } else if !b.on_ground {
            b.vel.x / RUN_MAX * if b.gliding { 0.06 } else { 0.12 }
        } else {
            0.0
        };
        let kt = 1.0 - (-dt * 18.0).exp();
        a.tilt += (tilt_target - a.tilt) * kt;
        let bob_target = if moving { a.run_phase.sin().abs() * 3.0 * (speed / RUN_MAX).max(0.6) } else { 0.0 };
        a.bob += (bob_target - a.bob) * kt;
        let spin_target = if b.gliding {
            if b.in_wind {
                46.0
            } else {
                30.0
            }
        } else {
            0.0
        };
        a.prop_speed = approach(a.prop_speed, spin_target, 120.0 * dt);
        a.prop += a.prop_speed * dt;
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
        assert!(g.alive && g.body.on_ground && g.deaths == 0 && g.hearts == MAX_HEARTS);
    }

    #[test]
    fn falling_in_a_pit_costs_a_heart() {
        let mut g = game(&["  P                 ", "#####     ##########"]);
        run(&mut g, 1.0, walk_right);
        assert_eq!(g.hearts, 2);
        assert_eq!(g.deaths, 0);
        assert!(g.alive);
        assert!(g.body.pos.x < 5.0 * TILE, "put back before the pit");
        assert!(g.invuln > 0.0);
    }

    #[test]
    fn three_falls_lose_a_life() {
        let mut g = game(&["  P                 ", "#####     ##########"]);
        run(&mut g, 2.6, walk_right);
        assert_eq!(g.deaths, 1);
        run(&mut g, 2.0, idle);
        assert!(g.alive && g.hearts == MAX_HEARTS, "respawned with full hearts");
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
        assert_eq!(g.hearts, MAX_HEARTS);
    }

    #[test]
    fn hedgehogs_hurt_when_stomped() {
        let mut g = game(&["       P    ", "            ", "            ", "       h    ", "############"]);
        run(&mut g, 1.0, idle);
        assert_eq!(g.hearts, MAX_HEARTS - 1);
        assert_eq!(g.deaths, 0);
    }

    #[test]
    fn thrown_orange_knocks_out_a_hedgehog() {
        let mut g = game(&["  P       h      ", "#################"]);
        g.oranges = 2;
        run(&mut g, 0.3, idle);
        g.update(&Input { throw_pressed: true, ..Default::default() }, DT);
        assert_eq!(g.oranges, 1);
        run(&mut g, 1.0, idle);
        assert!(g.enemies.iter().all(|e| e.ko), "hedgehog knocked out");
        run(&mut g, 1.6, idle);
        assert!(g.enemies.is_empty(), "and gone once it flew off");
        assert_eq!(g.hearts, MAX_HEARTS);
    }

    #[test]
    fn throwing_without_oranges_does_nothing() {
        let mut g = game(&["  P       ", "##########"]);
        g.update(&Input { throw_pressed: true, ..Default::default() }, DT);
        assert!(g.thrown.is_empty());
        assert!(g.sfx.contains(&Sfx::NoAmmo));
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
        run(&mut g, 0.4, idle);
        assert!(g.body.on_ground, "still standing: blocks wait {CRUMBLE_DELAY}s");
        run(&mut g, 1.2, idle);
        assert!(g.hearts < MAX_HEARTS, "fell after the blocks crumbled");
    }

    #[test]
    fn shield_absorbs_one_hit() {
        let mut g = game(&["  PS     ^^^       ", "###################"]);
        run(&mut g, 2.0, walk_right);
        assert_eq!(g.hearts, MAX_HEARTS, "shield saved us from the spikes");
        assert!(!g.shield);
        let mut g2 = game(&["  P      ^^^       ", "###################"]);
        run(&mut g2, 2.0, walk_right);
        assert_eq!(g2.hearts, MAX_HEARTS - 1, "no shield -> spikes hurt");
        assert_eq!(g2.deaths, 0);
    }

    #[test]
    fn checkpoint_heals_and_goal_clears() {
        let mut g = game(&["                        ", "                        ", "                        ", " P    K         G       ", "########################"]);
        g.hearts = 1;
        run(&mut g, 1.0, walk_right);
        assert!(g.checkpoints[0].active);
        assert_eq!(g.hearts, MAX_HEARTS);
        g.retry();
        assert!((g.body.center().x - 6.5 * TILE).abs() < 1.0, "respawn at checkpoint");
        run(&mut g, 4.0, walk_right);
        assert!(g.cleared);
    }

    #[test]
    fn turret_seeds_hurt() {
        let mut g = game(&["  P        <", "############"]);
        run(&mut g, 4.0, idle);
        assert!(g.hearts < MAX_HEARTS, "standing still in front of a turret hurts");
    }

    #[test]
    fn spring_launches_high() {
        let mut g = game(&["  P   ", "      ", "      ", "      ", "      ", "      ", "      ", "      ", "  T   ", "######"]);
        let y0 = (g.lv.h - 2) as f32 * TILE;
        let mut top = f32::MAX;
        for _ in 0..90 {
            g.update(&Input::default(), DT);
            top = top.min(g.body.pos.y + PH);
        }
        assert!(y0 - top > 5.0 * TILE, "spring height {} tiles", (y0 - top) / TILE);
    }

    #[test]
    fn cookie_box_pops_oranges() {
        let mut g = game(&["        ", "   Q    ", "        ", "        ", "   P    ", "########"]);
        assert_eq!(g.orange_total, 3);
        run(&mut g, 0.2, idle);
        run(&mut g, 0.5, |t| Input { jump_pressed: t < DT, jump_held: true, ..Default::default() });
        let i = g.lv.idx(3, g.lv.h - 5).unwrap();
        assert!(g.box_used[i], "box was bumped");
        run(&mut g, 2.0, idle);
        let popped: Vec<_> = g.items.iter().filter(|i| i.kind == ItemKind::Orange).collect();
        assert_eq!(popped.len(), 3);
        assert!(popped.iter().all(|i| !i.falling), "oranges settled");
    }

    #[test]
    fn hidden_room_reveals() {
        let mut g = game(&["           ", " P   %%  o ", "###########"]);
        run(&mut g, 2.0, walk_right);
        assert!(g.reveal[0] > 0.9);
        assert_eq!(g.collected, 1, "walked through the fake wall to the orange");
    }

    #[test]
    fn slides_push_you_downhill() {
        let mut g = game(&[" P          ", "#\\          ", "##\\         ", "###\\        ", "####\\       ", "#####\\      ", "############"]);
        let x0 = g.body.pos.x;
        run(&mut g, 1.0, |_| Input { dir: -1.0, ..Default::default() });
        assert!(g.body.pos.x > x0 + 3.0 * TILE, "slid {} px even while holding left", g.body.pos.x - x0);
        assert!(g.alive && g.hearts == MAX_HEARTS);
    }

    #[test]
    fn gliding_crosses_a_wide_gap() {
        let mut g = game(&["  P                          ", "#####           #############"]);
        // run, jump, release, press again and hold
        run(&mut g, 3.0, |t| Input {
            dir: 1.0,
            run: true,
            jump_pressed: (t > 0.3 && t < 0.3 + DT) || (t > 0.75 && t < 0.75 + DT),
            jump_held: (t > 0.3 && t < 0.7) || t > 0.75,
            ..Default::default()
        });
        assert_eq!(g.hearts, MAX_HEARTS, "glided over an 11-tile gap");
        assert!(g.body.pos.x > 16.0 * TILE);
    }

    #[test]
    fn fan_updraft_lifts_a_glider() {
        let mut g = game(&["         ", "         ", "         ", "         ", "         ", "         ", "  P      ", " #UUU#   ", "#########"]);
        let start_y = g.body.pos.y;
        run(&mut g, 0.2, idle);
        run(&mut g, 1.5, |t| Input { jump_pressed: t < DT || (t > 0.2 && t < 0.2 + DT), jump_held: t < 0.15 || t > 0.2, ..Default::default() });
        assert!(g.body.pos.y < start_y - 4.0 * TILE, "rose {} tiles", (start_y - g.body.pos.y) / TILE);
    }

    #[test]
    fn whip_knocks_out_a_hedgehog() {
        let mut g = game(&["  P h     ", "##########"]);
        run(&mut g, 0.05, idle);
        g.update(&Input { whip_pressed: true, ..Default::default() }, DT);
        run(&mut g, 0.3, idle);
        assert!(g.enemies.iter().all(|e| e.ko), "hedgehog knocked out");
        assert_eq!(g.hearts, MAX_HEARTS);
    }

    #[test]
    fn whip_combo_reaches_the_third_hit() {
        let mut g = game(&["  P       ", "##########"]);
        let mut best = 0;
        let mut t = 0.0;
        while t < 1.0 {
            let press = [0.0, 0.1, 0.3].iter().any(|p| t >= *p && t < p + DT);
            g.update(&Input { whip_pressed: press, ..Default::default() }, DT);
            if let Some(w) = &g.whip {
                best = best.max(w.stage);
            }
            t += DT;
        }
        assert_eq!(best, 3);
    }

    #[test]
    fn ground_pound_breaks_cracked_floors() {
        // landing normally doesn't break them
        let mut g = game(&["  P   ", "      ", " BBB  ", "######"]);
        run(&mut g, 1.0, idle);
        assert!(g.cracked_gone.iter().all(|c| !c));
        // a ground pound does, and Lulu drops through
        let mut g = game(&["  P   ", "      ", "      ", "      ", " BBB  ", "      ", "      ", "######"]);
        g.update(&Input { pound_pressed: true, ..Default::default() }, DT);
        run(&mut g, 1.5, idle);
        assert_eq!(g.cracked_gone.iter().filter(|c| **c).count(), 3);
        assert!(g.body.on_ground && (g.body.pos.y + PH - (g.lv.h - 1) as f32 * TILE).abs() < 1.0, "feet at {} ground {}", g.body.pos.y + PH, g.body.on_ground);
    }

    #[test]
    fn super_jump_goes_about_a_tile_higher() {
        let height = |charge: bool| {
            let mut rows = vec!["  P  "];
            rows.extend(std::iter::repeat_n("     ", 8));
            rows.push("#####");
            let mut g = game(&rows);
            run(&mut g, 0.5, idle);
            let y0 = g.body.pos.y;
            if charge {
                run(&mut g, 0.5, |_| Input { down: true, ..Default::default() });
            }
            let mut top = y0;
            let mut t = 0.0;
            while t < 1.2 {
                g.update(&Input { jump_pressed: t < DT, jump_held: true, ..Default::default() }, DT);
                top = top.min(g.body.pos.y);
                t += DT;
            }
            (y0 - top) / TILE
        };
        let (normal, sup) = (height(false), height(true));
        assert!(sup > normal + 0.8 && sup < normal + 1.6, "normal {normal} super {sup}");
    }

    fn boss_game() -> Game {
        let mut g = Game::new(Level::parse(crate::level::LEVELS[3]));
        g.view_w = 640.0;
        g.update_camera(0.0, true);
        g
    }

    #[test]
    fn boss_only_gets_hurt_when_tired() {
        let mut g = boss_game();
        run(&mut g, 4.0, idle);
        let (hp, at) = g.boss.as_ref().map(|b| (b.hp, b.pos)).unwrap();
        assert!(!g.boss_attacked(at, false));
        assert_eq!(g.boss.as_ref().unwrap().hp, hp);
        g.boss.as_mut().unwrap().state = BState::Tired;
        assert!(g.boss_attacked(at, false));
        assert_eq!(g.boss.as_ref().unwrap().hp, hp - 1);
    }

    #[test]
    fn pounding_the_boss_head_hurts_it_any_time() {
        let mut g = boss_game();
        run(&mut g, 3.0, idle);
        let bp = g.boss.as_ref().unwrap().pos;
        g.body.pos = vec2(bp.x - PW / 2.0, bp.y - BOSS_H / 2.0 - PH - 8.0);
        g.body.vel = Vec2::ZERO;
        g.body.on_ground = false;
        g.update(&Input { pound_pressed: true, ..Default::default() }, DT);
        run(&mut g, 0.6, idle);
        assert_eq!(g.boss.as_ref().unwrap().hp, BOSS_HP - 1);
        assert_eq!(g.hearts, MAX_HEARTS);
    }

    #[test]
    fn boss_arena_updraft_reaches_above_the_boss() {
        let mut g = boss_game();
        g.teleport(1, 9);
        run(&mut g, 3.0, |t| Input { jump_pressed: t < DT || (t > 0.25 && t < 0.25 + DT), jump_held: true, ..Default::default() });
        let boss_top = g.boss.as_ref().unwrap().pos.y - BOSS_H / 2.0;
        assert!(g.body.pos.y + PH < boss_top, "feet at {} vs boss top {boss_top}", g.body.pos.y + PH);
    }

    #[test]
    fn losing_a_life_restarts_the_boss_phase() {
        let mut g = boss_game();
        run(&mut g, 3.0, idle);
        // finish phase 1 (2 hits), then one more hit in phase 2
        for _ in 0..3 {
            g.boss_damage();
        }
        let b = g.boss.as_ref().unwrap();
        assert_eq!((b.phase, b.hp), (1, BOSS_HP - 3));
        g.retry();
        let b = g.boss.as_ref().unwrap();
        assert_eq!((b.phase, b.hp), (1, BOSS_HP - PHASE_HP[0]));
        assert_eq!(g.deaths, 1);
    }

    #[test]
    fn beating_the_boss_opens_the_gate() {
        let mut g = boss_game();
        run(&mut g, 3.0, idle);
        assert!(g.goal.x < 0.0, "no gate during the fight");
        for _ in 0..BOSS_HP {
            g.boss_damage();
        }
        assert_eq!(g.boss.as_ref().unwrap().state, BState::Defeated);
        run(&mut g, 2.5, idle);
        assert!(g.boss.as_ref().unwrap().gone);
        let (gx, gy) = g.goal_tile;
        g.teleport(gx + 1, gy);
        run(&mut g, 0.5, idle);
        assert!(g.cleared);
    }
}
