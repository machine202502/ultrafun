use game::{Actions, ArenaPhase, ArenaSpawn, Game, GameEvent, MachineSpawn, TargetKind, Zone};
use glam::{Vec2, Vec3};

mod common;
use common::*;

fn zone(center: Vec3, size: Vec3) -> Zone {
    Zone { center, size }
}

fn arena_with(waves: Vec<Vec<Vec3>>, machine: Option<MachineSpawn>) -> ArenaSpawn {
    ArenaSpawn {
        trigger: zone(Vec3::new(0.0, 1.0, -5.0), Vec3::new(10.0, 4.0, 4.0)),
        doors: vec![zone(Vec3::new(0.0, 3.0, 2.0), Vec3::new(10.0, 6.0, 1.0))],
        waves,
        machine,
    }
}

fn two_wave_arena() -> ArenaSpawn {
    arena_with(
        vec![
            vec![Vec3::new(-5.0, 0.0, -20.0), Vec3::new(5.0, 0.0, -20.0)],
            vec![Vec3::new(0.0, 0.0, -25.0)],
        ],
        None,
    )
}

fn started(arena: ArenaSpawn) -> Game {
    let mut game = game_with_arena(Vec::new(), arena, Vec3::ZERO);
    game.player.position = Vec3::new(0.0, 0.01, -5.0);
    game.update(&Actions::default());
    assert_eq!(game.arenas[0].phase, ArenaPhase::Active);
    game
}

fn backward() -> Actions {
    Actions {
        move_input: Vec2::NEG_Y,
        ..Actions::default()
    }
}

#[test]
fn arena_starts_when_player_enters_trigger() {
    let mut game = game_with_arena(Vec::new(), two_wave_arena(), Vec3::ZERO);
    run(
        &mut game,
        Actions {
            move_input: Vec2::Y,
            ..Actions::default()
        },
        0.5,
    );
    assert_eq!(game.arenas[0].phase, ArenaPhase::Active);
    assert_eq!(game.enemies.len(), 2);
    assert!(game.events.contains(&GameEvent::ArenaStarted { arena: 0 }));
}

#[test]
fn closed_door_blocks_player() {
    let mut game = started(two_wave_arena());
    run(&mut game, backward(), 1.0);
    assert!(game.player.position.z < 1.1, "z {}", game.player.position.z);
}

#[test]
fn next_wave_spawns_after_previous_dies() {
    let mut game = started(two_wave_arena());
    game.enemies.clear();
    game.update(&Actions::default());
    assert_eq!(game.arenas[0].wave, 1);
    assert_eq!(game.enemies.len(), 1);
}

#[test]
fn arena_clears_and_opens_doors_after_last_wave() {
    let mut game = started(two_wave_arena());
    for _ in 0..2 {
        game.enemies.clear();
        game.update(&Actions::default());
    }
    assert_eq!(game.arenas[0].phase, ArenaPhase::Cleared);
    assert!(game.events.contains(&GameEvent::ArenaCleared { arena: 0 }));
    run(&mut game, backward(), 1.0);
    assert!(game.player.position.z > 3.0, "z {}", game.player.position.z);
}

#[test]
fn machine_spawns_up_to_limit_until_destroyed() {
    let machine = MachineSpawn {
        center: Vec3::new(0.0, 2.0, -40.0),
        size: Vec3::splat(2.0),
        health: 6.0,
        spawn_point: Vec3::new(0.0, 0.0, -37.0),
        spawn_interval_seconds: 1.0,
        max_alive: 2,
        say: None,
    };
    let mut game = started(arena_with(Vec::new(), Some(machine)));
    run(&mut game, Actions::default(), 3.5);
    assert_eq!(game.enemies.len(), 2);

    game.targets
        .retain(|target| target.kind != TargetKind::Machine);
    game.enemies.clear();
    run(&mut game, Actions::default(), 2.0);
    assert!(game.enemies.is_empty());
    assert_eq!(game.arenas[0].phase, ArenaPhase::Cleared);
}

#[test]
fn restart_resets_arena() {
    let mut game = started(two_wave_arena());
    game.update(&Actions {
        respawn: true,
        ..Actions::default()
    });
    assert_eq!(game.arenas[0].phase, ArenaPhase::Waiting);
    assert!(game.enemies.is_empty());
    assert_eq!(game.closed_doors().count(), 0);
}
