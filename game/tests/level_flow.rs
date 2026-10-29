use game::{Actions, ArenaSpawn, Game, GameEvent, Zone};
use glam::{Vec2, Vec3};

mod common;
use common::*;

fn forward() -> Actions {
    Actions {
        move_input: Vec2::Y,
        ..Actions::default()
    }
}

fn exit_ahead() -> Zone {
    Zone {
        center: Vec3::new(0.0, 1.0, -10.0),
        size: Vec3::new(6.0, 4.0, 2.0),
    }
}

fn game_with_exit(arenas: Vec<ArenaSpawn>) -> Game {
    let mut game = game_with_level(Vec::new(), Vec::new(), Vec::new(), Vec3::ZERO);
    let mut level = game.level().clone();
    level.exit = Some(exit_ahead());
    level.arenas = arenas;
    game.set_level(level);
    game
}

fn far_arena() -> ArenaSpawn {
    ArenaSpawn {
        trigger: Zone {
            center: Vec3::new(0.0, 1.0, 50.0),
            size: Vec3::splat(2.0),
        },
        doors: Vec::new(),
        waves: vec![vec![Vec3::new(0.0, 0.0, 60.0)]],
        machine: None,
    }
}

#[test]
fn reaching_exit_completes_level() {
    let mut game = game_with_exit(Vec::new());
    run(&mut game, forward(), 1.0);
    let result = game.result.expect("level should be completed");
    assert!(result.seconds > 0.4 && result.seconds < 1.0, "{result:?}");
    assert_eq!(result.kills, 0);
    assert!(game.events.contains(&GameEvent::LevelCompleted));
}

#[test]
fn player_is_frozen_after_completion() {
    let mut game = game_with_exit(Vec::new());
    run(&mut game, forward(), 1.0);
    let position = game.player.position;
    run(&mut game, forward(), 0.5);
    assert_eq!(game.player.position, position);
}

#[test]
fn exit_is_closed_until_arenas_are_cleared() {
    let mut game = game_with_exit(vec![far_arena()]);
    run(&mut game, forward(), 1.0);
    assert!(game.result.is_none());
}

#[test]
fn kills_are_counted_in_result() {
    let mut game = game_with_exit(Vec::new());
    let mut level = game.level().clone();
    level.enemies = vec![game::EnemySpawn {
        position: Vec3::new(0.0, 0.0, -6.0),
    }];
    game.set_level(level);
    game.player.pitch = -0.2;
    run(
        &mut game,
        Actions {
            fire_held: true,
            ..Actions::default()
        },
        0.4,
    );
    assert!(game.enemies.is_empty());
    run(&mut game, forward(), 1.0);
    assert_eq!(game.result.map(|result| result.kills), Some(1));
}

#[test]
fn restart_clears_result() {
    let mut game = game_with_exit(Vec::new());
    run(&mut game, forward(), 1.0);
    assert!(game.result.is_some());
    game.update(&Actions {
        respawn: true,
        ..Actions::default()
    });
    assert!(game.result.is_none());
    assert!(game.events.contains(&GameEvent::LevelStarted));
}
