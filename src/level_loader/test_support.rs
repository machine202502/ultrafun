use std::path::Path;

use game::{Actions, Game, MoveState};
use glam::Vec2;

use super::{BlenderLevelFile, load_blender_level};

pub fn game_config() -> game::GameConfig {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/data");
    let load = |name: &str| std::fs::read_to_string(data.join(name)).unwrap();
    game::GameConfig {
        movement: ron::from_str(&load("movement.ron")).unwrap(),
        weapon: ron::from_str(&load("weapons.ron")).unwrap(),
        health: ron::from_str(&load("health.ron")).unwrap(),
        grunt: ron::from_str(&load("grunt.ron")).unwrap(),
    }
}

pub fn shipped_game(level: game::Level) -> Game {
    Game::new(level, game_config())
}

pub fn run(game: &mut Game, actions: Actions, seconds: f32) {
    for _ in 0..(seconds / game::STEP_SECONDS) as usize {
        game.update(&actions);
    }
}

pub fn foundry_game() -> Game {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/levels");
    let text = std::fs::read_to_string(directory.join("foundry.ron")).unwrap();
    let file: BlenderLevelFile = ron::from_str(&text).unwrap();
    shipped_game(load_blender_level(&directory, &file).unwrap().0)
}

pub fn place(game: &mut Game, position: glam::Vec3, yaw_degrees: f32) {
    game.player.position = position + glam::Vec3::Y * 0.01;
    game.player.previous_position = game.player.position;
    game.player.velocity = glam::Vec3::ZERO;
    game.player.yaw = yaw_degrees.to_radians();
    game.player.pitch = 0.0;
    game.player.grounded = false;
    game.player.state = MoveState::Normal;
}

pub fn forward() -> Actions {
    Actions {
        move_input: Vec2::Y,
        ..Actions::default()
    }
}

pub fn run_toward(game: &mut Game, target: glam::Vec3, seconds: f32) {
    let delta = target - game.player.position;
    game.player.yaw = delta.x.atan2(-delta.z);
    run(game, forward(), seconds);
}

pub fn jump_toward(game: &mut Game, from: glam::Vec3, to: glam::Vec3) {
    game.player.position = from + glam::Vec3::Y * 0.01;
    game.player.previous_position = game.player.position;
    game.player.velocity = glam::Vec3::ZERO;
    game.player.pitch = 0.0;
    game.player.grounded = false;
    game.player.state = MoveState::Normal;
    let delta = to - from;
    game.player.yaw = delta.x.atan2(-delta.z);
    run(game, Actions::default(), 0.05);
    game.update(&Actions {
        move_input: Vec2::Y,
        jump_pressed: true,
        ..Actions::default()
    });
    for _ in 0..160 {
        if game.player.grounded && game.player.position.y > from.y + 0.5 {
            break;
        }
        game.update(&forward());
    }
}
