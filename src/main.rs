#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod game;
mod level;
mod physics;
#[cfg(test)]
mod reach;
mod render;
mod save;

use macroquad::miniquad::conf::Icon;
use macroquad::prelude::*;

use game::{Game, VIEW_H};
use level::{Level, Theme, LEVELS};
use physics::Input;
use save::Progress;

const FIXED_DT: f32 = 1.0 / 120.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "Super Lulu".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: true,
        high_dpi: true,
        sample_count: 4,
        icon: Some(Icon {
            small: *include_bytes!("../assets/sprites/icon16.rgba"),
            medium: *include_bytes!("../assets/sprites/icon32.rgba"),
            big: *include_bytes!("../assets/sprites/icon64.rgba"),
        }),
        ..Default::default()
    }
}

#[derive(PartialEq, Clone, Copy)]
enum State {
    Title,
    Select,
    Playing,
    Paused,
    Cleared,
}

fn any_down(keys: &[KeyCode]) -> bool {
    keys.iter().any(|k| is_key_down(*k))
}
fn any_pressed(keys: &[KeyCode]) -> bool {
    keys.iter().any(|k| is_key_pressed(*k))
}

// W A D move and jump, S crouches (hold it, then W = super jump), J K L attack.
const JUMP: &[KeyCode] = &[KeyCode::W, KeyCode::Space, KeyCode::Up];
const RUN: &[KeyCode] = &[KeyCode::LeftShift, KeyCode::RightShift];
const DOWN: &[KeyCode] = &[KeyCode::S, KeyCode::Down];
const WHIP: &[KeyCode] = &[KeyCode::J];
const POUND: &[KeyCode] = &[KeyCode::K];
const THROW: &[KeyCode] = &[KeyCode::L];
const LEFT: &[KeyCode] = &[KeyCode::Left, KeyCode::A];
const RIGHT: &[KeyCode] = &[KeyCode::Right, KeyCode::D];
const START: &[KeyCode] = &[KeyCode::Enter, KeyCode::KpEnter, KeyCode::Space];

/// Debug/preview mode, plays for a moment, saves a screenshot and exits:
/// `super_lulu --shot out.png <level> <tile_x> <tile_y> [seconds] [hold-right|glide|super|whip|pound|tired|defeat]`,
/// `super_lulu --title-shot out.png` or `super_lulu --select-shot out.png`.
struct Shot {
    out: String,
    screen: State,
    level: usize,
    tile: (i32, i32),
    secs: f32,
    action: String,
}

/// Scripted input for a screenshot, for the frame that covers game time t0..t1.
#[derive(Default)]
struct Script {
    right: bool,
    down: bool,
    jump_held: bool,
    jump: bool,
    whip: bool,
    pound: bool,
}

impl Shot {
    fn script(&self, t0: f32, t1: f32) -> Script {
        let at = |t: f32| t0 < t && t <= t1;
        match self.action.as_str() {
            "hold-right" => Script { right: true, ..Default::default() },
            // run right, jump, then press jump again and keep holding it
            "glide" => Script { right: true, jump_held: true, jump: at(0.1) || at(0.55), ..Default::default() },
            // crouch to charge, then super jump to the right
            "super" => Script { down: t1 > 0.1 && t1 < 0.65, jump: at(0.66), jump_held: t1 > 0.66, right: t1 > 0.75, ..Default::default() },
            // three-hit combo
            "whip" => Script { whip: at(0.1) || at(0.2) || at(0.45), ..Default::default() },
            // jump, then ground pound
            "pound" => Script { jump: at(0.1), jump_held: t1 < 0.4, pound: at(0.45), ..Default::default() },
            _ => Script::default(),
        }
    }
}

fn parse_shot() -> Option<Shot> {
    let a: Vec<String> = std::env::args().collect();
    let simple = |screen| Some(Shot { out: a.get(2)?.clone(), screen, level: 0, tile: (0, 0), secs: 0.5, action: String::new() });
    match a.get(1).map(|s| s.as_str()) {
        Some("--title-shot") => simple(State::Title),
        Some("--select-shot") => simple(State::Select),
        Some("--shot") => Some(Shot {
            out: a.get(2)?.clone(),
            screen: State::Playing,
            level: a.get(3)?.parse::<usize>().ok()?.clamp(1, LEVELS.len()) - 1,
            tile: (a.get(4)?.parse().ok()?, a.get(5)?.parse().ok()?),
            secs: a.get(6).and_then(|s| s.parse().ok()).unwrap_or(0.7),
            action: a.get(7).cloned().unwrap_or_default(),
        }),
        _ => None,
    }
}

fn new_game(level: usize) -> Game {
    let mut g = Game::new(Level::parse(LEVELS[level]));
    g.view_w = screen_width() / screen_height().max(1.0) * VIEW_H;
    g.update_camera(0.0, true);
    g
}

#[macroquad::main(window_conf)]
async fn main() {
    render::load_font();
    let assets = render::Assets::load();
    let mut audio = audio::Audio::load().await;
    let names: Vec<(String, Theme)> = LEVELS
        .iter()
        .map(|src| {
            let lv = Level::parse(src);
            (lv.name, lv.theme)
        })
        .collect();
    let mut progress = Progress::load();
    let mut level = 0usize;
    let mut sel = 0usize;
    let mut game = new_game(level);
    let mut state = State::Title;
    let mut acc = 0.0f32;
    let mut pending_jump = false;
    let mut pending_throw = false;
    let mut pending_whip = false;
    let mut pending_pound = false;
    let mut fullscreen = false;
    let mut title_t = 0.0f32;
    let mut new_best = false;

    let shot = parse_shot();
    let mut shot_t = 0.0f32;
    if let Some(s) = &shot {
        state = s.screen;
        if s.screen == State::Playing {
            level = s.level;
            game = new_game(level);
            game.teleport(s.tile.0, s.tile.1);
        }
    }

    loop {
        // screenshots run on a fixed clock so they come out the same every time
        let dt = if shot.is_some() { 1.0 / 60.0 } else { get_frame_time().min(0.05) };
        game.view_w = screen_width() / screen_height().max(1.0) * VIEW_H;

        if is_key_pressed(KeyCode::F11) || (is_key_pressed(KeyCode::Enter) && any_down(&[KeyCode::LeftAlt, KeyCode::RightAlt])) {
            fullscreen = !fullscreen;
            set_fullscreen(fullscreen);
        }
        if is_key_pressed(KeyCode::M) {
            audio.toggle_mute();
        }
        let alt = any_down(&[KeyCode::LeftAlt, KeyCode::RightAlt]);

        let sc = shot.as_ref().map(|s| s.script(shot_t, shot_t + dt)).unwrap_or_default();
        let mut input = Input {
            dir: (any_down(RIGHT) || sc.right) as i32 as f32 - any_down(LEFT) as i32 as f32,
            run: any_down(RUN) || sc.right,
            down: any_down(DOWN) || sc.down,
            jump_held: any_down(JUMP) || sc.jump_held,
            ..Default::default()
        };
        pending_jump |= any_pressed(JUMP) || sc.jump;
        pending_throw |= any_pressed(THROW);
        pending_whip |= any_pressed(WHIP) || sc.whip;
        pending_pound |= any_pressed(POUND) || sc.pound;

        let was_playing = state == State::Playing;
        match state {
            State::Title => {
                title_t += dt;
                if is_key_pressed(KeyCode::Escape) {
                    break;
                }
                if any_pressed(START) && !alt {
                    state = State::Select;
                    sel = (0..LEVELS.len()).rev().find(|&i| progress.unlocked(i) && !progress.levels[i].cleared).unwrap_or(level);
                }
            }
            State::Select => {
                title_t += dt;
                if any_pressed(LEFT) {
                    sel = sel.saturating_sub(1);
                }
                if any_pressed(RIGHT) {
                    sel = (sel + 1).min(LEVELS.len() - 1);
                }
                if is_key_pressed(KeyCode::Escape) {
                    state = State::Title;
                } else if any_pressed(START) && !alt && progress.unlocked(sel) {
                    level = sel;
                    game = new_game(level);
                    state = State::Playing;
                    acc = 0.0;
                    audio.start_bgm(level);
                }
            }
            State::Playing | State::Cleared => {
                if state == State::Playing {
                    if is_key_pressed(KeyCode::Escape) {
                        state = State::Paused;
                    }
                    if is_key_pressed(KeyCode::R) {
                        game.retry();
                    }
                } else {
                    input = Input::default();
                    let has_next = level + 1 < LEVELS.len();
                    let go = game.clear_t > 2.0 && any_pressed(&[KeyCode::Enter, KeyCode::KpEnter, KeyCode::Space]) && !alt;
                    if go && has_next {
                        level += 1;
                        game = new_game(level);
                        state = State::Playing;
                        audio.start_bgm(level);
                    } else if go || is_key_pressed(KeyCode::Escape) {
                        state = State::Select;
                        sel = if has_next { level + 1 } else { level };
                        audio.stop_bgm();
                    } else if !has_next && is_key_pressed(KeyCode::R) {
                        game = new_game(level);
                        state = State::Playing;
                        audio.start_bgm(level);
                    }
                }
                if state == State::Playing || state == State::Cleared {
                    acc += dt;
                    while acc >= FIXED_DT {
                        let live = state == State::Playing;
                        input.jump_pressed = pending_jump && live;
                        input.throw_pressed = pending_throw && live;
                        input.whip_pressed = pending_whip && live;
                        input.pound_pressed = pending_pound && live;
                        pending_jump = false;
                        pending_throw = false;
                        pending_whip = false;
                        pending_pound = false;
                        game.update(&input, FIXED_DT);
                        acc -= FIXED_DT;
                    }
                    for s in game.sfx.drain(..) {
                        if s == game::Sfx::Win {
                            audio.stop_bgm();
                        }
                        audio.play(s);
                    }
                    if state == State::Playing && game.cleared {
                        state = State::Cleared;
                        new_best = progress.record(level, game.time, game.golden_mask);
                        if shot.is_none() {
                            progress.save();
                        }
                    }
                }
            }
            State::Paused => {
                if is_key_pressed(KeyCode::Escape) {
                    state = State::Playing;
                    acc = 0.0;
                } else if is_key_pressed(KeyCode::R) {
                    game.retry();
                    state = State::Playing;
                    acc = 0.0;
                } else if is_key_pressed(KeyCode::Q) {
                    state = State::Select;
                    sel = level;
                    audio.stop_bgm();
                }
            }
        }

        // presses made outside of play must not leak into the game
        if !was_playing || state != State::Playing {
            (pending_jump, pending_throw, pending_whip, pending_pound) = (false, false, false, false);
        }

        clear_background(BLACK);
        match state {
            State::Title => {
                render::draw_title_scene(title_t, game.view_w);
                render::draw_title(&assets, title_t, (title_t * 2.0) as i32 % 2 == 0);
            }
            State::Select => {
                render::draw_title_scene(title_t, game.view_w);
                render::dim(0.25);
                render::draw_level_select(&names, &progress, sel, title_t);
            }
            State::Playing => {
                render::draw_world(&game, &assets);
                render::draw_hud(&game);
            }
            State::Paused => {
                render::draw_world(&game, &assets);
                render::draw_hud(&game);
                render::draw_pause();
            }
            State::Cleared => {
                render::draw_world(&game, &assets);
                render::draw_hud(&game);
                render::draw_clear(&game, new_best, level + 1 < LEVELS.len());
            }
        }

        if let Some(s) = &shot {
            // boss previews: skip ahead to the tired / beaten boss
            if shot_t <= 3.0 && shot_t + dt > 3.0 {
                if let Some(b) = game.boss.as_mut() {
                    match s.action.as_str() {
                        "tired" => b.state = game::BState::Tired,
                        "defeat" => b.state = game::BState::Defeated,
                        _ => {}
                    }
                    b.t = 0.0;
                }
            }
            shot_t += dt;
            if shot_t >= s.secs {
                get_screen_data().export_png(&s.out);
                break;
            }
        }
        next_frame().await;
    }
}
