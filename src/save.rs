//! Level progress (cleared levels, best times, golden oranges), stored as a small text file in
//! `%APPDATA%\SuperLulu\progress.txt`.

use std::path::PathBuf;

pub const LEVEL_COUNT: usize = 4;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct LevelSave {
    pub cleared: bool,
    pub best_time: Option<f32>,
    /// Bit i set = golden orange i has been found (in any run).
    pub goldens: u8,
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Progress {
    pub levels: [LevelSave; LEVEL_COUNT],
}

impl Progress {
    fn path() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("SuperLulu").join("progress.txt"))
    }

    pub fn load() -> Progress {
        Self::path().and_then(|p| std::fs::read_to_string(p).ok()).map(|s| Self::parse(&s)).unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = Self::path() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(p, self.to_text());
        }
    }

    pub fn parse(s: &str) -> Progress {
        let mut p = Progress::default();
        for line in s.lines() {
            let mut lv: Option<usize> = None;
            let mut save = LevelSave::default();
            for tok in line.split_whitespace() {
                let Some((k, v)) = tok.split_once('=') else { continue };
                match k {
                    "level" => lv = v.parse::<usize>().ok().filter(|n| (1..=LEVEL_COUNT).contains(n)).map(|n| n - 1),
                    "cleared" => save.cleared = v == "1",
                    "best" => save.best_time = v.parse::<f32>().ok().filter(|t| *t > 0.0),
                    "goldens" => save.goldens = v.parse().unwrap_or(0) & 0b111,
                    _ => {}
                }
            }
            if let Some(i) = lv {
                p.levels[i] = save;
            }
        }
        p
    }

    pub fn to_text(&self) -> String {
        let mut s = String::from("# Super Lulu progress\n");
        for (i, l) in self.levels.iter().enumerate() {
            let best = l.best_time.map(|t| format!("{t:.2}")).unwrap_or_else(|| "0".into());
            s += &format!("level={} cleared={} best={} goldens={}\n", i + 1, l.cleared as u8, best, l.goldens);
        }
        s
    }

    pub fn unlocked(&self, i: usize) -> bool {
        i == 0 || self.levels.get(i - 1).is_some_and(|l| l.cleared)
    }

    /// Record a finished run. Returns true if it set a new best time.
    pub fn record(&mut self, i: usize, time: f32, golden_mask: u8) -> bool {
        let l = &mut self.levels[i];
        l.cleared = true;
        l.goldens |= golden_mask;
        let best = l.best_time.is_none_or(|b| time < b);
        if best {
            l.best_time = Some(time);
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_unlocks() {
        let mut p = Progress::default();
        assert!(p.unlocked(0) && !p.unlocked(1));
        assert!(p.record(0, 95.5, 0b101));
        assert!(!p.record(0, 120.0, 0b010), "slower run is not a best");
        assert_eq!(p.levels[0].goldens, 0b111);
        assert!(p.unlocked(1) && !p.unlocked(2));
        let q = Progress::parse(&p.to_text());
        assert_eq!(q, p);
        assert_eq!(Progress::parse("garbage\nlevel=9 cleared=1"), Progress::default());
    }
}
