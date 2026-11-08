#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod game;
mod level;
mod physics;
mod render;
#[cfg(test)]
mod reach;

use macroquad::miniquad::conf::Icon;
use macroquad::prelude::*;

use game::{Game, VIEW_H};
use level::Level;
use physics::Input;

const FIXED_DT: f32 = 1.0 / 120.0;
const LEVEL: &str = include_str!("levels/level1.txt");

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
const LEFT: &[KeyCode] = &[KeyCode::Left, KeyCode::A];
const RIGHT: &[KeyCode] = &[KeyCode::Right, KeyCode::D];
const START: &[KeyCode] = &[KeyCode::Enter, KeyCode::KpEnter, KeyCode::Space, KeyCode::Z];

/// Debug/preview mode: `super_lulu --shot out.png <tile_x> <tile_y> <frames> [hold-right]`
/// or `super_lulu --title-shot out.png`. Renders, saves a screenshot and exits.
struct Shot {
    out: String,
    title: bool,
    tile: (i32, i32),
    frames: u32,
    hold_right: bool,
}

fn parse_shot() -> Option<Shot> {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(|s| s.as_str()) {
        Some("--title-shot") => Some(Shot { out: a.get(2)?.clone(), title: true, tile: (0, 0), frames: 30, hold_right: false }),
        Some("--shot") => Some(Shot {
            out: a.get(2)?.clone(),
            title: false,
            tile: (a.get(3)?.parse().ok()?, a.get(4)?.parse().ok()?),
            frames: a.get(5).and_then(|s| s.parse().ok()).unwrap_or(40),
            hold_right: a.get(6).is_some_and(|s| s == "hold-right"),
        }),
        _ => None,
    }
}

fn save_screenshot(path: &str) {
    get_screen_data().export_png(path);
}

#[macroquad::main(window_conf)]
async fn main() {
    let assets = render::Assets::load();
    let mut audio = audio::Audio::load().await;
    let mut game = Game::new(Level::parse(LEVEL));
    let mut state = State::Title;
    let mut acc = 0.0f32;
    let mut pending_jump = false;
    let mut fullscreen = false;
    let mut title_t = 0.0f32;

    let shot = parse_shot();
    let mut frame = 0u32;
    if let Some(s) = &shot {
        if !s.title {
            state = State::Playing;
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

        let scripted_right = shot.as_ref().is_some_and(|s| s.hold_right);
        let mut input = Input {
            dir: (any_down(RIGHT) || scripted_right) as i32 as f32 - any_down(LEFT) as i32 as f32,
            run: any_down(RUN) || scripted_right,
            jump_pressed: false,
            jump_held: any_down(JUMP),
        };
        if any_pressed(JUMP) {
            pending_jump = true;
        }

        match state {
            State::Title => {
                title_t += dt;
                game.update_camera(dt, false);
                if is_key_pressed(KeyCode::Escape) {
                    break;
                }
                if any_pressed(START) && !any_down(&[KeyCode::LeftAlt, KeyCode::RightAlt]) {
                    game = Game::new(Level::parse(LEVEL));
                    game.view_w = screen_width() / screen_height().max(1.0) * VIEW_H;
                    game.update_camera(0.0, true);
                    state = State::Playing;
                    pending_jump = false;
                    acc = 0.0;
                    audio.start_bgm();
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
                    if game.clear_t > 2.0 && any_pressed(&[KeyCode::Enter, KeyCode::KpEnter, KeyCode::Space]) {
                        game = Game::new(Level::parse(LEVEL));
                        game.view_w = screen_width() / screen_height().max(1.0) * VIEW_H;
                        game.update_camera(0.0, true);
                        state = State::Playing;
                        pending_jump = false;
                        audio.start_bgm();
                    } else if is_key_pressed(KeyCode::Escape) {
                        state = State::Title;
                        title_t = 0.0;
                        game = Game::new(Level::parse(LEVEL));
                        audio.stop_bgm();
                    }
                }
                if state != State::Paused {
                    acc += dt;
                    while acc >= FIXED_DT {
                        input.jump_pressed = pending_jump && state == State::Playing;
                        pending_jump = false;
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
                    }
                }
            }
            State::Paused => {
                pending_jump = false;
                if is_key_pressed(KeyCode::Escape) {
                    state = State::Playing;
                    acc = 0.0;
                } else if is_key_pressed(KeyCode::R) {
                    game.retry();
                    state = State::Playing;
                    acc = 0.0;
                } else if is_key_pressed(KeyCode::Q) {
                    state = State::Title;
                    title_t = 0.0;
                    game = Game::new(Level::parse(LEVEL));
                    audio.stop_bgm();
                }
            }
        }

        clear_background(BLACK);
        let level_h = game.lv.h as f32 * physics::TILE;
        if state == State::Title {
            render::draw_title_scene(title_t, game.view_w, level_h);
        } else {
            render::draw_world(&game, &assets);
        }
        match state {
            State::Title => {
                render::draw_title(&assets, title_t, (title_t * 2.0) as i32 % 2 == 0, level_h);
            }
            State::Playing => render::draw_hud(&game),
            State::Paused => {
                render::draw_hud(&game);
                render::draw_pause();
            }
            State::Cleared => {
                render::draw_hud(&game);
                render::draw_clear(&game);
            }
        }

        if let Some(s) = &shot {
            frame += 1;
            if frame >= s.frames {
                save_screenshot(&s.out);
                break;
            }
        }
        next_frame().await;
    }
}
