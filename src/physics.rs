//! Platformer physics: tile collision, player movement, jump feel, glide, super jump, ground pound,
//! springs, wind, slides.
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
pub const COYOTE: f32 = 0.12;
/// A jump pressed this long before landing still fires on landing.
pub const JUMP_BUFFER: f32 = 0.15;
pub const STOMP_BOUNCE: f32 = 480.0;
pub const STOMP_BOUNCE_HELD: f32 = 560.0;

/// Glide (orange propeller): press jump again in the air and hold.
pub const GLIDE_FALL: f32 = 75.0;
pub const GLIDE_DECEL: f32 = 2400.0;
pub const GLIDE_MAX_X: f32 = 190.0;
/// Jelly spring launch speed (~6 tiles high).
pub const SPRING_V: f32 = 740.0;
/// Updraft above fans: gliders rise, everyone else just gets lighter.
pub const WIND_LIFT: f32 = 600.0;
pub const WIND_RISE: f32 = 200.0;
pub const WIND_ACCEL: f32 = 1800.0;
/// Slides (45-degree slopes): you always slide downhill.
pub const SLIDE_MAX: f32 = 330.0;
pub const SLIDE_ACCEL: f32 = 800.0;
/// Super jump: crouch (hold down) this long, then jump. About one tile higher than a normal jump.
pub const SUPER_CHARGE: f32 = 0.4;
pub const SUPER_JUMP_V: f32 = 640.0;
/// A full charge is kept this long after letting go of down (W and S share a finger on WASD).
pub const SUPER_GRACE: f32 = 0.5;
/// Ground pound: hang in the air for a moment, then slam straight down.
pub const POUND_WINDUP: f32 = 0.16;
pub const POUND_V: f32 = 820.0;
/// Pressing pound on the ground first hops up this fast.
pub const POUND_HOP: f32 = 330.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Coll {
    Empty,
    Solid,
    /// Jump-through platform: only blocks from above.
    OneWay,
    /// Invisible marker that turns enemies and moving platforms around.
    Bumper,
}

/// Special surface behaviour of a tile.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fx {
    None,
    Spring,
    /// Slide; the value is the direction you slide (+1 = down to the right, -1 = down to the left).
    Slope(i8),
}

pub trait Collider {
    fn coll(&self, tx: i32, ty: i32) -> Coll;
    /// One-way moving platforms (top surface only).
    fn plats(&self) -> &[Rect];
    fn fx(&self, _tx: i32, _ty: i32) -> Fx {
        Fx::None
    }
    fn wind(&self, _tx: i32, _ty: i32) -> bool {
        false
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Input {
    pub dir: f32,
    pub run: bool,
    pub jump_pressed: bool,
    pub jump_held: bool,
    /// Down held (crouch / charge a super jump).
    pub down: bool,
    pub throw_pressed: bool,
    pub whip_pressed: bool,
    pub pound_pressed: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Pound {
    None,
    /// Little hop before a pound started on the ground.
    Hop,
    /// Hanging in the air (time so far).
    Windup(f32),
    Slam,
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
    /// Jump was pressed again in the air and is still held.
    pub glide_armed: bool,
    pub gliding: bool,
    /// Launched by a spring: full-height rise regardless of the jump button.
    pub boost: bool,
    /// Sliding direction while on a slope (0 = not on a slope).
    pub slope: i8,
    pub in_wind: bool,
    /// Crouching (down held on the ground) and how long the super jump has charged.
    pub crouch: bool,
    pub charge: f32,
    pub super_grace: f32,
    /// Rising from a super jump (for the sparkle trail).
    pub super_jumping: bool,
    pub pound: Pound,
}

impl Body {
    pub fn new(pos: Vec2) -> Self {
        Body {
            pos,
            vel: Vec2::ZERO,
            on_ground: false,
            coyote: 0.0,
            jump_buf: 0.0,
            jumping: false,
            plat: None,
            facing: 1.0,
            glide_armed: false,
            gliding: false,
            boost: false,
            slope: 0,
            in_wind: false,
            crouch: false,
            charge: 0.0,
            super_grace: 0.0,
            super_jumping: false,
            pound: Pound::None,
        }
    }
    /// A super jump would fire if jump were pressed now.
    pub fn super_ready(&self) -> bool {
        (self.crouch && self.charge >= SUPER_CHARGE) || self.super_grace > 0.0
    }
    pub fn pounding(&self) -> bool {
        matches!(self.pound, Pound::Windup(_) | Pound::Slam)
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
    /// Tile hit with the head (cookie boxes react to this).
    pub bump_tile: Option<(i32, i32)>,
    pub sprung: Option<(i32, i32)>,
    pub glide_start: bool,
    pub super_jump: bool,
    /// The super jump just finished charging.
    pub charged: bool,
    pub pound_start: bool,
    /// Landed from a ground pound slam.
    pub pound_land: bool,
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

/// Move horizontally, stopping at solid tiles. Returns the blocking tile, if any.
pub fn sweep_x(pos: &mut Vec2, size: Vec2, dx: f32, w: &dyn Collider, bumper_blocks: bool) -> Option<(i32, i32)> {
    if dx == 0.0 {
        return None;
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
                    return Some((c, r));
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
                    return Some((c, r));
                }
            }
        }
    }
    pos.x = nx;
    None
}

#[derive(Default, Debug, Clone, Copy)]
pub struct YHit {
    pub landed: bool,
    pub plat: Option<usize>,
    pub bumped: bool,
    /// Tile we landed on / bumped into.
    pub tile: Option<(i32, i32)>,
}

/// Move vertically. Lands on solid tiles, one-way tiles and (optionally) moving platforms.
/// `corner_fix` nudges the body sideways when it clips a ceiling corner by a few pixels.
pub fn sweep_y(pos: &mut Vec2, size: Vec2, dy: f32, w: &dyn Collider, use_plats: bool, corner_fix: bool) -> YHit {
    let mut hit = YHit::default();
    let (c0, c1) = tile_range(pos.x, pos.x + size.x);
    if dy > 0.0 {
        let old_b = pos.y + size.y;
        let new_b = old_b + dy;
        let mut land: Option<(f32, Option<usize>, Option<(i32, i32)>)> = None;
        let r0 = (old_b / TILE).floor() as i32;
        let r1 = ((new_b - 0.001) / TILE).floor() as i32;
        'rows: for r in r0..=r1 {
            let top = r as f32 * TILE;
            if top < old_b - 0.5 {
                continue;
            }
            for c in c0..=c1 {
                if matches!(w.coll(c, r), Coll::Solid | Coll::OneWay) {
                    land = Some((top, None, Some((c, r))));
                    break 'rows;
                }
            }
        }
        if use_plats {
            for (i, p) in w.plats().iter().enumerate() {
                if pos.x + size.x > p.x && pos.x < p.x + p.w && p.y >= old_b - 2.5 && p.y <= new_b {
                    if land.map_or(true, |(t, _, _)| p.y < t) {
                        land = Some((p.y, Some(i), None));
                    }
                }
            }
        }
        if let Some((top, pi, tile)) = land {
            pos.y = top - size.y;
            hit.landed = true;
            hit.plat = pi;
            hit.tile = tile;
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
                    hit.tile = Some((c, r));
                    return hit;
                }
            }
        }
        pos.y = new_t;
    }
    hit
}

/// Surface height of a slope tile at world x (45 degrees).
pub fn slope_surface(dir: i8, tx: i32, ty: i32, x: f32) -> f32 {
    let lx = (x - tx as f32 * TILE).clamp(0.0, TILE);
    if dir > 0 {
        ty as f32 * TILE + lx // '\' : high on the left, slides right
    } else {
        (ty + 1) as f32 * TILE - lx // '/' : high on the right, slides left
    }
}

/// Keep the body glued to slide tiles. Returns the slide direction if standing on one.
/// `sticky` (we were on the ground last step) lets the body follow the slope down instead of launching off it.
fn snap_to_slope(b: &mut Body, w: &dyn Collider, sticky: bool) -> i8 {
    if b.vel.y < 0.0 {
        return 0;
    }
    let cx = b.pos.x + PW / 2.0;
    let feet = b.pos.y + PH;
    let tx = (cx / TILE).floor() as i32;
    let ty0 = ((feet - 0.01) / TILE).floor() as i32;
    let tol = if sticky { 12.0 } else { 0.5 };
    for ty in [ty0, ty0 + 1] {
        if let Fx::Slope(dir) = w.fx(tx, ty) {
            let surf = slope_surface(dir, tx, ty, cx);
            if feet >= surf - tol && feet <= surf + TILE {
                b.pos.y = surf - PH;
                return dir;
            }
        }
    }
    0
}

/// One fixed-timestep update of the player body.
pub fn step_player(b: &mut Body, inp: &Input, dt: f32, w: &dyn Collider) -> StepEvents {
    let mut ev = StepEvents::default();

    // --- horizontal ---
    let max = if b.gliding {
        GLIDE_MAX_X
    } else if inp.run {
        RUN_MAX
    } else {
        WALK_MAX
    };
    let (acc, turn) = if b.on_ground { (ACCEL_GROUND, TURN_GROUND) } else { (ACCEL_AIR, TURN_AIR) };
    // crouching charges a super jump (you stop, but can still turn around)
    b.crouch = b.on_ground && inp.down && b.slope == 0 && b.pound == Pound::None;
    if b.crouch {
        let was = b.charge;
        b.charge += dt;
        ev.charged = was < SUPER_CHARGE && b.charge >= SUPER_CHARGE;
    } else {
        if b.charge >= SUPER_CHARGE && b.on_ground {
            b.super_grace = SUPER_GRACE;
        }
        b.charge = 0.0;
    }
    if !b.on_ground {
        b.super_grace = 0.0;
    }
    b.super_grace = (b.super_grace - dt).max(0.0);
    if b.pounding() {
        b.vel.x = 0.0;
    } else if b.crouch {
        if inp.dir != 0.0 {
            b.facing = inp.dir.signum();
        }
        b.vel.x = approach(b.vel.x, 0.0, FRICTION * dt);
    } else if b.slope != 0 {
        // on a slide you always go downhill; steering only changes how fast
        let s = b.slope as f32;
        let k = if inp.dir * s > 0.0 {
            1.15
        } else if inp.dir * s < 0.0 {
            0.6
        } else {
            1.0
        };
        b.vel.x = approach(b.vel.x, s * SLIDE_MAX * k, SLIDE_ACCEL * dt);
        b.facing = s;
    } else if inp.dir != 0.0 {
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
        if !b.on_ground && b.coyote <= 0.0 {
            b.glide_armed = true;
        }
    } else {
        b.jump_buf -= dt;
    }
    if !inp.jump_held {
        b.glide_armed = false;
    }
    if b.jump_buf > 0.0 && b.coyote > 0.0 && b.pound == Pound::None {
        let sup = b.on_ground && b.super_ready();
        b.vel.y = if sup { -SUPER_JUMP_V } else { -(JUMP_V + JUMP_RUN_BONUS * (b.vel.x.abs() / RUN_MAX).min(1.0)) };
        b.super_jumping = sup;
        ev.super_jump = sup;
        b.crouch = false;
        b.charge = 0.0;
        b.super_grace = 0.0;
        b.jumping = true;
        b.on_ground = false;
        b.coyote = 0.0;
        b.jump_buf = 0.0;
        b.plat = None;
        b.slope = 0;
        b.glide_armed = false;
        ev.jumped = true;
    }

    // --- ground pound ---
    if inp.pound_pressed && b.pound == Pound::None {
        if b.on_ground {
            b.vel.y = -POUND_HOP;
            b.boost = true; // full little hop whatever the jump button does
            b.on_ground = false;
            b.coyote = 0.0;
            b.jump_buf = 0.0;
            b.plat = None;
            b.slope = 0;
            b.pound = Pound::Hop;
        } else {
            b.pound = Pound::Windup(0.0);
            ev.pound_start = true;
        }
        b.crouch = false;
        b.charge = 0.0;
        b.super_jumping = false;
    }
    if b.pound == Pound::Hop && b.vel.y >= -40.0 {
        b.pound = Pound::Windup(0.0);
        ev.pound_start = true;
    }
    if b.pounding() {
        b.glide_armed = false;
        b.boost = false;
        b.jumping = false;
    }
    if b.super_jumping && b.vel.y >= 0.0 {
        b.super_jumping = false;
    }

    // --- gliding ---
    let was_gliding = b.gliding;
    b.gliding = b.glide_armed && !b.on_ground && b.vel.y >= 0.0;
    ev.glide_start = b.gliding && !was_gliding;

    // --- gravity / glide / wind ---
    let c = b.center();
    b.in_wind = w.wind((c.x / TILE).floor() as i32, (c.y / TILE).floor() as i32);
    if b.boost && b.vel.y >= 0.0 {
        b.boost = false;
    }
    let mut max_fall = MAX_FALL;
    if let Pound::Windup(t) = b.pound {
        b.vel = Vec2::ZERO;
        b.pound = if t + dt >= POUND_WINDUP { Pound::Slam } else { Pound::Windup(t + dt) };
    } else if b.pound == Pound::Slam {
        b.vel.y = POUND_V;
        max_fall = POUND_V;
    } else if b.gliding || (b.glide_armed && b.in_wind) {
        if b.in_wind {
            b.vel.y = approach(b.vel.y, -WIND_RISE, WIND_ACCEL * dt);
            b.gliding = true;
        } else {
            b.vel.y = approach(b.vel.y, GLIDE_FALL, GLIDE_DECEL * dt);
        }
    } else {
        let g = if b.vel.y < 0.0 {
            if (b.jumping && inp.jump_held) || b.boost {
                G_HOLD
            } else {
                G_CUT
            }
        } else {
            G_FALL
        };
        b.vel.y += g * dt;
        if b.in_wind {
            b.vel.y -= WIND_LIFT * dt;
        }
    }
    b.vel.y = b.vel.y.min(max_fall);

    // --- move & collide ---
    let size = vec2(PW, PH);
    if sweep_x(&mut b.pos, size, b.vel.x * dt, w, false).is_some() {
        b.vel.x = 0.0;
    }
    let was_ground = b.on_ground;
    let hit = sweep_y(&mut b.pos, size, b.vel.y * dt, w, true, true);
    let slope = snap_to_slope(b, w, was_ground);
    if hit.landed || slope != 0 {
        b.on_ground = true;
        b.jumping = false;
        b.boost = false;
        b.gliding = false;
        b.glide_armed = false;
        b.super_jumping = false;
        if b.pound == Pound::Slam {
            ev.pound_land = true;
        }
        b.pound = Pound::None;
        b.vel.y = 0.0;
        b.plat = if slope != 0 { None } else { hit.plat };
        b.slope = slope;
        ev.landed = !was_ground;
    } else {
        b.on_ground = false;
        b.plat = None;
        b.slope = 0;
    }
    if hit.bumped {
        b.vel.y = 0.0;
        b.jumping = false;
        b.boost = false;
        b.super_jumping = false;
        ev.bumped = true;
        ev.bump_tile = hit.tile;
    }

    // --- springs ---
    if b.on_ground && b.slope == 0 && b.plat.is_none() {
        let row = ((b.pos.y + PH + 0.5) / TILE).floor() as i32;
        let (c0, c1) = tile_range(b.pos.x, b.pos.x + PW);
        if let Some(c) = (c0..=c1).find(|&c| w.fx(c, row) == Fx::Spring) {
            b.vel.y = -SPRING_V;
            b.jumping = true;
            b.boost = true;
            b.on_ground = false;
            b.coyote = 0.0;
            ev.sprung = Some((c, row));
            ev.landed = false;
        }
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

    fn jump(run: bool, vx: f32, glide: bool) -> (f32, f32, f32) {
        let dt = 1.0 / 120.0;
        let mut b = Body::new(vec2(0.0, 10.0 * TILE - PH - 1.0));
        settle(&mut b, &Flat);
        b.vel.x = vx;
        let (y0, x0) = (b.pos.y, b.pos.x);
        let mut min_y = y0;
        let mut t = 0.0;
        let mut i = 0;
        loop {
            // glide: release after 0.4s, press again and hold
            let press_again = glide && i == 50;
            let held = if glide { i < 48 || i >= 50 } else { true };
            let inp = Input { dir: if vx > 0.0 { 1.0 } else { 0.0 }, run, jump_pressed: i == 0 || press_again, jump_held: held, ..Default::default() };
            step_player(&mut b, &inp, dt, &Flat);
            t += dt;
            i += 1;
            min_y = min_y.min(b.pos.y);
            if b.on_ground && t > 0.05 {
                break;
            }
        }
        ((y0 - min_y) / TILE, (b.pos.x - x0) / TILE, t)
    }

    /// Prints the jump envelope used to design the levels.
    #[test]
    fn jump_capabilities() {
        for (name, run, vx, glide) in [
            ("standing", false, 0.0, false),
            ("walk", false, WALK_MAX, false),
            ("run", true, RUN_MAX, false),
            ("run+glide", true, RUN_MAX, true),
        ] {
            let (h, d, t) = jump(run, vx, glide);
            println!("{name:10}: height {h:.2} tiles  distance {d:.2} tiles  airtime {t:.2}s");
        }
        let (_, d, _) = jump(true, RUN_MAX, true);
        assert!(d > 8.0, "gliding should cross wide gaps");
        let h = super_jump();
        println!("super jump: height {h:.2} tiles");
        assert!(h > 4.3 && h < 5.0, "super jump is about a tile higher than a normal jump ({h:.2})");
    }

    /// Crouch for 0.5 s, let go of down, then jump (inside the grace time). Returns the height in tiles.
    fn super_jump() -> f32 {
        let dt = 1.0 / 120.0;
        let mut b = Body::new(vec2(0.0, 10.0 * TILE - PH - 1.0));
        settle(&mut b, &Flat);
        let y0 = b.pos.y;
        let mut min_y = y0;
        let mut jumped = false;
        for i in 0..400 {
            let inp = Input { down: i < 60, jump_pressed: i == 70, jump_held: i >= 70, ..Default::default() };
            jumped |= step_player(&mut b, &inp, dt, &Flat).super_jump;
            min_y = min_y.min(b.pos.y);
            if b.on_ground && i > 75 {
                break;
            }
        }
        assert!(jumped, "the charge is kept for a moment after letting go of down");
        (y0 - min_y) / TILE
    }

    #[test]
    fn ground_pound_slams_down() {
        let dt = 1.0 / 120.0;
        let mut b = Body::new(vec2(0.0, 2.0 * TILE));
        let mut landed = false;
        let mut hung = 0;
        for i in 0..200 {
            let ev = step_player(&mut b, &Input { pound_pressed: i == 0, ..Default::default() }, dt, &Flat);
            if matches!(b.pound, Pound::Windup(_)) {
                hung += 1;
            }
            if ev.pound_land {
                landed = true;
                break;
            }
        }
        assert!(landed && b.on_ground);
        assert!(hung >= 15, "hangs in the air before slamming ({hung} steps)");
        // from the ground: hop first, then pound
        let mut events = vec![];
        for i in 0..200 {
            let ev = step_player(&mut b, &Input { pound_pressed: i == 0, ..Default::default() }, dt, &Flat);
            if ev.pound_land {
                events.push(i);
            }
        }
        assert_eq!(events.len(), 1, "a pound on the ground hops and then slams");
    }
}
