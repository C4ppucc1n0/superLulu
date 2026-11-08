//! ASCII level loading. See the legend at the top of `levels/level1.txt`.

use crate::physics::Coll;

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
}

impl Tile {
    pub fn coll(self) -> Coll {
        match self {
            Tile::Ground | Tile::Stone | Tile::Crumble | Tile::TurretL | Tile::TurretR => Coll::Solid,
            Tile::OneWay => Coll::OneWay,
            Tile::Bumper => Coll::Bumper,
            _ => Coll::Empty,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpawnKind {
    Orange,
    Golden,
    Shield,
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
}

impl Level {
    pub fn parse(src: &str) -> Level {
        // collect '---' blocks (comments start with ';')
        let mut blocks: Vec<Vec<Vec<char>>> = vec![];
        for line in src.lines() {
            if line.starts_with(';') {
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

        let mut lv = Level { w, h, tiles: vec![Tile::Empty; (w * h) as usize], spawns: vec![], start: (2, h - 4) };
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_level1() {
        let lv = Level::parse(include_str!("levels/level1.txt"));
        assert_eq!(lv.h, 20);
        assert_eq!(lv.w, 290);
        assert_eq!(lv.spawns.iter().filter(|s| s.kind == SpawnKind::Golden).count(), 3);
        assert_eq!(lv.spawns.iter().filter(|s| s.kind == SpawnKind::Goal).count(), 1);
        assert_eq!(lv.start, (3, 16));
    }
}
