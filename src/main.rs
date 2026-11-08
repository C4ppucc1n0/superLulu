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

const JUMP: &[KeyCode] = &[KeyCode::Z, KeyCode::Space, KeyCode::K, KeyCode::Up, KeyCode::W];
const RUN: &[KeyCode] = &[KeyCode::X, KeyCode::LeftShift, KeyCode::RightShift, KeyCode::J];
const THROW: &[KeyCode] = &[KeyCode::C, KeyCode::L];
const LEFT: &[KeyCode] = &[KeyCode::Left, KeyCode::A];
const RIGHT: &[KeyCode] = &[KeyCode::Right, KeyCode::D];
const START: &[KeyCode] = &[KeyCode::Enter, KeyCode::KpEnter, KeyCode::Space, KeyCode::Z];

/// Debug/preview mode, renders a few frames, saves a screenshot and exits:
/// `super_lulu --shot out.png <level 1-3> <tile_x> <tile_y> [frames] [hold-right|glide]`,
/// `super_lulu --title-shot out.png` or `super_lulu --select-shot out.png`.
struct Shot {
    out: String,
    screen: State,
    level: usize,
    tile: (i32, i32),
    frames: u32,
    hold_right: bool,
    /// Run right, jump, then press jump again and keep holding it.
    glide: bool,
}

fn parse_shot() -> Option<Shot> {
    let a: Vec<String> = std::env::args().collect();
    let simple = |screen| Some(Shot { out: a.get(2)?.clone(), screen, level: 0, tile: (0, 0), frames: 30, hold_right: false, glide: false });
    match a.get(1).map(|s| s.as_str()) {
        Some("--title-shot") => simple(State::Title),
        Some("--select-shot") => simple(State::Select),
        Some("--shot") => Some(Shot {
            out: a.get(2)?.clone(),
            screen: State::Playing,
            level: a.get(3)?.parse::<usize>().ok()?.clamp(1, LEVELS.len()) - 1,
            tile: (a.get(4)?.parse().ok()?, a.get(5)?.parse().ok()?),
            frames: a.get(6).and_then(|s| s.parse().ok()).unwrap_or(40),
            hold_right: a.get(7).is_some_and(|s| s == "hold-right" || s == "glide"),
            glide: a.get(7).is_some_and(|s| s == "glide"),
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
    let mut fullscreen = false;
    let mut title_t = 0.0f32;
    let mut new_best = false;

    let shot = parse_shot();
    let mut frame = 0u32;
    if let Some(s) = &shot {
        state = s.screen;
        if s.screen == State::Playing {
            level = s.level;
            game = new_game(level);
            game.teleport(s.tile.0, s.tile.1);
        }
    }

    loop {
        let dt = get_frame_time().min(0.05);
        game.view_w = screen_width() / screen_height().max(1.0) * VIEW_H;

        if is_key_pressed(KeyCode::F11) || (is_key_pressed(KeyCode::Enter) && any_down(&[KeyCode::LeftAlt, KeyCode::RightAlt])) {
            fullscreen = !fullscreen;
            set_fullscreen(fullscreen);
        }
        if is_key_pressed(KeyCode::M) {
            audio.toggle_mute();
        }
        let alt = any_down(&[KeyCode::LeftAlt, KeyCode::RightAlt]);

        let scripted_right = shot.as_ref().is_some_and(|s| s.hold_right);
        let scripted_glide = shot.as_ref().is_some_and(|s| s.glide);
        let mut input = Input {
            dir: (any_down(RIGHT) || scripted_right) as i32 as f32 - any_down(LEFT) as i32 as f32,
            run: any_down(RUN) || scripted_right,
            jump_pressed: false,
            jump_held: any_down(JUMP) || scripted_glide,
            throw_pressed: false,
        };
        if any_pressed(JUMP) || (scripted_glide && (frame == 2 || frame == 30)) {
            pending_jump = true;
        }
        if any_pressed(THROW) {
            pending_throw = true;
        }

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
                    pending_jump = false;
                    pending_throw = false;
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
                        pending_jump = false;
                        pending_throw = false;
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
                        input.jump_pressed = pending_jump && state == State::Playing;
                        input.throw_pressed = pending_throw && state == State::Playing;
                        pending_jump = false;
                        pending_throw = false;
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
                pending_jump = false;
                pending_throw = false;
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
            frame += 1;
            if frame >= s.frames {
                get_screen_data().export_png(&s.out);
                break;
            }
        }
        next_frame().await;
    }
}
