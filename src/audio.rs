//! Chiptune-style sound effects and an original background loop, synthesized at startup
//! (no audio files needed, everything stays inside the exe).

use std::collections::HashMap;

use macroquad::audio::{load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound};

use crate::game::Sfx;

const SR: f32 = 44100.0;

#[derive(Clone, Copy)]
enum Wave {
    Square(f32),
    Tri,
    Sine,
    /// sample-and-hold noise at the given rate
    Noise,
}

struct Synth {
    buf: Vec<f32>,
    rng: u32,
}

fn midi(n: i32) -> f32 {
    440.0 * 2f32.powf((n - 69) as f32 / 12.0)
}

impl Synth {
    fn new(seconds: f32) -> Self {
        Synth { buf: vec![0.0; (seconds * SR) as usize], rng: 0x9E37_79B9 }
    }

    fn noise_sample(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng as f32 / u32::MAX as f32) * 2.0 - 1.0
    }

    /// Tone with exponential pitch slide f0 -> f1, linear attack and release.
    #[allow(clippy::too_many_arguments)]
    fn tone(&mut self, start: f32, dur: f32, f0: f32, f1: f32, wave: Wave, vol: f32, attack: f32, release: f32, vibrato: f32) {
        let n0 = (start * SR) as usize;
        let n = (dur * SR) as usize;
        let (na, nr) = ((attack * SR).max(1.0), (release * SR).max(1.0));
        let mut phase = 0.0f32;
        let mut held = 0.0f32;
        for i in 0..n {
            let k = i as f32 / n as f32;
            let tsec = i as f32 / SR;
            let f = f0 * (f1 / f0).powf(k) * (1.0 + vibrato * (tsec * std::f32::consts::TAU * 7.0).sin());
            let prev = phase;
            phase += f / SR;
            let s = match wave {
                Wave::Square(d) => {
                    if phase.fract() < d {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Wave::Tri => 4.0 * (phase.fract() - 0.5).abs() - 1.0,
                Wave::Sine => (phase * std::f32::consts::TAU).sin(),
                Wave::Noise => {
                    if phase.floor() != prev.floor() || i == 0 {
                        held = self.noise_sample();
                    }
                    held
                }
            };
            let env = (i as f32 / na).min(1.0) * ((n - i) as f32 / nr).min(1.0);
            if let Some(b) = self.buf.get_mut(n0 + i) {
                *b += s * vol * env;
            }
        }
    }

    fn note(&mut self, start: f32, dur: f32, n: i32, wave: Wave, vol: f32) {
        let f = midi(n);
        self.tone(start, dur, f, f, wave, vol, 0.004, dur.min(0.06), 0.0);
    }

    fn wav(&self) -> Vec<u8> {
        let n = self.buf.len();
        let peak = self.buf.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(1.0);
        let mut v = Vec::with_capacity(44 + n * 2);
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&((36 + n * 2) as u32).to_le_bytes());
        v.extend_from_slice(b"WAVEfmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes()); // PCM
        v.extend_from_slice(&1u16.to_le_bytes()); // mono
        v.extend_from_slice(&(SR as u32).to_le_bytes());
        v.extend_from_slice(&((SR as u32) * 2).to_le_bytes());
        v.extend_from_slice(&2u16.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&((n * 2) as u32).to_le_bytes());
        for s in &self.buf {
            v.extend_from_slice(&(((s / peak) * 30000.0) as i16).to_le_bytes());
        }
        v
    }
}

fn make_sfx(s: Sfx) -> Synth {
    use Wave::*;
    match s {
        Sfx::Jump => {
            let mut y = Synth::new(0.16);
            y.tone(0.0, 0.15, 290.0, 720.0, Square(0.25), 0.45, 0.004, 0.07, 0.0);
            y
        }
        Sfx::Land => {
            let mut y = Synth::new(0.07);
            y.tone(0.0, 0.06, 150.0, 60.0, Tri, 0.35, 0.002, 0.04, 0.0);
            y
        }
        Sfx::Stomp => {
            let mut y = Synth::new(0.14);
            y.tone(0.0, 0.12, 720.0, 150.0, Square(0.5), 0.35, 0.002, 0.05, 0.0);
            y.tone(0.0, 0.06, 5000.0, 5000.0, Noise, 0.2, 0.001, 0.05, 0.0);
            y
        }
        Sfx::Coin => {
            let mut y = Synth::new(0.24);
            y.tone(0.0, 0.05, midi(91), midi(91), Tri, 0.5, 0.002, 0.01, 0.0);
            y.tone(0.05, 0.18, midi(96), midi(96), Tri, 0.5, 0.002, 0.14, 0.0);
            y.tone(0.05, 0.12, midi(84), midi(84), Square(0.125), 0.08, 0.002, 0.1, 0.0);
            y
        }
        Sfx::Golden => {
            let mut y = Synth::new(0.8);
            for (i, n) in [84, 88, 91, 96, 100].iter().enumerate() {
                let len = if i == 4 { 0.4 } else { 0.07 };
                y.tone(i as f32 * 0.07, len, midi(*n), midi(*n), Square(0.25), 0.18, 0.003, len * 0.6, 0.0);
                y.tone(i as f32 * 0.07, len, midi(*n), midi(*n), Tri, 0.3, 0.003, len * 0.6, 0.0);
            }
            y
        }
        Sfx::Shield => {
            let mut y = Synth::new(0.4);
            y.tone(0.0, 0.38, 500.0, 1400.0, Sine, 0.4, 0.01, 0.15, 0.02);
            y.tone(0.06, 0.3, 1000.0, 2100.0, Tri, 0.15, 0.01, 0.15, 0.0);
            y
        }
        Sfx::ShieldPop => {
            let mut y = Synth::new(0.25);
            y.tone(0.0, 0.2, 9000.0, 3000.0, Noise, 0.3, 0.001, 0.15, 0.0);
            y.tone(0.0, 0.2, 900.0, 200.0, Square(0.5), 0.2, 0.001, 0.12, 0.0);
            y
        }
        Sfx::Die => {
            let mut y = Synth::new(0.95);
            y.tone(0.0, 0.09, 880.0, 880.0, Square(0.5), 0.25, 0.002, 0.02, 0.0);
            y.tone(0.13, 0.75, 700.0, 110.0, Square(0.25), 0.28, 0.005, 0.3, 0.025);
            y
        }
        Sfx::Checkpoint => {
            let mut y = Synth::new(0.55);
            for (i, n) in [79, 84, 88, 91].iter().enumerate() {
                let len = if i == 3 { 0.25 } else { 0.08 };
                y.tone(i as f32 * 0.08, len, midi(*n), midi(*n), Square(0.25), 0.2, 0.003, len * 0.5, 0.0);
                y.tone(i as f32 * 0.08, len, midi(*n - 12), midi(*n - 12), Tri, 0.25, 0.003, len * 0.5, 0.0);
            }
            y
        }
        Sfx::Crumble => {
            let mut y = Synth::new(0.2);
            y.tone(0.0, 0.18, 1800.0, 600.0, Noise, 0.25, 0.002, 0.12, 0.0);
            y
        }
        Sfx::Shoot => {
            let mut y = Synth::new(0.12);
            y.tone(0.0, 0.1, 260.0, 100.0, Square(0.5), 0.25, 0.002, 0.05, 0.0);
            y.tone(0.0, 0.05, 3000.0, 1000.0, Noise, 0.2, 0.001, 0.04, 0.0);
            y
        }
        Sfx::Win => {
            let mut y = Synth::new(2.2);
            let tune = [(72, 0.0, 0.12), (76, 0.12, 0.12), (79, 0.24, 0.12), (84, 0.36, 0.3), (79, 0.72, 0.12), (84, 0.84, 0.9)];
            for (n, t, len) in tune {
                y.tone(t, len, midi(n), midi(n), Square(0.25), 0.2, 0.004, len * 0.4, 0.004);
                y.tone(t, len, midi(n + 12), midi(n + 12), Tri, 0.12, 0.004, len * 0.4, 0.0);
            }
            for (n, t, len) in [(48, 0.0, 0.36), (55, 0.36, 0.36), (48, 0.84, 1.0)] {
                y.tone(t, len, midi(n), midi(n), Tri, 0.35, 0.004, len * 0.5, 0.0);
            }
            y
        }
        Sfx::Hurt => {
            let mut y = Synth::new(0.3);
            y.tone(0.0, 0.22, 620.0, 180.0, Square(0.5), 0.3, 0.002, 0.1, 0.03);
            y.tone(0.0, 0.08, 4000.0, 1500.0, Noise, 0.18, 0.001, 0.06, 0.0);
            y
        }
        Sfx::Heart => {
            let mut y = Synth::new(0.5);
            for (i, n) in [72, 76, 79, 84].iter().enumerate() {
                let len = if i == 3 { 0.28 } else { 0.06 };
                y.tone(i as f32 * 0.06, len, midi(*n), midi(*n), Tri, 0.4, 0.003, len * 0.6, 0.0);
                y.tone(i as f32 * 0.06, len, midi(*n + 12), midi(*n + 12), Square(0.125), 0.06, 0.003, len * 0.6, 0.0);
            }
            y
        }
        Sfx::Spring => {
            let mut y = Synth::new(0.42);
            y.tone(0.0, 0.4, 170.0, 640.0, Sine, 0.5, 0.003, 0.2, 0.09);
            y.tone(0.0, 0.18, 340.0, 900.0, Square(0.25), 0.1, 0.003, 0.1, 0.0);
            y
        }
        Sfx::Glide => {
            let mut y = Synth::new(0.36);
            y.tone(0.0, 0.34, 2400.0, 700.0, Noise, 0.16, 0.06, 0.22, 0.0);
            y.tone(0.0, 0.3, 500.0, 760.0, Tri, 0.14, 0.04, 0.2, 0.04);
            y
        }
        Sfx::Throw => {
            let mut y = Synth::new(0.12);
            y.tone(0.0, 0.08, 380.0, 950.0, Square(0.25), 0.25, 0.002, 0.04, 0.0);
            y.tone(0.0, 0.06, 6000.0, 2500.0, Noise, 0.1, 0.002, 0.05, 0.0);
            y
        }
        Sfx::NoAmmo => {
            let mut y = Synth::new(0.18);
            y.tone(0.0, 0.05, 170.0, 160.0, Square(0.5), 0.2, 0.002, 0.02, 0.0);
            y.tone(0.09, 0.06, 140.0, 130.0, Square(0.5), 0.2, 0.002, 0.03, 0.0);
            y
        }
        Sfx::Pop => {
            let mut y = Synth::new(0.2);
            y.tone(0.0, 0.09, 950.0, 260.0, Sine, 0.5, 0.001, 0.05, 0.0);
            y.tone(0.0, 0.04, 7000.0, 3000.0, Noise, 0.22, 0.001, 0.03, 0.0);
            y.tone(0.05, 0.12, midi(88), midi(88), Tri, 0.2, 0.002, 0.1, 0.0);
            y
        }
        Sfx::BoxHit => {
            let mut y = Synth::new(0.36);
            y.tone(0.0, 0.07, 320.0, 200.0, Tri, 0.4, 0.001, 0.05, 0.0);
            y.tone(0.06, 0.06, midi(86), midi(86), Square(0.25), 0.15, 0.002, 0.03, 0.0);
            y.tone(0.12, 0.2, midi(93), midi(93), Square(0.25), 0.15, 0.002, 0.15, 0.0);
            y
        }
        Sfx::BoxEmpty => {
            let mut y = Synth::new(0.1);
            y.tone(0.0, 0.08, 150.0, 90.0, Tri, 0.45, 0.001, 0.06, 0.0);
            y
        }
        Sfx::Secret => {
            let mut y = Synth::new(1.0);
            for (i, n) in [79, 83, 86, 91, 95, 98].iter().enumerate() {
                let t = i as f32 * 0.07;
                y.tone(t, 0.4, midi(*n), midi(*n), Tri, 0.22, 0.003, 0.35, 0.0);
                y.tone(t + 0.21, 0.3, midi(*n), midi(*n), Tri, 0.07, 0.003, 0.25, 0.0);
            }
            y
        }
    }
}

struct Song {
    bpm: f32,
    melody: [&'static [(i32, i32)]; 8],
    roots: [[i32; 2]; 8],
    minor: [bool; 8],
    lead: Wave,
    /// 0 = root/octave bounce, 1 = oom-pah (root/fifth), 2 = long soft notes
    bass: u8,
    arp: f32,
}

/// Three original 8-bar loops, one per level.
const SONGS: [Song; 3] = [
    // Orange Meadow: C - Am - F - G, bright and bouncy
    Song {
        bpm: 140.0,
        melody: [
            &[(76, 2), (79, 1), (81, 1), (79, 2), (76, 2)],
            &[(72, 1), (74, 1), (76, 2), (74, 2), (72, 2)],
            &[(69, 2), (72, 1), (77, 1), (76, 2), (72, 2)],
            &[(74, 3), (71, 1), (67, 4)],
            &[(79, 1), (76, 1), (79, 2), (84, 2), (83, 1), (81, 1)],
            &[(81, 2), (79, 1), (76, 1), (72, 2), (76, 2)],
            &[(77, 2), (81, 2), (79, 2), (74, 2)],
            &[(72, 4), (0, 4)],
        ],
        roots: [[48, 48], [45, 45], [41, 41], [43, 43], [48, 48], [45, 45], [41, 43], [48, 48]],
        minor: [false, true, false, false, false, true, false, false],
        lead: Wave::Square(0.25),
        bass: 0,
        arp: 0.03,
    },
    // Cookie Factory: F - G - Em - Am - Dm - G - C, oom-pah machinery
    Song {
        bpm: 152.0,
        melody: [
            &[(72, 1), (74, 1), (77, 2), (76, 1), (74, 1), (72, 2)],
            &[(71, 1), (72, 1), (74, 2), (79, 2), (74, 2)],
            &[(76, 1), (79, 1), (76, 1), (72, 1), (71, 2), (67, 2)],
            &[(69, 2), (72, 2), (76, 3), (0, 1)],
            &[(77, 1), (76, 1), (74, 1), (72, 1), (74, 2), (77, 2)],
            &[(79, 2), (77, 1), (76, 1), (74, 2), (71, 2)],
            &[(72, 1), (76, 1), (79, 1), (84, 1), (83, 1), (79, 1), (76, 1), (74, 1)],
            &[(72, 4), (0, 4)],
        ],
        roots: [[41, 41], [43, 43], [40, 40], [45, 45], [38, 38], [43, 43], [48, 48], [48, 48]],
        minor: [false, false, true, true, true, false, false, false],
        lead: Wave::Square(0.5),
        bass: 1,
        arp: 0.02,
    },
    // Sky World: Fmaj7 - Em7 - Dm7 - Cmaj7 - F - G - Am - G, floaty
    Song {
        bpm: 112.0,
        melody: [
            &[(81, 3), (79, 1), (77, 2), (76, 2)],
            &[(79, 3), (76, 1), (74, 2), (72, 2)],
            &[(74, 2), (77, 2), (81, 2), (79, 2)],
            &[(76, 6), (0, 2)],
            &[(77, 2), (81, 2), (84, 3), (83, 1)],
            &[(83, 2), (79, 2), (74, 2), (79, 2)],
            &[(81, 3), (84, 1), (83, 2), (81, 2)],
            &[(79, 6), (0, 2)],
        ],
        roots: [[41, 41], [40, 40], [38, 38], [36, 36], [41, 41], [43, 43], [45, 45], [43, 43]],
        minor: [false, true, true, false, false, false, true, false],
        lead: Wave::Tri,
        bass: 2,
        arp: 0.06,
    },
];

fn make_bgm(song: &Song) -> Synth {
    use Wave::*;
    let e = 60.0 / song.bpm / 2.0; // one eighth note
    let bars = 8;
    let mut y = Synth::new(e * 8.0 * bars as f32);
    let soft = song.bass == 2;
    for bar in 0..bars {
        let t0 = bar as f32 * 8.0 * e;
        let mut pos = 0;
        for &(n, len) in song.melody[bar] {
            if n > 0 {
                let d = len as f32 * e * 0.92;
                let vol = if soft { 0.2 } else { 0.11 };
                y.tone(t0 + pos as f32 * e, d, midi(n), midi(n), song.lead, vol, 0.004, 0.05, 0.003);
                let under = if soft { Sine } else { Square(0.5) };
                y.tone(t0 + pos as f32 * e, d, midi(n - 12), midi(n - 12), under, 0.035, 0.004, 0.05, 0.0);
            }
            pos += len;
        }
        for k in 0..8 {
            let t = t0 + k as f32 * e;
            let root = song.roots[bar][k / 4];
            match song.bass {
                0 => y.note(t, e * 0.8, if k % 2 == 0 { root } else { root + 12 }, Tri, 0.22),
                1 => y.note(t, e * 0.6, if k % 2 == 0 { root } else { root + 7 }, Tri, 0.24),
                _ => {
                    if k % 4 == 0 {
                        y.tone(t, e * 3.8, midi(root), midi(root), Sine, 0.3, 0.02, e * 1.5, 0.0);
                    }
                }
            }
            if !soft {
                if k % 2 == 1 {
                    y.tone(t, 0.03, 8000.0, 8000.0, Noise, 0.035, 0.001, 0.02, 0.0);
                }
                if k % 4 == 0 {
                    y.tone(t, 0.12, 150.0, 45.0, Sine, 0.32, 0.001, 0.08, 0.0);
                }
                if k % 4 == 2 {
                    y.tone(t, 0.08, 2500.0, 1200.0, Noise, 0.08, 0.001, 0.06, 0.0);
                }
            } else if k % 8 == 4 {
                y.tone(t, 0.05, 6000.0, 6000.0, Noise, 0.025, 0.001, 0.04, 0.0);
            }
            // arpeggio sparkle
            let third = if song.minor[bar] { 3 } else { 4 };
            let chord = [root + 24, root + 24 + third, root + 31, root + 36];
            let (a, len) = if soft { (chord[k % 4], e * 0.9) } else { (chord[k % 3] + 12, e * 0.4) };
            y.note(t + e * 0.5, len, a, Tri, song.arp);
        }
    }
    y
}

pub struct Audio {
    sfx: HashMap<Sfx, Sound>,
    bgm: Vec<Option<Sound>>,
    playing: Option<usize>,
    pub muted: bool,
}

const BGM_VOLUME: f32 = 0.4;

impl Audio {
    pub async fn load() -> Audio {
        use Sfx::*;
        let all = [
            Jump, Land, Stomp, Coin, Golden, Shield, ShieldPop, Die, Checkpoint, Crumble, Shoot, Win, Hurt, Heart, Spring, Glide, Throw, NoAmmo,
            Pop, BoxHit, BoxEmpty, Secret,
        ];
        let mut sfx = HashMap::new();
        for s in all {
            if let Ok(snd) = load_sound_from_bytes(&make_sfx(s).wav()).await {
                sfx.insert(s, snd);
            }
        }
        let mut bgm = vec![];
        for song in &SONGS {
            bgm.push(load_sound_from_bytes(&make_bgm(song).wav()).await.ok());
        }
        Audio { sfx, bgm, playing: None, muted: false }
    }

    pub fn play(&self, s: Sfx) {
        if self.muted {
            return;
        }
        let volume = match s {
            Sfx::Land => 0.25,
            Sfx::Shoot => 0.3,
            Sfx::Crumble => 0.5,
            Sfx::Coin => 0.45,
            Sfx::Glide => 0.45,
            Sfx::Throw => 0.4,
            _ => 0.6,
        };
        if let Some(snd) = self.sfx.get(&s) {
            play_sound(snd, PlaySoundParams { looped: false, volume });
        }
    }

    /// Start the loop for level `i` (0-based) from the beginning.
    pub fn start_bgm(&mut self, i: usize) {
        self.stop_bgm();
        if let Some(Some(b)) = self.bgm.get(i) {
            play_sound(b, PlaySoundParams { looped: true, volume: if self.muted { 0.0 } else { BGM_VOLUME } });
            self.playing = Some(i);
        }
    }

    pub fn stop_bgm(&mut self) {
        if let Some(i) = self.playing.take() {
            if let Some(Some(b)) = self.bgm.get(i) {
                stop_sound(b);
            }
        }
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if let Some(i) = self.playing {
            if let Some(Some(b)) = self.bgm.get(i) {
                set_sound_volume(b, if self.muted { 0.0 } else { BGM_VOLUME });
            }
        }
    }
}
