//! Level sanity checker (tests only): explores a level with the real player physics and
//! verifies the goal, every checkpoint and every golden orange can be reached.
//!
//! Simplifications: enemies/turret bullets are ignored, crumbling blocks count as solid,
//! and moving platforms are replaced by one-way tiles along their whole path (optimistic).
//! Any spike touch or fall into a pit discards that jump, so the found routes never need
//! to take damage. Jumps include gliding, springs, updrafts and slides.

use std::collections::{HashMap, HashSet, VecDeque};

use macroquad::math::{vec2, Rect};

use crate::game::{checkpoint_rect, goal_rect, hurt_rect, spike_rect};
use crate::level::{Level, SpawnKind, Tile, LEVELS};
use crate::physics::*;

struct Static<'a> {
    lv: &'a Level,
    /// Cells covered by a moving platform's path (treated as one-way floors).
    extra: Vec<bool>,
}

impl Collider for Static<'_> {
    fn coll(&self, tx: i32, ty: i32) -> Coll {
        let on_path = self.lv.idx(tx, ty).is_some_and(|i| self.extra[i]);
        match self.lv.tile(tx, ty).coll() {
            Coll::Empty | Coll::Bumper if on_path => Coll::OneWay,
            Coll::Bumper => Coll::Empty,
            c => c,
        }
    }
    fn plats(&self) -> &[Rect] {
        &[]
    }
    fn fx(&self, tx: i32, ty: i32) -> Fx {
        self.lv.tile(tx, ty).fx()
    }
    fn wind(&self, tx: i32, ty: i32) -> bool {
        self.lv.wind_at(tx, ty)
    }
}

fn blocked(lv: &Level, tx: i32, ty: i32) -> bool {
    lv.tile(tx, ty) == Tile::Bumper || lv.tile(tx, ty).coll() == Coll::Solid
}

fn build(lv: &Level) -> Static<'_> {
    let mut extra = vec![false; lv.tiles.len()];
    let mut mark = |x: i32, y: i32| {
        if let Some(i) = lv.idx(x, y) {
            extra[i] = true;
        }
    };
    for s in &lv.spawns {
        match s.kind {
            SpawnKind::PlatH => {
                let mut l = s.tx;
                while !blocked(lv, l - 1, s.ty) {
                    l -= 1;
                }
                let mut r = s.tx + s.len - 1;
                while !blocked(lv, r + 1, s.ty) {
                    r += 1;
                }
                for x in l..=r {
                    mark(x, s.ty);
                }
            }
            SpawnKind::PlatV => {
                let cols = s.tx..s.tx + s.len;
                let mut top = s.ty;
                while !cols.clone().any(|c| blocked(lv, c, top - 1)) && top > 0 {
                    top -= 1;
                }
                let mut bot = s.ty;
                while !cols.clone().any(|c| blocked(lv, c, bot + 1)) && bot < lv.h {
                    bot += 1;
                }
                for y in top..=bot {
                    for c in cols.clone() {
                        mark(c, y);
                    }
                }
            }
            _ => {}
        }
    }
    Static { lv, extra }
}

/// A horizontal run of tiles the player can stand on.
#[derive(Debug, Clone, Copy)]
struct Surface {
    row: i32,
    c0: i32,
    c1: i32,
}

fn standable(w: &Static, tx: i32, ty: i32) -> bool {
    let free = |y: i32| w.coll(tx, y) != Coll::Solid && !matches!(w.lv.tile(tx, y), Tile::SpikeUp | Tile::SpikeDown);
    matches!(w.coll(tx, ty), Coll::Solid | Coll::OneWay) && w.fx(tx, ty) != Fx::Spring && free(ty - 1) && free(ty - 2)
}

fn surfaces(w: &Static) -> (Vec<Surface>, HashMap<(i32, i32), usize>) {
    let mut out = vec![];
    let mut map = HashMap::new();
    for ty in 0..w.lv.h {
        let mut tx = 0;
        while tx < w.lv.w {
            if standable(w, tx, ty) {
                let c0 = tx;
                while tx < w.lv.w && standable(w, tx, ty) {
                    map.insert((tx, ty), out.len());
                    tx += 1;
                }
                out.push(Surface { row: ty, c0, c1: tx - 1 });
            } else {
                tx += 1;
            }
        }
    }
    (out, map)
}

/// How the simulated player presses the buttons during one jump.
#[derive(Clone, Copy, Debug)]
struct Plan {
    /// Direction held during [d0, d1) and again from d2 on.
    d0: f32,
    d1: f32,
    d2: f32,
    jump: bool,
    /// Jump held until this time.
    hold: f32,
    /// Press jump again at this time (glide) and hold until `glide_until`.
    glide_at: f32,
    glide_until: f32,
}

impl Plan {
    fn input(&self, t: f32, dir: f32, run: bool, dt: f32) -> Input {
        let steer = (t >= self.d0 && t < self.d1) || t >= self.d2;
        let glide_press = self.glide_at >= 0.0 && t >= self.glide_at && t < self.glide_at + dt;
        let glide_hold = self.glide_at >= 0.0 && t >= self.glide_at && t < self.glide_until;
        Input {
            dir: if steer { dir } else { 0.0 },
            run,
            jump_pressed: (self.jump && t < dt * 0.5) || glide_press,
            jump_held: (self.jump && t < self.hold) || glide_hold,
            throw_pressed: false,
        }
    }
}

fn plans() -> Vec<Plan> {
    const NEVER: f32 = 99.0;
    let base = Plan { d0: 0.0, d1: NEVER, d2: NEVER, jump: false, hold: 0.0, glide_at: -1.0, glide_until: 0.0 };
    let mut v = vec![];
    // walk off a ledge (optionally gliding)
    for d1 in [0.12, 0.3, NEVER] {
        v.push(Plan { d1, ..base });
    }
    v.push(Plan { glide_at: 0.14, glide_until: NEVER, ..base });
    // plain jumps of different heights and lengths
    for hold in [0.0, 0.08, 0.16, 0.26, NEVER] {
        for d1 in [0.12, 0.3, NEVER] {
            v.push(Plan { d1, jump: true, hold, ..base });
        }
    }
    // full jump, then glide from the top
    let glide = Plan { jump: true, hold: 0.38, glide_at: 0.42, glide_until: NEVER, ..base };
    v.push(glide);
    v.push(Plan { glide_until: 1.0, ..glide });
    v.push(Plan { d1: 0.3, ..glide });
    // updraft rides: hop in, glide straight up, steer out later
    for d2 in [0.9, 1.6, 2.4] {
        v.push(Plan { d1: 0.15, d2, jump: true, hold: 0.12, glide_at: 0.2, glide_until: NEVER, ..base });
    }
    v
}

#[derive(Default)]
struct Found {
    goal: bool,
    checkpoints: HashSet<usize>,
    goldens: HashSet<(i32, i32)>,
    oranges: HashSet<(i32, i32)>,
}

fn explore(lv: &Level, start: (i32, i32)) -> (Found, usize, usize) {
    let w = build(lv);
    let (surf, map) = surfaces(&w);
    let goldens: Vec<(i32, i32)> = lv.spawns.iter().filter(|s| s.kind == SpawnKind::Golden).map(|s| (s.tx, s.ty)).collect();
    let oranges: Vec<(i32, i32)> = lv.spawns.iter().filter(|s| s.kind == SpawnKind::Orange).map(|s| (s.tx, s.ty)).collect();
    let goal = lv.spawns.iter().find(|s| s.kind == SpawnKind::Goal).map(|s| (s.tx, s.ty)).unwrap();
    let goal_r = goal_rect(goal.0, goal.1);
    let cps: Vec<Rect> = lv
        .spawns
        .iter()
        .filter(|s| s.kind == SpawnKind::Checkpoint)
        .map(|s| checkpoint_rect(vec2((s.tx as f32 + 0.5) * TILE, (s.ty + 1) as f32 * TILE)))
        .collect();
    let item_rect = |(tx, ty): (i32, i32), r: f32| {
        let c = vec2((tx as f32 + 0.5) * TILE, (ty as f32 + 0.5) * TILE);
        Rect::new(c.x - r, c.y - r, 2.0 * r, 2.0 * r)
    };
    let plans = plans();

    let mut found = Found::default();
    let first = *map.get(&(start.0, start.1 + 1)).unwrap_or_else(|| panic!("start {start:?} must stand on something"));
    let mut seen = vec![false; surf.len()];
    seen[first] = true;
    let mut q = VecDeque::from([first]);
    let dt = 1.0 / 120.0;
    let mut sims = 0usize;

    while let Some(si) = q.pop_front() {
        let s = surf[si];
        let top = s.row as f32 * TILE;
        let len = s.c1 - s.c0 + 1;
        let mut xs = vec![s.c0 as f32 * TILE - PW + 3.0, (s.c1 + 1) as f32 * TILE - 3.0];
        for c in s.c0..=s.c1 {
            xs.push(c as f32 * TILE);
            xs.push((c + 1) as f32 * TILE - PW);
        }
        let speeds: &[f32] = if len >= 2 { &[0.0, WALK_MAX, RUN_MAX] } else { &[0.0, WALK_MAX] };
        for &x in &xs {
            for &dir in &[-1.0f32, 1.0] {
                for &v in speeds {
                    for plan in &plans {
                        sims += 1;
                        let mut b = Body::new(vec2(x, top - PH));
                        b.on_ground = true;
                        b.vel.x = v * dir;
                        let run = v > WALK_MAX;
                        let mut t = 0.0;
                        let mut ok = true;
                        loop {
                            let inp = plan.input(t, dir, run, dt);
                            step_player(&mut b, &inp, dt, &w);
                            t += dt;
                            let pr = b.rect();
                            let hr = hurt_rect(&b);
                            let (c0, c1) = ((hr.x / TILE).floor() as i32, ((hr.x + hr.w) / TILE).floor() as i32);
                            let (r0, r1) = ((hr.y / TILE).floor() as i32, ((hr.y + hr.h) / TILE).floor() as i32);
                            'spikes: for ty in r0..=r1 {
                                for tx in c0..=c1 {
                                    if let Some(sr) = spike_rect(lv.tile(tx, ty), tx, ty) {
                                        if sr.overlaps(&hr) {
                                            ok = false;
                                            break 'spikes;
                                        }
                                    }
                                }
                            }
                            if !ok || b.pos.y > lv.h as f32 * TILE + 40.0 || t > 5.0 {
                                ok = false;
                                break;
                            }
                            if pr.overlaps(&goal_r) {
                                found.goal = true;
                            }
                            for (i, c) in cps.iter().enumerate() {
                                if c.overlaps(&pr) {
                                    found.checkpoints.insert(i);
                                }
                            }
                            for g in &goldens {
                                if item_rect(*g, 12.0).overlaps(&pr) {
                                    found.goldens.insert(*g);
                                }
                            }
                            for o in &oranges {
                                if item_rect(*o, 9.0).overlaps(&pr) {
                                    found.oranges.insert(*o);
                                }
                            }
                            if b.on_ground && b.slope == 0 && t > 0.02 {
                                break;
                            }
                        }
                        if !ok {
                            continue;
                        }
                        let row = ((b.pos.y + PH) / TILE).round() as i32;
                        let cl = (b.pos.x / TILE).floor() as i32;
                        let cr = ((b.pos.x + PW - 0.01) / TILE).floor() as i32;
                        for c in [cl, cr] {
                            if let Some(&ni) = map.get(&(c, row)) {
                                if !seen[ni] {
                                    seen[ni] = true;
                                    q.push_back(ni);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let visited = seen.iter().filter(|s| **s).count();
    (found, visited, sims)
}

fn check_level(n: usize) {
    let lv = Level::parse(LEVELS[n - 1]);
    let (found, visited, sims) = explore(&lv, lv.start);
    println!("level {n} ({}): visited {visited} surfaces with {sims} simulated jumps", lv.name);
    let total_oranges = lv.spawns.iter().filter(|s| s.kind == SpawnKind::Orange).count();
    let missing: Vec<_> = lv
        .spawns
        .iter()
        .filter(|s| s.kind == SpawnKind::Orange && !found.oranges.contains(&(s.tx, s.ty)))
        .map(|s| (s.tx, s.ty))
        .collect();
    println!("  oranges reachable: {}/{}  unreachable: {:?}", found.oranges.len(), total_oranges, missing);
    let goldens: Vec<_> = lv.spawns.iter().filter(|s| s.kind == SpawnKind::Golden).map(|s| (s.tx, s.ty)).collect();
    let lost: Vec<_> = goldens.iter().filter(|g| !found.goldens.contains(g)).collect();
    println!("  goldens reachable: {}/3  unreachable: {lost:?}", found.goldens.len());
    let cps: Vec<_> = lv.spawns.iter().filter(|s| s.kind == SpawnKind::Checkpoint).map(|s| (s.tx, s.ty)).collect();
    let mut problems = vec![];
    if !found.goal {
        problems.push("goal not reachable from start".to_string());
    }
    for (i, cp) in cps.iter().enumerate() {
        if !found.checkpoints.contains(&i) {
            problems.push(format!("checkpoint {cp:?} not reachable from start"));
        }
    }
    if !lost.is_empty() {
        problems.push(format!("golden oranges unreachable: {lost:?}"));
    }
    if !missing.is_empty() {
        problems.push(format!("oranges unreachable: {missing:?}"));
    }
    for cp in &cps {
        let (f, v, _) = explore(&lv, *cp);
        println!("  from checkpoint {cp:?}: visited {v} surfaces, goal {}", f.goal);
        if !f.goal {
            problems.push(format!("goal not reachable from checkpoint {cp:?}"));
        }
    }
    assert!(problems.is_empty(), "level {n}: {problems:#?}");
}

#[test]
fn level1_is_beatable() {
    check_level(1);
}

#[test]
fn level2_is_beatable() {
    check_level(2);
}

#[test]
fn level3_is_beatable() {
    check_level(3);
}
