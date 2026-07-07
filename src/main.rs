//! Macroquad game template wired to macroquad-toolkit.

use macroquad::prelude::*;
use macroquad_toolkit::capture;

mod audio;
mod combat;
mod data;
mod game;
mod model;
mod state;
mod ui;
mod util;

use game::Game;

fn window_conf() -> Conf {
    capture::capture_window_conf(
        "IRON_FAUNA",
        "IRON FAUNA",
        ui::LOGICAL_WIDTH as i32,
        ui::LOGICAL_HEIGHT as i32,
    )
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new().await;

    // Screenshot harness: when IRON_FAUNA_CAPTURE_PATH is set, render
    // deterministic frames, write a PNG, and exit.
    if let Some(config) = capture::CaptureConfig::from_env("IRON_FAUNA") {
        game.begin_capture_scene(&config.scene);
        capture::run_capture(&config, |dt| {
            game.update(dt);
            game.draw();
        })
        .await;
        return;
    }

    loop {
        let dt = get_frame_time().min(0.1);
        game.update(dt);
        game.draw();
        next_frame().await;
    }
}
