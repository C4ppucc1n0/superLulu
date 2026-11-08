//! ASCII level loading. See the legend at the top of `levels/level1.txt`.

use crate::physics::{Coll, Fx};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    Empty,
    Ground,
    Stone,
    OneWay,
    Crumble,
    SpikeUp,
    SpikeDown,
    TurretL,
    TurretR,
    Bumper,
    /// Jelly bounce pad.
    Spring,
    /// Blows an updraft straight up.
    Fan,
    /// Cookie box with oranges / with a heart (bump it from below or hit it with an orange).
    BoxOrange,
    BoxHeart,
    /// Looks like ground but you can walk through it (hidden rooms).
    Fake,
    /// Slides: `\` goes down to the right, `/` goes down to the left.
    SlideR,
    SlideL,
    /// Signpost 1-9 (text comes from `@signN` lines).
    Sign(u8),
    /// Cracked cookie floor: solid until you ground-pound it.
    Cracked,
    /// Stone pedestal that marks a super-jump spot (plain solid block).
    Pedestal,
}

impl Tile {
    pub fn coll(self) -> Coll {
        match self {
            Tile::Ground
            | Tile::Stone
            | Tile::Crumble
            | Tile::TurretL
            | Tile::TurretR
            | Tile::Spring
            | Tile::Fan
            | Tile::BoxOrange
            | Tile::BoxHeart
            | Tile::Cracked
            | Tile::Pedestal => Coll::Solid,
            Tile::OneWay => Coll::OneWay,
            Tile::Bumper => Coll::Bumper,
            _ => Coll::Empty,
        }
    }
    pub fn fx(self) -> Fx {
        match self {
            Tile::Spring => Fx::Spring,
            Tile::SlideR => Fx::Slope(1),
            Tile::SlideL => Fx::Slope(-1),
            _ => Fx::None,
        }
    }
    pub fn is_box(self) -> bool {
        matches!(self, Tile::BoxOrange | Tile::BoxHeart)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Meadow,
    Factory,
    Sky,
    /// The boss arena: dark storm clouds (clears up when the boss is beaten).
    Storm,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpawnKind {
    Orange,
    Golden,
    Shield,
    Heart,
    Checkpoint,
    Goal,
    Jelly,
    Hedgehog,
    Frog,
    PlatH,
    PlatV,
}

#[derive(Clone, Copy, Debug)]
pub struct Spawn {
    pub kind: SpawnKind,
    pub tx: i32,
    pub ty: i32,
    /// Width in tiles (moving platforms only).
    pub len: i32,
}

pub struct Level {
    pub w: i32,
    pub h: i32,
    pub tiles: Vec<Tile>,
    pub spawns: Vec<Spawn>,
    pub start: (i32, i32),
    pub name: String,
    pub theme: Theme,
    pub signs: Vec<String>,
    /// Updraft cells above fans.
    pub wind: Vec<bool>,
    /// Hidden-room group of each fake tile (-1 = not fake).
    pub fake_group: Vec<i32>,
    pub fake_groups: usize,
    /// Boss arena (`@boss`): the goal only opens once the boss is beaten.
    pub boss: bool,
}

impl Level {
    pub fn parse(src: &str) -> Level {
        // collect '---' blocks (comments start with ';', metadata with '@')
        let mut blocks: Vec<Vec<Vec<char>>> = vec![];
        let mut name = String::from("Level");
        let mut theme = Theme::Meadow;
        let mut signs = vec![String::new(); 9];
        let mut wind_h = 8;
        let mut boss = false;
        for line in src.lines() {
            if line.starts_with(';') {
                continue;
            }
            if let Some(meta) = line.strip_prefix('@') {
                let (key, val) = meta.split_once(' ').unwrap_or((meta, ""));
                let val = val.trim();
                match key {
                    "name" => name = val.to_string(),
                    "theme" => {
                        theme = match val {
                            "factory" => Theme::Factory,
                            "sky" => Theme::Sky,
                            "storm" => Theme::Storm,
                            _ => Theme::Meadow,
                        }
                    }
                    "wind" => wind_h = val.parse().unwrap_or(8),
                    "boss" => boss = true,
                    k if k.starts_with("sign") => {
                        if let Ok(n) = k[4..].parse::<usize>() {
                            if (1..=9).contains(&n) {
                                signs[n - 1] = val.to_string();
                            }
                        }
                    }
                    _ => {}
                }
                continue;
            }
            if line.starts_with("---") {
                blocks.push(vec![]);
                continue;
            }
            if blocks.is_empty() {
                blocks.push(vec![]);
            }
            blocks.last_mut().unwrap().push(line.chars().collect());
        }
        blocks.retain(|b| !b.is_empty());
        let h = blocks.iter().map(|b| b.len()).max().unwrap_or(0).max(12) as i32;
        let w: i32 = blocks.iter().map(|b| b.iter().map(|r| r.len()).max().unwrap_or(0) as i32).sum();

        let mut grid = vec![' '; (w * h) as usize];
        let mut x0 = 0;
        for b in &blocks {
            let bw = b.iter().map(|r| r.len()).max().unwrap_or(0) as i32;
            let y0 = h - b.len() as i32; // bottom-aligned
            for (ry, row) in b.iter().enumerate() {
                for (rx, &ch) in row.iter().enumerate() {
                    grid[((y0 + ry as i32) * w + x0 + rx as i32) as usize] = ch;
                }
            }
            x0 += bw;
        }

        let n = (w * h) as usize;
        let mut lv = Level {
            w,
            h,
            tiles: vec![Tile::Empty; n],
            spawns: vec![],
            start: (2, h - 4),
            name,
            theme,
            signs,
            wind: vec![false; n],
            fake_group: vec![-1; n],
            fake_groups: 0,
            boss,
        };
        for y in 0..h {
            let mut x = 0;
            while x < w {
                let ch = grid[(y * w + x) as usize];
                let mut spawn = |kind| lv.spawns.push(Spawn { kind, tx: x, ty: y, len: 1 });
                let tile = match ch {
                    '#' => Tile::Ground,
                    'X' => Tile::Stone,
                    '=' => Tile::OneWay,
                    'C' => Tile::Crumble,
                    '^' => Tile::SpikeUp,
                    'v' => Tile::SpikeDown,
                    '<' => Tile::TurretL,
                    '>' => Tile::TurretR,
                    '|' => Tile::Bumper,
                    'T' => Tile::Spring,
                    'U' => Tile::Fan,
                    'Q' => Tile::BoxOrange,
                    'R' => Tile::BoxHeart,
                    '%' => Tile::Fake,
                    '\\' => Tile::SlideR,
                    '/' => Tile::SlideL,
                    '1'..='9' => Tile::Sign(ch as u8 - b'0'),
                    'B' => Tile::Cracked,
                    'Y' => Tile::Pedestal,
                    'o' => {
                        spawn(SpawnKind::Orange);
                        Tile::Empty
                    }
                    '*' => {
                        spawn(SpawnKind::Golden);
                        Tile::Empty
                    }
                    'S' => {
                        spawn(SpawnKind::Shield);
                        Tile::Empty
                    }
                    '+' => {
                        spawn(SpawnKind::Heart);
                        Tile::Empty
                    }
                    'K' => {
                        spawn(SpawnKind::Checkpoint);
                        Tile::Empty
                    }
                    'G' => {
                        spawn(SpawnKind::Goal);
                        Tile::Empty
                    }
                    'e' => {
                        spawn(SpawnKind::Jelly);
                        Tile::Empty
                    }
                    'h' => {
                        spawn(SpawnKind::Hedgehog);
                        Tile::Empty
                    }
                    'f' => {
                        spawn(SpawnKind::Frog);
                        Tile::Empty
                    }
                    'P' => {
                        lv.start = (x, y);
                        Tile::Empty
                    }
                    'H' | 'V' => {
                        let mut len = 1;
                        while x + len < w && grid[(y * w + x + len) as usize] == ch {
                            len += 1;
                        }
                        let kind = if ch == 'H' { SpawnKind::PlatH } else { SpawnKind::PlatV };
                        lv.spawns.push(Spawn { kind, tx: x, ty: y, len });
                        x += len;
                        continue;
                    }
                    _ => Tile::Empty,
                };
                lv.tiles[(y * w + x) as usize] = tile;
                x += 1;
            }
        }

        // items inside a secret room stay hidden: a spawn cell with fake ground above or below it
        // becomes fake ground too (so the room doesn't show holes where its oranges are)
        let spawn_cells: Vec<usize> = lv.spawns.iter().filter(|s| !matches!(s.kind, SpawnKind::PlatH | SpawnKind::PlatV)).filter_map(|s| lv.idx(s.tx, s.ty)).collect();
        loop {
            let mut changed = false;
            for &i in &spawn_cells {
                let (x, y) = ((i as i32) % w, (i as i32) / w);
                if lv.tiles[i] == Tile::Empty && (lv.tile(x, y - 1) == Tile::Fake || lv.tile(x, y + 1) == Tile::Fake) {
                    lv.tiles[i] = Tile::Fake;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        // updrafts: straight up from each fan until something solid
        for y in 0..h {
            for x in 0..w {
                if lv.tile(x, y) == Tile::Fan {
                    for k in 1..=wind_h {
                        if lv.tile(x, y - k).coll() == Coll::Solid || y - k < 0 {
                            break;
                        }
                        let i = lv.idx(x, y - k).unwrap();
                        lv.wind[i] = true;
                    }
                }
            }
        }

        // hidden rooms: connected groups of fake tiles are revealed together
        for start in 0..n {
            if lv.tiles[start] != Tile::Fake || lv.fake_group[start] >= 0 {
                continue;
            }
            let g = lv.fake_groups as i32;
            lv.fake_groups += 1;
            let mut stack = vec![start];
            lv.fake_group[start] = g;
            while let Some(i) = stack.pop() {
                let (x, y) = ((i as i32) % w, (i as i32) / w);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    if let Some(j) = lv.idx(x + dx, y + dy) {
                        if lv.tiles[j] == Tile::Fake && lv.fake_group[j] < 0 {
                            lv.fake_group[j] = g;
                            stack.push(j);
                        }
                    }
                }
            }
        }
        lv
    }

    /// Outside the level: solid walls left/right, open sky above, bottomless below.
    pub fn tile(&self, x: i32, y: i32) -> Tile {
        if x < 0 || x >= self.w {
            return Tile::Stone;
        }
        if y < 0 || y >= self.h {
            return Tile::Empty;
        }
        self.tiles[(y * self.w + x) as usize]
    }

    pub fn idx(&self, x: i32, y: i32) -> Option<usize> {
        (x >= 0 && x < self.w && y >= 0 && y < self.h).then(|| (y * self.w + x) as usize)
    }

    pub fn wind_at(&self, x: i32, y: i32) -> bool {
        self.idx(x, y).is_some_and(|i| self.wind[i])
    }

    pub fn sign_text(&self, n: u8) -> &str {
        self.signs.get(n as usize - 1).map(|s| s.as_str()).unwrap_or("")
    }
}

/// The levels, in order (the last one is the boss fight).
pub const LEVELS: [&str; 4] = [
    include_str!("levels/level1.txt"),
    include_str!("levels/level2.txt"),
    include_str!("levels/level3.txt"),
    include_str!("levels/level4.txt"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_levels() {
        for (i, src) in LEVELS.iter().enumerate() {
            let lv = Level::parse(src);
            assert_eq!(lv.spawns.iter().filter(|s| s.kind == SpawnKind::Goal).count(), 1, "level {}", i + 1);
            if lv.boss {
                assert_eq!(i, LEVELS.len() - 1, "the boss is the last level");
                continue;
            }
            assert_eq!(lv.spawns.iter().filter(|s| s.kind == SpawnKind::Golden).count(), 3, "level {}", i + 1);
            assert!(lv.spawns.iter().filter(|s| s.kind == SpawnKind::Checkpoint).count() >= 2, "level {}", i + 1);
            assert!(!lv.tiles.contains(&Tile::SpikeDown), "no ceiling spikes in level {}", i + 1);
            assert!(lv.name != "Level");
        }
    }

    #[test]
    fn metadata_wind_and_hidden_rooms() {
        let src = "@name Test\n@theme sky\n@sign1 Hello there\n@wind 3\n   %%  \n 1 %%  \n  U    \n#######";
        let lv = Level::parse(src);
        assert_eq!(lv.name, "Test");
        assert_eq!(lv.theme, Theme::Sky);
        assert_eq!(lv.sign_text(1), "Hello there");
        let fan_y = lv.h - 2;
        assert!(lv.wind_at(2, fan_y - 1) && lv.wind_at(2, fan_y - 3) && !lv.wind_at(2, fan_y - 4));
        assert_eq!(lv.fake_groups, 1);
        assert_eq!(lv.tile(1, fan_y - 1), Tile::Sign(1));
    }
}
