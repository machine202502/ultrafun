use game::{Actions, GameEvent};
use glam::Vec3;

mod common;
use common::*;

fn fire() -> Actions {
    Actions {
        fire_held: true,
        ..Actions::default()
    }
}

fn respawn() -> Actions {
    Actions {
        respawn: true,
        ..Actions::default()
    }
}

fn target_in_front(health: f32) -> Vec<game::TargetSpawn> {
    vec![target(Vec3::new(0.0, 2.1, -10.0), health)]
}

#[test]
fn shot_damages_target_in_view() {
    let mut game = game_with_targets(Vec::new(), target_in_front(3.0), Vec3::ZERO);
    game.update(&fire());
    assert_eq!(game.targets[0].health, 2.0);
    assert!(game.events.contains(&GameEvent::TargetHit { id: 0 }));
}

#[test]
fn target_is_destroyed_when_health_runs_out() {
    let mut game = game_with_targets(Vec::new(), target_in_front(2.0), Vec3::ZERO);
    run(&mut game, fire(), 0.5);
    assert!(game.targets.is_empty());
    assert!(
        game.events
            .iter()
            .any(|event| matches!(event, GameEvent::TargetDestroyed { id: 0, .. }))
    );
}

#[test]
fn wall_blocks_shot_and_leaves_impact() {
    let wall = solid(Vec3::new(0.0, 3.0, -5.0), Vec3::new(10.0, 6.0, 0.5));
    let mut game = game_with_targets(vec![wall], target_in_front(3.0), Vec3::ZERO);
    game.update(&fire());
    assert_eq!(game.targets[0].health, 3.0);
    let impact = game.events.iter().find_map(|event| match event {
        GameEvent::ShotImpact { position, normal } => Some((*position, *normal)),
        _ => None,
    });
    let (position, normal) = impact.expect("shot should hit the wall");
    assert!((position.z + 4.75).abs() < 0.01, "impact {position}");
    assert!(normal.abs_diff_eq(Vec3::Z, 1e-4));
    let shot_end = game.events.iter().find_map(|event| match event {
        GameEvent::ShotFired { end, .. } => Some(*end),
        _ => None,
    });
    assert!(shot_end.is_some_and(|end| end.abs_diff_eq(position, 1e-4)));
}

#[test]
fn fire_rate_is_limited_by_cooldown() {
    let mut game = game_with_targets(Vec::new(), target_in_front(100.0), Vec3::ZERO);
    run(&mut game, fire(), 1.0);
    let shots = game
        .events
        .iter()
        .filter(|event| matches!(event, GameEvent::ShotFired { .. }))
        .count();
    assert_eq!(shots, 4);
}

#[test]
fn respawn_restores_targets() {
    let mut game = game_with_targets(Vec::new(), target_in_front(1.0), Vec3::ZERO);
    game.update(&fire());
    assert!(game.targets.is_empty());
    game.update(&respawn());
    assert_eq!(game.targets.len(), 1);
    assert_eq!(game.targets[0].health, 1.0);
}
