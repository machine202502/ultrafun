use game::{
    Actions, BreakableSpawn, DoorSpawn, Game, GameEvent, HazardSpawn, KeySpawn, ScriptAction,
    SpawnGroup, SwitchSpawn, TriggerSpawn, Zone,
};
use glam::Vec3;

mod common;
use common::*;

fn zone(center: Vec3, size: Vec3) -> Zone {
    Zone { center, size }
}

fn door(name: &str, center: Vec3, size: Vec3) -> DoorSpawn {
    DoorSpawn {
        name: name.into(),
        zones: vec![zone(center, size)],
        color: [0.45, 0.47, 0.5],
    }
}

fn key(name: &str, center: Vec3) -> KeySpawn {
    KeySpawn {
        name: name.into(),
        zone: zone(center, Vec3::splat(0.5)),
        color: [1.0, 0.85, 0.2],
    }
}

fn breakable(name: &str, zones: Vec<Zone>, by_slam: bool) -> BreakableSpawn {
    BreakableSpawn {
        name: name.into(),
        zones,
        color: [0.45, 0.47, 0.5],
        by_slam,
    }
}

fn trigger(zone: Zone, requires_key: Option<&str>, actions: Vec<ScriptAction>) -> TriggerSpawn {
    TriggerSpawn {
        zone,
        requires_key: requires_key.map(str::to_owned),
        actions,
    }
}

fn switch(center: Vec3, actions: Vec<ScriptAction>) -> SwitchSpawn {
    SwitchSpawn {
        name: "valve".into(),
        zone: zone(center, Vec3::new(2.0, 3.0, 2.0)),
        color: [0.9, 0.1, 0.1],
        health: 1.0,
        actions,
    }
}

fn hazard(center: Vec3) -> HazardSpawn {
    HazardSpawn {
        name: "lava".into(),
        zone: zone(center, Vec3::new(4.0, 2.0, 4.0)),
        color: [1.0, 0.45, 0.08],
        damage_per_second: 1000.0,
    }
}

fn forward() -> Actions {
    Actions {
        move_input: glam::Vec2::Y,
        ..Actions::default()
    }
}

fn says(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Say { .. }))
        .count()
}

#[test]
fn closed_door_blocks_player_and_open_door_lets_through() {
    let mut game = game_with_environment(Vec3::ZERO, |level| {
        level.doors.push(door(
            "gate",
            Vec3::new(0.0, 2.0, -5.0),
            Vec3::new(4.0, 4.0, 1.0),
        ));
    });
    run(&mut game, forward(), 1.0);
    assert!(
        game.player.position.z > -4.5,
        "z {}",
        game.player.position.z
    );

    let mut open_game = game_with_environment(Vec3::ZERO, |level| {
        level.doors.push(door(
            "gate",
            Vec3::new(0.0, 2.0, -5.0),
            Vec3::new(4.0, 4.0, 1.0),
        ));
        level.triggers.push(trigger(
            zone(Vec3::new(0.0, 1.0, 0.0), Vec3::new(2.0, 3.0, 2.0)),
            None,
            vec![ScriptAction::Open("gate".into())],
        ));
    });
    run(&mut open_game, forward(), 2.0);
    assert!(open_game.doors[0].open);
    assert!(
        open_game.player.position.z < -6.0,
        "z {}",
        open_game.player.position.z
    );
}

#[test]
fn closed_door_emits_event_and_close_blocks_again_once() {
    let mut game = game_with_environment(Vec3::ZERO, |level| {
        level.doors.push(door(
            "gate",
            Vec3::new(0.0, 2.0, -5.0),
            Vec3::new(4.0, 4.0, 1.0),
        ));
        level.triggers.push(trigger(
            zone(Vec3::new(0.0, 1.0, 0.0), Vec3::new(2.0, 3.0, 2.0)),
            None,
            vec![ScriptAction::Open("gate".into())],
        ));
    });
    game.update(&Actions::default());
    assert!(game.events.contains(&GameEvent::DoorOpened {
        name: "gate".into()
    }));
    assert!(!game.events.contains(&GameEvent::DoorClosed {
        name: "gate".into()
    }));
}

#[test]
fn trigger_fires_once() {
    let mut game = game_with_environment(Vec3::ZERO, |level| {
        level.triggers.push(trigger(
            zone(Vec3::new(0.0, 1.0, 0.0), Vec3::new(2.0, 3.0, 2.0)),
            None,
            vec![ScriptAction::Say("test".into())],
        ));
    });
    run(&mut game, Actions::default(), 0.5);
    assert_eq!(says(&game), 1);
    game.events.clear();
    run(&mut game, Actions::default(), 0.5);
    assert_eq!(says(&game), 0);
    assert!(game.triggers[0]);
}

#[test]
fn trigger_waits_for_key_and_fires_later() {
    let mut game = game_with_environment(Vec3::ZERO, |level| {
        level.doors.push(door(
            "gate",
            Vec3::new(0.0, 2.0, -5.0),
            Vec3::new(4.0, 4.0, 1.0),
        ));
        level.keys.push(key("master", Vec3::new(5.0, 1.25, 0.0)));
        level.triggers.push(trigger(
            zone(Vec3::new(0.0, 1.0, 0.0), Vec3::new(2.0, 3.0, 2.0)),
            Some("master"),
            vec![
                ScriptAction::Open("gate".into()),
                ScriptAction::Say("gate_opened".into()),
            ],
        ));
    });
    run(&mut game, Actions::default(), 0.5);
    assert!(!game.doors[0].open);
    assert_eq!(says(&game), 0);
    assert!(!game.triggers[0]);

    game.player.position = Vec3::new(5.0, 0.01, 0.0);
    run(&mut game, Actions::default(), 0.2);
    assert!(game.keys[0].collected);
    assert!(game.events.contains(&GameEvent::KeyCollected {
        name: "master".into()
    }));

    game.player.position = Vec3::ZERO;
    run(&mut game, Actions::default(), 0.2);
    assert!(game.doors[0].open);
    assert_eq!(says(&game), 1);
}

#[test]
fn key_is_collected_once_and_stays_in_inventory() {
    let mut game = game_with_environment(Vec3::ZERO, |level| {
        level.keys.push(key("master", Vec3::new(0.0, 1.25, 0.0)));
    });
    run(&mut game, Actions::default(), 0.2);
    assert!(game.has_key("master"));
    let collected = game
        .events
        .iter()
        .filter(|event| matches!(event, GameEvent::KeyCollected { .. }))
        .count();
    assert_eq!(collected, 1);
    run(&mut game, Actions::default(), 0.5);
    assert!(game.has_key("master"));
}

#[test]
fn break_action_removes_the_group_and_drops_the_player() {
    let mut game = game_with_environment(Vec3::new(0.0, 3.01, 0.0), |level| {
        level.breakables.push(breakable(
            "floor",
            vec![zone(Vec3::new(0.0, 2.5, 0.0), Vec3::new(6.0, 1.0, 6.0))],
            false,
        ));
        level.triggers.push(trigger(
            zone(Vec3::new(0.0, 3.0, 0.0), Vec3::new(6.0, 2.0, 6.0)),
            None,
            vec![ScriptAction::Break("floor".into())],
        ));
    });
    run(&mut game, Actions::default(), 0.2);
    assert!(game.breakables[0].broken);
    assert!(game.events.contains(&GameEvent::Broken {
        name: "floor".into(),
        position: Vec3::new(0.0, 2.5, 0.0),
    }));
    run(&mut game, Actions::default(), 1.0);
    assert!(game.player.position.y < 1.0, "y {}", game.player.position.y);
}

#[test]
fn slam_breaks_only_groups_that_allow_it() {
    let mut breaking = game_with_environment(Vec3::new(0.0, 8.0, 0.0), |level| {
        level.breakables.push(breakable(
            "catwalk",
            vec![zone(Vec3::new(0.0, 4.25, 0.0), Vec3::new(6.0, 0.5, 6.0))],
            true,
        ));
    });
    run(&mut breaking, slam(), 1.0);
    assert!(breaking.breakables[0].broken);
    assert!(
        breaking
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::Slammed { .. }))
    );

    let mut intact = game_with_environment(Vec3::new(0.0, 8.0, 0.0), |level| {
        level.breakables.push(breakable(
            "shelf",
            vec![zone(Vec3::new(0.0, 4.25, 0.0), Vec3::new(6.0, 0.5, 6.0))],
            false,
        ));
    });
    run(&mut intact, slam(), 1.0);
    assert!(!intact.breakables[0].broken);
    assert!(intact.player.position.y > 4.0);
}

fn slam() -> Actions {
    Actions {
        slide_pressed: true,
        ..Actions::default()
    }
}

#[test]
fn destroyed_switch_runs_actions_once() {
    let mut game = game_with_environment(Vec3::ZERO, |level| {
        level.breakables.push(breakable(
            "barrier",
            vec![zone(Vec3::new(0.0, 1.0, -20.0), Vec3::new(4.0, 2.0, 4.0))],
            false,
        ));
        level.switches.push(switch(
            Vec3::new(0.0, 1.5, -10.0),
            vec![
                ScriptAction::Break("barrier".into()),
                ScriptAction::Say("valve_closed".into()),
            ],
        ));
    });
    game.update(&Actions {
        fire_held: true,
        ..Actions::default()
    });
    assert!(game.switches[0].activated);
    assert!(game.breakables[0].broken);
    assert!(game.events.contains(&GameEvent::SwitchActivated {
        name: "valve".into()
    }));
    assert_eq!(says(&game), 1);

    run(&mut game, Actions::default(), 0.5);
    assert_eq!(says(&game), 1);
}

#[test]
fn spawn_action_releases_a_dummy_at_every_point() {
    let mut game = game_with_environment(Vec3::ZERO, |level| {
        level.spawns.push(SpawnGroup {
            name: "ambush".into(),
            positions: vec![
                Vec3::new(5.0, 0.0, 0.0),
                Vec3::new(6.0, 0.0, 0.0),
                Vec3::new(7.0, 0.0, 0.0),
            ],
        });
        level.triggers.push(trigger(
            zone(Vec3::new(0.0, 1.0, 0.0), Vec3::new(2.0, 3.0, 2.0)),
            None,
            vec![ScriptAction::Spawn("ambush".into())],
        ));
    });
    game.update(&Actions::default());
    assert_eq!(game.enemies.len(), 3);
}

#[test]
fn respawn_keeps_progress_and_returns_to_the_checkpoint() {
    let mut game = game_with_environment(Vec3::ZERO, |level| {
        level.doors.push(door(
            "gate",
            Vec3::new(0.0, 2.0, -5.0),
            Vec3::new(4.0, 4.0, 1.0),
        ));
        level.keys.push(key("master", Vec3::new(0.0, 1.25, 0.0)));
        level.breakables.push(breakable(
            "shelf",
            vec![zone(Vec3::new(3.0, 2.0, 0.0), Vec3::new(1.0, 4.0, 4.0))],
            false,
        ));
        level
            .checkpoints
            .push(zone(Vec3::new(0.0, 2.0, 0.0), Vec3::new(4.0, 4.0, 4.0)));
        level.triggers.push(trigger(
            zone(Vec3::new(0.0, 1.0, 0.0), Vec3::new(2.0, 3.0, 2.0)),
            None,
            vec![
                ScriptAction::Open("gate".into()),
                ScriptAction::Break("shelf".into()),
            ],
        ));
    });
    run(&mut game, Actions::default(), 0.2);
    assert!(game.doors[0].open);
    assert!(game.breakables[0].broken);
    assert!(game.has_key("master"));
    assert!(game.triggers[0]);
    assert!(game.checkpoint.is_some());

    game.player.position = Vec3::new(12.0, 3.0, 0.0);
    game.update(&Actions {
        respawn: true,
        ..Actions::default()
    });
    assert!(game.doors[0].open, "respawn closed the opened door");
    assert!(
        game.breakables[0].broken,
        "respawn restored the broken shelf"
    );
    assert!(game.has_key("master"), "respawn lost the key");
    assert!(game.triggers[0], "respawn reset the trigger");
    assert!(game.checkpoint.is_some(), "respawn dropped the checkpoint");
    assert!(
        (game.player.position - Vec3::new(0.0, 0.0, 0.0)).length() < 1.0,
        "respawned at {}",
        game.player.position
    );
}

#[test]
fn respawn_without_a_checkpoint_restarts_the_level() {
    let mut game = game_with_environment(Vec3::ZERO, |level| {
        level.doors.push(door(
            "gate",
            Vec3::new(0.0, 2.0, -5.0),
            Vec3::new(4.0, 4.0, 1.0),
        ));
        level.keys.push(key("master", Vec3::new(0.0, 1.25, 0.0)));
        level.triggers.push(trigger(
            zone(Vec3::new(0.0, 1.0, 0.0), Vec3::new(2.0, 3.0, 2.0)),
            None,
            vec![ScriptAction::Open("gate".into())],
        ));
    });
    run(&mut game, Actions::default(), 0.2);
    assert!(game.doors[0].open);
    game.update(&Actions {
        respawn: true,
        ..Actions::default()
    });
    assert!(!game.doors[0].open);
    assert!(!game.has_key("master"));
    assert!(!game.triggers[0]);
}

fn checkpoint_game() -> Game {
    game_with_environment(Vec3::ZERO, |level| {
        level
            .checkpoints
            .push(zone(Vec3::new(0.0, 2.0, 0.0), Vec3::new(4.0, 4.0, 4.0)));
        level.hazards.push(hazard(Vec3::new(20.0, 1.0, 0.0)));
    })
}

fn die(game: &mut Game) {
    game.player.position = Vec3::new(20.0, 0.01, 0.0);
    run(game, Actions::default(), 2.0);
}

#[test]
fn death_respawns_at_checkpoint_with_full_health() {
    let mut game = checkpoint_game();
    game.player.yaw = 1.0;
    run(&mut game, Actions::default(), 0.2);
    assert!(game.checkpoint.is_some());
    assert!(
        game.events
            .contains(&GameEvent::CheckpointReached { index: 0 })
    );

    game.player.health = 10.0;
    let before = game.elapsed_seconds();
    die(&mut game);
    assert!((game.player.position - Vec3::new(0.0, 0.01, 0.0)).length() < 0.05);
    assert_eq!(game.player.health, 100.0);
    assert!((game.player.yaw - 1.0).abs() < 1e-6);
    assert!(game.restart_countdown_seconds.is_none());
    assert!(game.elapsed_seconds() >= before);
}

#[test]
fn death_without_checkpoint_restarts_the_level() {
    let mut game = game_with_environment(Vec3::new(3.0, 0.0, 3.0), |level| {
        level.hazards.push(hazard(Vec3::new(20.0, 1.0, 0.0)));
    });
    run(&mut game, Actions::default(), 0.2);
    die(&mut game);
    assert!((game.player.position - Vec3::new(3.0, 0.01, 3.0)).length() < 0.05);
    assert!(game.elapsed_seconds() < 1.0);
}

#[test]
fn respawn_resets_the_active_arena_but_keeps_a_cleared_one() {
    let arena = game::ArenaSpawn {
        trigger: zone(Vec3::new(0.0, 1.0, -5.0), Vec3::new(6.0, 4.0, 4.0)),
        doors: vec![zone(Vec3::new(0.0, 3.0, 2.0), Vec3::new(10.0, 6.0, 1.0))],
        waves: vec![vec![Vec3::new(0.0, 0.0, -20.0)]],
        machine: None,
    };
    let mut game = game_with_arena(Vec::new(), arena, Vec3::ZERO);
    game.player.position = Vec3::new(0.0, 0.01, -5.0);
    game.update(&Actions::default());
    assert_eq!(game.arenas[0].phase, game::ArenaPhase::Active);
    assert_eq!(game.enemies.len(), 1);

    game.arenas[0].phase = game::ArenaPhase::Cleared;
    let mut level = game.level().clone();
    level
        .checkpoints
        .push(zone(Vec3::new(0.0, 2.0, 0.0), Vec3::new(4.0, 4.0, 4.0)));
    level.hazards.push(hazard(Vec3::new(20.0, 1.0, 0.0)));
    level.arenas.push(game::ArenaSpawn {
        trigger: zone(Vec3::new(50.0, 1.0, 50.0), Vec3::new(2.0, 4.0, 2.0)),
        doors: Vec::new(),
        waves: vec![vec![Vec3::new(40.0, 0.0, 40.0)]],
        machine: None,
    });
    game.set_level(level);
    game.player.position = Vec3::new(0.0, 0.01, 0.0);
    game.arenas[0].phase = game::ArenaPhase::Cleared;
    game.arenas[1].phase = game::ArenaPhase::Active;
    game.arenas[1].wave_enemy_ids.push(99);
    game.enemies.push(game::Enemy::new(
        99,
        Vec3::new(40.0, 0.0, 40.0),
        &game.config.grunt,
    ));
    run(&mut game, Actions::default(), 0.2);

    die(&mut game);
    assert_eq!(game.arenas[0].phase, game::ArenaPhase::Cleared);
    assert_eq!(game.arenas[1].phase, game::ArenaPhase::Waiting);
    assert!(game.enemies.iter().all(|enemy| enemy.id != 99));
}

#[test]
fn checkpoint_only_moves_forward() {
    let mut game = game_with_environment(Vec3::ZERO, |level| {
        level
            .checkpoints
            .push(zone(Vec3::new(0.0, 2.0, 0.0), Vec3::new(4.0, 4.0, 4.0)));
        level
            .checkpoints
            .push(zone(Vec3::new(0.0, 2.0, -10.0), Vec3::new(4.0, 4.0, 4.0)));
    });
    run(&mut game, Actions::default(), 0.2);
    assert_eq!(
        game.checkpoint.as_ref().map(|checkpoint| checkpoint.index),
        Some(0)
    );

    game.player.position = Vec3::new(0.0, 0.01, -10.0);
    run(&mut game, Actions::default(), 0.2);
    assert_eq!(
        game.checkpoint.as_ref().map(|checkpoint| checkpoint.index),
        Some(1)
    );

    game.player.position = Vec3::ZERO;
    run(&mut game, Actions::default(), 0.2);
    assert_eq!(
        game.checkpoint.as_ref().map(|checkpoint| checkpoint.index),
        Some(1)
    );
}
