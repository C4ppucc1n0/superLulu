//! Platformer physics: tile collision, player movement, jump feel.
//! Pure logic (no rendering) so it can be unit-tested and reused by the level checker.

use macroquad::math::{vec2, Rect, Vec2};

pub const TILE: f32 = 32.0;
/// Player hitbox (the sprite is drawn a bit larger than this, so hits feel fair).
pub const PW: f32 = 20.0;
pub const PH: f32 = 38.0;

pub const WALK_MAX: f32 = 150.0;
pub const RUN_MAX: f32 = 250.0;
pub const ACCEL_GROUND: f32 = 1000.0;
pub const ACCEL_AIR: f32 = 760.0;
pub const FRICTION: f32 = 1400.0;
pub const TURN_GROUND: f32 = 2600.0;
pub const TURN_AIR: f32 = 1500.0;
pub const OVERSPEED_DECEL: f32 = 700.0;
pub const AIR_DRAG: f32 = 260.0;

pub const JUMP_V: f32 = 560.0;
pub const JUMP_RUN_BONUS: f32 = 45.0;
pub const G_HOLD: f32 = 1400.0; // rising with jump held
pub const G_CUT: f32 = 3600.0; // rising after jump released -> short hop
pub const G_FALL: f32 = 2300.0;
pub const MAX_FALL: f32 = 640.0;

/// Grace time to still jump after walking off a ledge.
pub const COYOTE: f32 = 0.085;
/// A jump pressed this long before landing still fires on landing.
pub const JUMP_BUFFER: f32 = 0.11;
pub const STOMP_BOUNCE: f32 = 480.0;
pub const STOMP_BOUNCE_HELD: f32 = 560.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Coll {
    Empty,
    Solid,
    /// Jump-through platform: only blocks from above.
    OneWay,
    /// Invisible marker that turns enemies and moving platforms around.
    Bumper,
}

pub trait Collider {
    fn coll(&self, tx: i32, ty: i32) -> Coll;
    /// One-way moving platforms (top surface only).
    fn plats(&self) -> &[Rect];
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Input {
    pub dir: f32,
    pub run: bool,
    pub jump_pressed: bool,
    pub jump_held: bool,
}

#[derive(Clone, Debug)]
pub struct Body {
    /// Top-left of the hitbox.
    pub pos: Vec2,
    pub vel: Vec2,
    pub on_ground: bool,
    pub coyote: f32,
    pub jump_buf: f32,
    /// Rising because of a jump/bounce (so releasing the button cuts the jump).
    pub jumping: bool,
    /// Index of the moving platform we stand on.
    pub plat: Option<usize>,
    pub facing: f32,
}

impl Body {
    pub fn new(pos: Vec2) -> Self {
        Body { pos, vel: Vec2::ZERO, on_ground: false, coyote: 0.0, jump_buf: 0.0, jumping: false, plat: None, facing: 1.0 }
    }
    pub fn rect(&self) -> Rect {
        Rect::new(self.pos.x, self.pos.y, PW, PH)
    }
    pub fn center(&self) -> Vec2 {
        self.pos + vec2(PW / 2.0, PH / 2.0)
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct StepEvents {
    pub jumped: bool,
    pub landed: bool,
    pub bumped: bool,
}

pub fn approach(v: f32, target: f32, amount: f32) -> f32 {
    if v < target {
        (v + amount).min(target)
    } else {
        (v - amount).max(target)
    }
}

fn tile_range(a: f32, b: f32) -> (i32, i32) {
    ((a / TILE).floor() as i32, ((b - 0.01) / TILE).floor() as i32)
}

pub fn overlaps_solid(r: Rect, w: &dyn Collider) -> bool {
    let (c0, c1) = tile_range(r.x, r.x + r.w);
    let (r0, r1) = tile_range(r.y, r.y + r.h);
    for ty in r0..=r1 {
        for tx in c0..=c1 {
            if w.coll(tx, ty) == Coll::Solid {
                return true;
            }
        }
    }
    false
}

/// Move horizontally, stopping at solid tiles. Returns true if blocked.
pub fn sweep_x(pos: &mut Vec2, size: Vec2, dx: f32, w: &dyn Collider, bumper_blocks: bool) -> bool {
    if dx == 0.0 {
        return false;
    }
    let blocks = |c: Coll| c == Coll::Solid || (bumper_blocks && c == Coll::Bumper);
    let (r0, r1) = tile_range(pos.y, pos.y + size.y);
    let nx = pos.x + dx;
    if dx > 0.0 {
        let c0 = ((pos.x + size.x) / TILE).floor() as i32;
        let c1 = ((nx + size.x - 0.01) / TILE).floor() as i32;
        for c in c0..=c1 {
            for r in r0..=r1 {
                if blocks(w.coll(c, r)) {
                    pos.x = c as f32 * TILE - size.x;
                    return true;
                }
            }
        }
    } else {
        let c0 = ((pos.x - 0.01) / TILE).floor() as i32;
        let c1 = (nx / TILE).floor() as i32;
        for c in (c1..=c0).rev() {
            for r in r0..=r1 {
                if blocks(w.coll(c, r)) {
                    pos.x = (c + 1) as f32 * TILE;
                    return true;
                }
            }
        }
    }
    pos.x = nx;
    false
}

#[derive(Default, Debug, Clone, Copy)]
pub struct YHit {
    pub landed: bool,
    pub plat: Option<usize>,
    pub bumped: bool,
}

/// Move vertically. Lands on solid tiles, one-way tiles and (optionally) moving platforms.
/// `corner_fix` nudges the body sideways when it clips a ceiling corner by a few pixels.
pub fn sweep_y(pos: &mut Vec2, size: Vec2, dy: f32, w: &dyn Collider, use_plats: bool, corner_fix: bool) -> YHit {
    let mut hit = YHit::default();
    let (c0, c1) = tile_range(pos.x, pos.x + size.x);
    if dy > 0.0 {
        let old_b = pos.y + size.y;
        let new_b = old_b + dy;
        let mut land: Option<(f32, Option<usize>)> = None;
        let r0 = (old_b / TILE).floor() as i32;
        let r1 = ((new_b - 0.001) / TILE).floor() as i32;
        'rows: for r in r0..=r1 {
            let top = r as f32 * TILE;
            if top < old_b - 0.5 {
                continue;
            }
            for c in c0..=c1 {
                if matches!(w.coll(c, r), Coll::Solid | Coll::OneWay) {
                    land = Some((top, None));
                    break 'rows;
                }
            }
        }
        if use_plats {
            for (i, p) in w.plats().iter().enumerate() {
                if pos.x + size.x > p.x && pos.x < p.x + p.w && p.y >= old_b - 2.5 && p.y <= new_b {
                    if land.map_or(true, |(t, _)| p.y < t) {
                        land = Some((p.y, Some(i)));
                    }
                }
            }
        }
        if let Some((top, pi)) = land {
            pos.y = top - size.y;
            hit.landed = true;
            hit.plat = pi;
        } else {
            pos.y = new_b - size.y;
        }
    } else if dy < 0.0 {
        let old_t = pos.y;
        let new_t = old_t + dy;
        let r0 = ((old_t - 0.01) / TILE).floor() as i32;
        let r1 = (new_t / TILE).floor() as i32;
        for r in (r1..=r0).rev() {
            for c in c0..=c1 {
                if w.coll(c, r) == Coll::Solid {
                    if corner_fix {
                        for off in 1..=7 {
                            for s in [-1.0f32, 1.0] {
                                let nx = pos.x + s * off as f32;
                                if !overlaps_solid(Rect::new(nx, new_t, size.x, size.y), w) {
                                    pos.x = nx;
                                    pos.y = new_t;
                                    return hit;
                                }
                            }
                        }
                    }
                    pos.y = (r + 1) as f32 * TILE;
                    hit.bumped = true;
                    return hit;
                }
            }
        }
        pos.y = new_t;
    }
    hit
}

/// One fixed-timestep update of the player body.
pub fn step_player(b: &mut Body, inp: &Input, dt: f32, w: &dyn Collider) -> StepEvents {
    let mut ev = StepEvents::default();

    // --- horizontal ---
    let max = if inp.run { RUN_MAX } else { WALK_MAX };
    let (acc, turn) = if b.on_ground { (ACCEL_GROUND, TURN_GROUND) } else { (ACCEL_AIR, TURN_AIR) };
    if inp.dir != 0.0 {
        b.facing = inp.dir.signum();
        if b.vel.x * inp.dir < 0.0 {
            b.vel.x = approach(b.vel.x, 0.0, turn * dt);
        } else if b.vel.x.abs() < max {
            b.vel.x = approach(b.vel.x, max * inp.dir, acc * dt);
        } else {
            let k = if b.on_ground { 1.0 } else { 0.3 };
            b.vel.x = approach(b.vel.x, max * inp.dir, OVERSPEED_DECEL * k * dt);
        }
    } else {
        let f = if b.on_ground { FRICTION } else { AIR_DRAG };
        b.vel.x = approach(b.vel.x, 0.0, f * dt);
    }

    // --- jump (with coyote time + input buffer) ---
    if b.on_ground {
        b.coyote = COYOTE;
    } else {
        b.coyote -= dt;
    }
    if inp.jump_pressed {
        b.jump_buf = JUMP_BUFFER;
    } else {
        b.jump_buf -= dt;
    }
    if b.jump_buf > 0.0 && b.coyote > 0.0 {
        b.vel.y = -(JUMP_V + JUMP_RUN_BONUS * (b.vel.x.abs() / RUN_MAX).min(1.0));
        b.jumping = true;
        b.on_ground = false;
        b.coyote = 0.0;
        b.jump_buf = 0.0;
        b.plat = None;
        ev.jumped = true;
    }

    // --- gravity (variable jump height) ---
    let g = if b.vel.y < 0.0 {
        if b.jumping && inp.jump_held {
            G_HOLD
        } else {
            G_CUT
        }
    } else {
        G_FALL
    };
    b.vel.y = (b.vel.y + g * dt).min(MAX_FALL);

    // --- move & collide ---
    let size = vec2(PW, PH);
    if sweep_x(&mut b.pos, size, b.vel.x * dt, w, false) {
        b.vel.x = 0.0;
    }
    let was_ground = b.on_ground;
    let hit = sweep_y(&mut b.pos, size, b.vel.y * dt, w, true, true);
    if hit.landed {
        b.on_ground = true;
        b.jumping = false;
        b.vel.y = 0.0;
        b.plat = hit.plat;
        ev.landed = !was_ground;
    } else {
        b.on_ground = false;
        b.plat = None;
    }
    if hit.bumped {
        b.vel.y = 0.0;
        b.jumping = false;
        ev.bumped = true;
    }
    ev
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Flat floor at row 10, nothing else.
    pub struct Flat;
    impl Collider for Flat {
        fn coll(&self, _tx: i32, ty: i32) -> Coll {
            if ty >= 10 {
                Coll::Solid
            } else {
                Coll::Empty
            }
        }
        fn plats(&self) -> &[Rect] {
            &[]
        }
    }

    fn settle(b: &mut Body, w: &dyn Collider) {
        for _ in 0..240 {
            step_player(b, &Input::default(), 1.0 / 120.0, w);
        }
        assert!(b.on_ground);
    }

    /// Prints the jump envelope used to design the level.
    #[test]
    fn jump_capabilities() {
        let dt = 1.0 / 120.0;
        for (name, run, vx) in [("standing", false, 0.0), ("walk", false, WALK_MAX), ("run", true, RUN_MAX)] {
            let mut b = Body::new(vec2(0.0, 10.0 * TILE - PH - 1.0));
            settle(&mut b, &Flat);
            b.vel.x = vx;
            let y0 = b.pos.y;
            let x0 = b.pos.x;
            let mut min_y = y0;
            let mut t = 0.0;
            let mut first = true;
            loop {
                let inp = Input { dir: if vx > 0.0 { 1.0 } else { 0.0 }, run, jump_pressed: first, jump_held: true };
                first = false;
                step_player(&mut b, &inp, dt, &Flat);
                t += dt;
                min_y = min_y.min(b.pos.y);
                if b.on_ground && t > 0.05 {
                    break;
                }
            }
            println!(
                "{name:9}: height {:.1}px ({:.2} tiles)  distance {:.1}px ({:.2} tiles)  airtime {:.2}s",
                y0 - min_y,
                (y0 - min_y) / TILE,
                b.pos.x - x0,
                (b.pos.x - x0) / TILE,
                t
            );
        }
        // short hop
        let mut b = Body::new(vec2(0.0, 10.0 * TILE - PH - 1.0));
        settle(&mut b, &Flat);
        let y0 = b.pos.y;
        let mut min_y = y0;
        for i in 0..120 {
            let inp = Input { jump_pressed: i == 0, jump_held: i < 2, ..Default::default() };
            step_player(&mut b, &inp, dt, &Flat);
            min_y = min_y.min(b.pos.y);
        }
        println!("short hop: height {:.1}px ({:.2} tiles)", y0 - min_y, (y0 - min_y) / TILE);
    }
}
