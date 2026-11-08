//! Level sanity checker (tests only): explores the level with the real player physics and
//! verifies the goal, checkpoint and every golden orange can be reached.
//!
//! Simplifications: enemies/turret bullets are ignored, crumbling blocks count as solid,
//! and moving platforms are replaced by one-way tiles along their whole path (optimistic).
//! Any spike touch or fall into a pit discards that jump.

use std::collections::{HashSet, VecDeque};

use macroquad::math::{vec2, Rect};

use crate::game::{hurt_rect, spike_rect};
use crate::level::{Level, SpawnKind, Tile};
use crate::physics::*;

struct Static<'a> {
    lv: &'a Level,
    extra: HashSet<(i32, i32)>,
}

impl Collider for Static<'_> {
    fn coll(&self, tx: i32, ty: i32) -> Coll {
        match self.lv.tile(tx, ty).coll() {
            Coll::Empty | Coll::Bumper if self.extra.contains(&(tx, ty)) => Coll::OneWay,
            Coll::Bumper => Coll::Empty,
            c => c,
        }
    }
    fn plats(&self) -> &[Rect] {
        &[]
    }
}

fn blocked(lv: &Level, tx: i32, ty: i32) -> bool {
    lv.tile(tx, ty) == Tile::Bumper || lv.tile(tx, ty).coll() == Coll::Solid
}

fn build(lv: &Level) -> Static<'_> {
    let mut extra = HashSet::new();
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
                    extra.insert((x, s.ty));
                }
            }
            SpawnKind::PlatV => {
                let cols = s.tx..s.tx + s.len;
                let mut top = s.ty;
                while !cols.clone().any(|c| blocked(lv, c, top - 1)) {
                    top -= 1;
                }
                let mut bot = s.ty;
                while !cols.clone().any(|c| blocked(lv, c, bot + 1)) && bot < lv.h {
                    bot += 1;
                }
                for y in top..=bot {
                    for c in cols.clone() {
                        extra.insert((c, y));
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
    matches!(w.coll(tx, ty), Coll::Solid | Coll::OneWay) && free(ty - 1) && free(ty - 2)
}

fn surfaces(w: &Static) -> (Vec<Surface>, std::collections::HashMap<(i32, i32), usize>) {
    let mut out = vec![];
    let mut map = std::collections::HashMap::new();
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

#[derive(Default)]
struct Found {
    goal: bool,
    checkpoint: bool,
    goldens: HashSet<(i32, i32)>,
    oranges: HashSet<(i32, i32)>,
}

fn explore(lv: &Level, start: (i32, i32)) -> (Found, usize, usize) {
    let w = build(lv);
    let (surf, map) = surfaces(&w);
    let goldens: Vec<(i32, i32)> = lv.spawns.iter().filter(|s| s.kind == SpawnKind::Golden).map(|s| (s.tx, s.ty)).collect();
    let oranges: Vec<(i32, i32)> = lv.spawns.iter().filter(|s| s.kind == SpawnKind::Orange).map(|s| (s.tx, s.ty)).collect();
    let goal = lv.spawns.iter().find(|s| s.kind == SpawnKind::Goal).map(|s| (s.tx, s.ty)).unwrap();
    let goal_rect = Rect::new(goal.0 as f32 * TILE + 30.0, (goal.1 - 3) as f32 * TILE, 36.0, 4.0 * TILE);
    let cps: Vec<Rect> = lv
        .spawns
        .iter()
        .filter(|s| s.kind == SpawnKind::Checkpoint)
        .map(|s| Rect::new((s.tx as f32 + 0.5) * TILE - 14.0, (s.ty - 1) as f32 * TILE, 28.0, 2.0 * TILE))
        .collect();
    let item_rect = |(tx, ty): (i32, i32), r: f32| {
        let c = vec2((tx as f32 + 0.5) * TILE, (ty as f32 + 0.5) * TILE);
        Rect::new(c.x - r, c.y - r, 2.0 * r, 2.0 * r)
    };

    let mut found = Found::default();
    let first = *map.get(&(start.0, start.1 + 1)).expect("start must stand on something");
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
                    for &dir_time in &[0.12f32, 0.3, 9.0] {
                        for &hold in &[-1.0f32, 0.0, 0.08, 0.16, 0.26, 9.0] {
                            sims += 1;
                            let mut b = Body::new(vec2(x, top - PH));
                            b.on_ground = true;
                            b.vel.x = v * dir;
                            let run = v > WALK_MAX;
                            let mut t = 0.0;
                            let mut ok = true;
                            let mut i = 0;
                            loop {
                                let inp = Input {
                                    dir: if t < dir_time { dir } else { 0.0 },
                                    run,
                                    jump_pressed: hold >= 0.0 && i == 0,
                                    jump_held: hold >= 0.0 && t < hold,
                                };
                                step_player(&mut b, &inp, dt, &w);
                                t += dt;
                                i += 1;
                                let pr = b.rect();
                                let hr = hurt_rect(&b);
                                let (c0, c1) = ((hr.x / TILE).floor() as i32, ((hr.x + hr.w) / TILE).floor() as i32);
                                let (r0, r1) = ((hr.y / TILE).floor() as i32, ((hr.y + hr.h) / TILE).floor() as i32);
                                for ty in r0..=r1 {
                                    for tx in c0..=c1 {
                                        if let Some(sr) = spike_rect(lv.tile(tx, ty), tx, ty) {
                                            if sr.overlaps(&hr) {
                                                ok = false;
                                            }
                                        }
                                    }
                                }
                                if !ok || b.pos.y > lv.h as f32 * TILE + 40.0 || t > 3.0 {
                                    ok = false;
                                    break;
                                }
                                if pr.overlaps(&goal_rect) {
                                    found.goal = true;
                                }
                                if cps.iter().any(|c| c.overlaps(&pr)) {
                                    found.checkpoint = true;
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
                                if b.on_ground && t > 0.02 {
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
    }
    let visited = seen.iter().filter(|s| **s).count();
    (found, visited, sims)
}

#[test]
fn level1_is_beatable() {
    let lv = Level::parse(include_str!("levels/level1.txt"));
    let (found, visited, sims) = explore(&lv, lv.start);
    println!("from start: visited {visited} surfaces with {sims} simulated jumps");
    let total_oranges = lv.spawns.iter().filter(|s| s.kind == SpawnKind::Orange).count();
    let missing: Vec<_> = lv
        .spawns
        .iter()
        .filter(|s| s.kind == SpawnKind::Orange && !found.oranges.contains(&(s.tx, s.ty)))
        .map(|s| (s.tx, s.ty))
        .collect();
    println!("oranges reachable: {}/{}  unreachable: {:?}", found.oranges.len(), total_oranges, missing);
    println!("goldens reachable: {}/3", found.goldens.len());
    assert!(found.checkpoint, "checkpoint not reachable from start");
    assert!(found.goal, "goal not reachable from start");
    assert_eq!(found.goldens.len(), 3, "a golden orange is unreachable");

    let cp = lv.spawns.iter().find(|s| s.kind == SpawnKind::Checkpoint).unwrap();
    let (found2, visited2, _) = explore(&lv, (cp.tx, cp.ty));
    println!("from checkpoint: visited {visited2} surfaces");
    assert!(found2.goal, "goal not reachable from checkpoint");
}
