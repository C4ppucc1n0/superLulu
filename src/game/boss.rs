//! The boss of level 4: 雷雨云大王, a grumpy storm cloud.
//!
//! It floats over the arena and cycles through a list of attacks (rain, lightning, a lightning sweep
//! with one safe gap, summoning little clouds, swooping across the floor). After each round it runs
//! out of rain and sinks down, tired: that's the moment to whip it, throw oranges at it or stomp it.
//! A ground pound on its head works any time. Three phases; losing a life restarts the current phase.

use super::*;

pub const BOSS_NAME: &str = "雷雨云大王";
pub const BOSS_W: f32 = 104.0;
pub const BOSS_H: f32 = 68.0;
/// Hits needed in each phase.
pub const PHASE_HP: [i32; 3] = [2, 2, 3];
pub const BOSS_HP: i32 = 7;
pub const DROP_R: f32 = 6.0;
pub const BOLT_HALF_W: f32 = 13.0;
pub const BOLT_STRIKE: f32 = 0.3;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Attack {
    Rain,
    Lightning(u8),
    /// A row of bolts across the whole arena with one safe gap.
    Sweep,
    Summon,
    Swoop,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BState {
    Intro,
    Float,
    /// Telegraphing the attack.
    Windup(Attack),
    Act(Attack),
    /// Out of rain: low, slow and vulnerable.
    Tired,
    Hurt,
    PhaseChange,
    Defeated,
}

#[derive(Clone, Debug)]
pub struct Bolt {
    pub x: f32,
    /// Counts down to the strike.
    pub warn: f32,
    pub warn_total: f32,
    /// Counts down while the bolt is striking.
    pub strike: f32,
}

#[derive(Clone, Debug)]
pub struct RainDrop {
    pub pos: Vec2,
    pub vel: Vec2,
}

#[derive(Clone, Debug)]
pub struct Boss {
    /// Centre.
    pub pos: Vec2,
    pub hp: i32,
    pub phase: usize,
    pub state: BState,
    /// Time in the current state.
    pub t: f32,
    pub flash: f32,
    /// Where the eyes look (-1 left .. 1 right).
    pub look: f32,
    pub scale: f32,
    pub bolts: Vec<Bolt>,
    pub drops: Vec<RainDrop>,
    pub name_t: f32,
    /// Time since it was beaten; `gone` once it has burst.
    pub defeat_t: f32,
    pub gone: bool,
    pub floor_y: f32,
    step: usize,
    float_wait: f32,
    fire_t: f32,
    swoop_from: Vec2,
    swoop_to: Vec2,
    min_x: f32,
    max_x: f32,
    home_y: f32,
}

fn phase_start_hp(phase: usize) -> i32 {
    BOSS_HP - PHASE_HP[..phase.min(3)].iter().sum::<i32>()
}

/// The attack rounds of each phase; the boss gets tired after each round.
fn plan(phase: usize) -> &'static [Attack] {
    match phase {
        0 => &[Attack::Rain, Attack::Lightning(2), Attack::Rain],
        1 => &[Attack::Lightning(3), Attack::Summon, Attack::Rain],
        _ => &[Attack::Swoop, Attack::Sweep, Attack::Rain, Attack::Lightning(3)],
    }
}

fn smooth(s: f32) -> f32 {
    s * s * (3.0 - 2.0 * s)
}

impl Boss {
    pub fn new(lv: &Level) -> Boss {
        let floor_y = (lv.start.1 + 1) as f32 * TILE;
        // stays clear of the updraft columns at both walls
        let min_x = 4.6 * TILE;
        let max_x = (lv.w as f32 - 4.6) * TILE;
        Boss {
            pos: vec2((min_x + max_x) / 2.0, -BOSS_H),
            hp: BOSS_HP,
            phase: 0,
            state: BState::Intro,
            t: 0.0,
            flash: 0.0,
            look: 0.0,
            scale: 1.0,
            bolts: vec![],
            drops: vec![],
            name_t: 0.0,
            defeat_t: 0.0,
            gone: false,
            floor_y,
            step: 0,
            float_wait: 1.4,
            fire_t: 0.0,
            swoop_from: Vec2::ZERO,
            swoop_to: Vec2::ZERO,
            min_x,
            max_x,
            home_y: 2.6 * TILE,
        }
    }

    pub fn rect(&self) -> Rect {
        let (w, h) = (BOSS_W * self.scale, BOSS_H * self.scale);
        Rect::new(self.pos.x - w / 2.0, self.pos.y - h / 2.0, w, h)
    }

    /// Body used for touching (a little smaller than the drawing).
    fn body_rect(&self) -> Rect {
        let r = self.rect();
        Rect::new(r.x + 8.0, r.y + 6.0, r.w - 16.0, r.h - 10.0)
    }

    pub fn tired(&self) -> bool {
        self.state == BState::Tired
    }

    /// Can be touched / hit at all right now.
    fn solid(&self) -> bool {
        !self.gone && !matches!(self.state, BState::Intro | BState::Defeated | BState::Hurt | BState::PhaseChange)
    }

    /// Phases go faster.
    fn k(&self) -> f32 {
        [1.0, 1.15, 1.3][self.phase.min(2)]
    }

    fn set(&mut self, s: BState) {
        self.state = s;
        self.t = 0.0;
    }

    fn home_x(&self) -> f32 {
        (self.min_x + self.max_x) / 2.0
    }

    fn follow(&mut self, x: f32, speed: f32, dt: f32) {
        let tx = x.clamp(self.min_x, self.max_x);
        self.pos.x = approach(self.pos.x, tx, speed * dt);
    }

    fn hover(&mut self, dt: f32) {
        let ty = self.home_y + (self.t * 2.2).sin() * 6.0;
        self.pos.y = approach(self.pos.y, ty, 160.0 * dt);
    }

    /// Next attack in the round, or get tired at the end of it.
    fn next(&mut self) {
        self.step += 1;
        if self.step >= plan(self.phase).len() {
            self.step = 0;
            self.set(BState::Tired);
        } else {
            self.float_wait = 0.55 / self.k();
            self.set(BState::Float);
        }
    }
}

impl Game {
    pub(super) fn update_boss(&mut self, dt: f32) {
        let Some(mut bs) = self.boss.take() else { return };
        let pc = self.body.center();
        bs.t += dt;
        bs.name_t += dt;
        bs.flash = (bs.flash - dt).max(0.0);
        let k = bs.k();
        let look = ((pc.x - bs.pos.x) / 120.0).clamp(-1.0, 1.0);
        bs.look += (look - bs.look) * (1.0 - (-dt * 8.0).exp());
        bs.scale = approach(bs.scale, if bs.tired() { 0.8 } else { 1.0 }, dt * 1.5);

        match bs.state {
            BState::Intro => {
                bs.pos.y = approach(bs.pos.y, bs.home_y, 90.0 * dt);
                if bs.t > 2.8 {
                    self.sfx.push(Sfx::BossRoar);
                    self.shake = 0.3;
                    bs.float_wait = 1.0;
                    bs.set(BState::Float);
                }
            }
            BState::Float => {
                bs.follow(pc.x, 70.0 * k, dt);
                bs.hover(dt);
                if bs.t >= bs.float_wait {
                    let a = plan(bs.phase)[bs.step];
                    if a == Attack::Swoop {
                        // fly to the side away from Lulu, then swoop across
                        let left = pc.x > bs.home_x();
                        bs.swoop_from = vec2(if left { bs.min_x } else { bs.max_x }, bs.home_y);
                        bs.swoop_to = vec2(if left { bs.max_x } else { bs.min_x }, bs.home_y);
                    }
                    bs.set(BState::Windup(a));
                }
            }
            BState::Windup(a) => {
                let dur = match a {
                    Attack::Rain => 0.7,
                    Attack::Swoop => 1.0,
                    _ => 0.6,
                } / k.sqrt();
                if a == Attack::Swoop {
                    bs.pos = vec2(approach(bs.pos.x, bs.swoop_from.x, 300.0 * dt), approach(bs.pos.y, bs.home_y, 160.0 * dt));
                } else {
                    bs.hover(dt);
                }
                if bs.t >= dur {
                    self.boss_begin(&mut bs, a, pc);
                    bs.set(BState::Act(a));
                }
            }
            BState::Act(a) => match a {
                Attack::Rain => {
                    bs.follow(pc.x, 105.0 * k, dt);
                    bs.hover(dt);
                    bs.fire_t -= dt;
                    if bs.fire_t <= 0.0 {
                        bs.fire_t += 0.3 / k;
                        let x = bs.pos.x + (self.rand() - 0.5) * 60.0;
                        bs.drops.push(RainDrop { pos: vec2(x, bs.pos.y + 26.0), vel: vec2(0.0, 110.0) });
                    }
                    if bs.t > 2.6 {
                        bs.next();
                    }
                }
                Attack::Lightning(_) | Attack::Sweep => {
                    bs.hover(dt);
                    if bs.bolts.is_empty() && bs.t > 0.3 {
                        bs.next();
                    }
                }
                Attack::Summon => {
                    bs.hover(dt);
                    if bs.t > 0.7 {
                        bs.next();
                    }
                }
                Attack::Swoop => {
                    let dur = 1.5 / k;
                    let s = (bs.t / dur).min(1.0);
                    let low = bs.floor_y - BOSS_H / 2.0 - 4.0;
                    bs.pos.x = bs.swoop_from.x + (bs.swoop_to.x - bs.swoop_from.x) * smooth(s);
                    bs.pos.y = bs.home_y + (low - bs.home_y) * (s * std::f32::consts::PI).sin();
                    if s >= 1.0 {
                        bs.next();
                    }
                }
            },
            BState::Tired => {
                bs.pos.y = approach(bs.pos.y, bs.floor_y - 40.0, 220.0 * dt);
                bs.pos.x = bs.pos.x.clamp(bs.min_x, bs.max_x);
                let dur = [3.6, 3.1, 2.7][bs.phase.min(2)];
                if bs.t > dur {
                    bs.float_wait = 1.0 / k;
                    bs.set(BState::Float);
                }
            }
            BState::Hurt => {
                let home = vec2(bs.home_x(), bs.home_y);
                bs.pos = vec2(approach(bs.pos.x, home.x, 320.0 * dt), approach(bs.pos.y, home.y, 320.0 * dt));
                if bs.t > 1.0 {
                    bs.float_wait = 0.8 / k;
                    bs.set(BState::Float);
                }
            }
            BState::PhaseChange => {
                let home = vec2(bs.home_x(), bs.home_y);
                bs.pos = vec2(approach(bs.pos.x, home.x, 320.0 * dt), approach(bs.pos.y, home.y, 320.0 * dt));
                if bs.t > 0.5 && bs.t - dt <= 0.5 {
                    self.sfx.push(Sfx::BossRoar);
                    self.shake = 0.45;
                }
                if bs.t > 2.0 {
                    bs.float_wait = 0.8;
                    bs.set(BState::Float);
                }
            }
            BState::Defeated => {
                bs.defeat_t += dt;
                self.shake = self.shake.max(0.1);
                if (bs.defeat_t * 12.0) as i32 % 3 == 0 {
                    let p = bs.pos + vec2((self.rand() - 0.5) * 90.0, (self.rand() - 0.5) * 50.0);
                    self.puff(p, 3, Color::new(0.85, 0.88, 1.0, 0.9));
                }
                if bs.defeat_t > 1.8 && !bs.gone {
                    bs.gone = true;
                    self.shake = 0.5;
                    self.sfx.push(Sfx::BossDefeat);
                    self.confetti(bs.pos, 60);
                    self.puff(bs.pos, 30, Color::new(1.0, 1.0, 1.0, 0.95));
                    self.goal = goal_rect(self.goal_tile.0, self.goal_tile.1);
                    self.toast = Some(("打败了雷雨云大王！", 2.5));
                }
            }
        }

        // rain drops
        let hr = hurt_rect(&self.body);
        let mut splash = vec![];
        let mut hit = None;
        {
            let wv = WorldView { lv: &self.lv, crumbles: &self.crumbles, cracked: &self.cracked_gone, plats: &self.plat_rects };
            let floor = bs.floor_y;
            bs.drops.retain_mut(|d| {
                d.vel.y = (d.vel.y + 420.0 * dt).min(360.0);
                d.pos += d.vel * dt;
                let r = Rect::new(d.pos.x - DROP_R, d.pos.y - DROP_R, DROP_R * 2.0, DROP_R * 2.0);
                if r.overlaps(&hr) {
                    hit = Some(d.pos);
                    splash.push(d.pos);
                    return false;
                }
                let solid = wv.coll((d.pos.x / TILE).floor() as i32, ((d.pos.y + DROP_R) / TILE).floor() as i32) == Coll::Solid;
                if solid || d.pos.y > floor + 40.0 {
                    splash.push(d.pos);
                    return false;
                }
                true
            });
        }
        for p in splash {
            self.puff(p, 5, Color::new(0.55, 0.75, 1.0, 0.9));
        }
        // lightning
        let mut struck = false;
        for b in bs.bolts.iter_mut() {
            if b.warn > 0.0 {
                b.warn -= dt;
                if b.warn <= 0.0 {
                    b.strike = BOLT_STRIKE;
                    struck = true;
                }
            } else {
                b.strike -= dt;
                if hr.x < b.x + BOLT_HALF_W && hr.x + hr.w > b.x - BOLT_HALF_W && hr.y < bs.floor_y {
                    hit = Some(vec2(b.x, hr.y));
                }
            }
        }
        bs.bolts.retain(|b| b.warn > 0.0 || b.strike > 0.0);
        if struck {
            self.sfx.push(Sfx::Thunder);
            self.shake = self.shake.max(0.2);
            self.screen_flash = 0.12;
        }
        self.boss = Some(bs);
        if let Some(from) = hit {
            self.hurt(from);
        }
    }

    /// Set up an attack when its wind-up ends.
    fn boss_begin(&mut self, bs: &mut Boss, a: Attack, pc: Vec2) {
        let k = bs.k();
        let (lo, hi) = (1.5 * TILE, (self.lv.w as f32 - 1.5) * TILE);
        match a {
            Attack::Rain => bs.fire_t = 0.0,
            Attack::Lightning(n) => {
                let warn = 1.0 / k;
                let mut xs = vec![pc.x.clamp(lo, hi)];
                let mut tries = 0;
                while xs.len() < n as usize && tries < 50 {
                    tries += 1;
                    let x = lo + self.rand() * (hi - lo);
                    if xs.iter().all(|o| (o - x).abs() > 3.0 * TILE) {
                        xs.push(x);
                    }
                }
                for x in xs {
                    bs.bolts.push(Bolt { x, warn, warn_total: warn, strike: 0.0 });
                }
                self.sfx.push(Sfx::Zap);
            }
            Attack::Sweep => {
                // a wall of bolts with one safe gap a few tiles away from Lulu (always reachable in time)
                let warn = 1.3;
                let side = if self.rand() < 0.5 { -1.0 } else { 1.0 };
                let mut gap = pc.x + side * (3.0 + self.rand() * 3.0) * TILE;
                if gap < lo + 2.0 * TILE || gap > hi - 2.0 * TILE {
                    gap = pc.x - side * (3.0 + self.rand() * 3.0) * TILE;
                }
                let gap = gap.clamp(lo + 2.0 * TILE, hi - 2.0 * TILE);
                let mut x = lo;
                while x <= hi {
                    if (x - gap).abs() > 2.0 * TILE {
                        bs.bolts.push(Bolt { x, warn, warn_total: warn, strike: 0.0 });
                    }
                    x += TILE;
                }
                self.sfx.push(Sfx::Zap);
            }
            Attack::Summon => {
                for s in [-1.0f32, 1.0] {
                    let p = bs.pos + vec2(s * 64.0, 6.0);
                    let mut e = Enemy::new(EKind::Cloudlet, p - EKind::Cloudlet.size() / 2.0);
                    e.active = true;
                    e.dir = s;
                    e.vel = vec2(s * 80.0, -40.0);
                    self.enemies.push(e);
                    self.puff(p, 8, Color::new(0.8, 0.82, 0.92, 0.9));
                }
                self.sfx.push(Sfx::Pop);
            }
            Attack::Swoop => self.sfx.push(Sfx::BossRoar),
        }
    }

    pub(super) fn boss_overlaps(&self, r: Rect) -> bool {
        self.boss.as_ref().is_some_and(|b| b.solid() && b.body_rect().overlaps(&r))
    }

    /// Hit by the whip, an orange or a pound shockwave. Only hurts while it's tired; otherwise the
    /// attack just bounces off. Returns true if it took damage.
    pub(super) fn boss_attacked(&mut self, at: Vec2, head_pound: bool) -> bool {
        let Some(bs) = &self.boss else { return false };
        if !bs.solid() {
            return false;
        }
        if bs.tired() || head_pound {
            self.boss_damage();
            true
        } else {
            self.sfx.push(Sfx::Clink);
            self.stars(at, 4);
            false
        }
    }

    pub(super) fn boss_damage(&mut self) {
        let Some(bs) = &mut self.boss else { return };
        bs.hp -= 1;
        bs.flash = 0.4;
        bs.bolts.clear();
        bs.drops.clear();
        bs.scale = 1.0;
        let pos = bs.pos;
        self.sfx.push(Sfx::BossHurt);
        self.hitstop = self.hitstop.max(0.14);
        self.shake = self.shake.max(0.3);
        if bs.hp <= 0 {
            bs.set(BState::Defeated);
            self.hitstop = 0.25;
            // its little clouds go with it
            for i in 0..self.enemies.len() {
                if self.enemies[i].kind == EKind::Cloudlet && self.enemies[i].hittable() {
                    self.knock_out(i, vec2(0.0, -300.0));
                }
            }
        } else if bs.hp <= phase_start_hp(bs.phase + 1) {
            bs.phase += 1;
            bs.step = 0;
            bs.set(BState::PhaseChange);
            // a heart to keep going
            let mut it = Item::new(ItemKind::Heart, pos + vec2(0.0, 30.0));
            it.vel = vec2(0.0, -200.0);
            it.falling = true;
            self.items.push(it);
        } else {
            bs.set(BState::Hurt);
        }
        // and a couple of oranges to throw
        for s in [-1.0f32, 1.0] {
            let mut it = Item::new(ItemKind::Orange, pos + vec2(s * 20.0, 30.0));
            it.vel = vec2(s * 90.0, -260.0);
            it.falling = true;
            self.items.push(it);
        }
        self.stars(pos, 12);
    }

    /// Touching the boss: pound or stomp its head, or get hurt.
    pub(super) fn boss_contact(&mut self, prev_bottom: f32, jump_held: bool) {
        let Some(bs) = &self.boss else { return };
        if !bs.solid() {
            return;
        }
        let br = bs.body_rect();
        if !self.body.rect().overlaps(&br) {
            return;
        }
        let from = bs.pos;
        let tired = bs.tired();
        let from_above = prev_bottom <= br.y + 16.0 && self.body.vel.y >= 0.0;
        let slam = self.body.pound == Pound::Slam;
        if from_above {
            if slam || tired {
                self.boss_damage();
            } else {
                self.sfx.push(Sfx::Spring);
            }
            let b = &mut self.body;
            b.pound = Pound::None;
            b.vel.y = -if jump_held || slam { STOMP_BOUNCE_HELD } else { STOMP_BOUNCE };
            b.jumping = true;
            b.on_ground = false;
            self.anim.scale = vec2(0.8, 1.2);
        } else if !tired {
            self.hurt(from);
        }
    }

    /// Losing a life restarts the current phase.
    pub(super) fn boss_reset_phase(&mut self) {
        let Some(bs) = &mut self.boss else { return };
        if bs.state == BState::Defeated {
            return;
        }
        bs.hp = phase_start_hp(bs.phase);
        bs.step = 0;
        bs.bolts.clear();
        bs.drops.clear();
        bs.scale = 1.0;
        bs.pos = vec2(bs.home_x(), bs.home_y);
        bs.float_wait = 2.0;
        bs.set(BState::Float);
    }
}
