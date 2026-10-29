use game::{
    Actions, BeltSpawn, CrusherSpawn, Game, GameEvent, HazardSpawn, LevelBox, PadSpawn, TubeSpawn,
    Zone,
};
use glam::Vec3;

mod common;
use common::*;

fn zone(center: Vec3, size: Vec3) -> Zone {
    Zone { center, size }
}

fn hazard_game() -> Game {
    game_with_environment(Vec3::ZERO, |level| {
        level.hazards.push(HazardSpawn {
            name: "lava".into(),
            zone: zone(Vec3::new(0.0, 1.0, 0.0), Vec3::new(4.0, 2.0, 4.0)),
            color: [1.0, 0.45, 0.08],
            damage_per_second: 1000.0,
        });
    })
}

#[test]
fn player_in_lethal_hazard_dies() {
    let mut game = hazard_game();
    run(&mut game, Actions::default(), 0.2);
    assert_eq!(game.player.health, 0.0);
    assert!(game.events.contains(&GameEvent::PlayerDied));
}

#[test]
fn disabled_hazard_does_not_hurt() {
    let mut game = hazard_game();
    game.disable_hazard("lava");
    run(&mut game, Actions::default(), 0.5);
    assert_eq!(game.player.health, 100.0);
}

#[test]
fn gentle_hazard_drains_health_over_time() {
    let mut game = game_with_environment(Vec3::ZERO, |level| {
        level.hazards.push(HazardSpawn {
            name: "steam".into(),
            zone: zone(Vec3::new(0.0, 1.0, 0.0), Vec3::new(4.0, 2.0, 4.0)),
            color: [0.8, 0.8, 0.8],
            damage_per_second: 20.0,
        });
    });
    run(&mut game, Actions::default(), 1.0);
    assert!(
        (game.player.health - 80.0).abs() < 1.0,
        "{}",
        game.player.health
    );
}

fn pad_game(launch_speed: f32) -> Game {
    game_with_environment(Vec3::ZERO, |level| {
        level.pads.push(PadSpawn {
            zone: zone(Vec3::new(0.0, 0.5, 0.0), Vec3::new(4.0, 1.0, 4.0)),
            launch_speed,
        });
    })
}

#[test]
fn pad_launches_player_upward() {
    let mut game = pad_game(31.0);
    game.update(&Actions::default());
    assert_eq!(game.player.velocity.y, 31.0);
    assert!(!game.player.grounded);
    assert!(
        game.events
            .iter()
            .any(|event| matches!(event, GameEvent::Launched { .. }))
    );
}

#[test]
fn pad_does_not_slow_a_faster_rise() {
    let mut game = pad_game(31.0);
    game.player.velocity.y = 40.0;
    game.player.grounded = false;
    game.update(&Actions::default());
    assert!(game.player.velocity.y > 31.0, "{}", game.player.velocity.y);
    assert!(
        !game
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::Launched { .. }))
    );
}

fn belt_game(velocity: Vec3, boxes: Vec<LevelBox>) -> Game {
    game_with_environment(Vec3::new(0.0, 0.5, 0.0), |level| {
        level.boxes.extend(boxes);
        level
            .boxes
            .push(solid(Vec3::new(0.0, 0.25, 0.0), Vec3::new(20.0, 0.5, 20.0)));
        level.belts.push(BeltSpawn {
            zone: zone(Vec3::new(0.0, 0.25, 0.0), Vec3::new(20.0, 0.5, 20.0)),
            velocity,
        });
    })
}

#[test]
fn belt_carries_standing_player() {
    let mut game = belt_game(Vec3::new(0.0, 0.0, -4.0), Vec::new());
    run(&mut game, Actions::default(), 1.0);
    assert!(game.player.grounded);
    assert!(
        game.player.position.z < -3.0,
        "z {}",
        game.player.position.z
    );
}

#[test]
fn belt_does_not_carry_airborne_player() {
    let mut game = belt_game(Vec3::new(0.0, 0.0, -4.0), Vec::new());
    run(&mut game, Actions::default(), 0.2);
    let start = game.player.position;
    game.update(&Actions {
        jump_pressed: true,
        ..Actions::default()
    });
    run(&mut game, Actions::default(), 0.2);
    assert!(!game.player.grounded);
    assert!(
        (game.player.position.z - start.z).abs() < 0.05,
        "z {}",
        game.player.position.z
    );
}

#[test]
fn belt_does_not_push_player_through_wall() {
    let mut game = belt_game(
        Vec3::new(6.0, 0.0, 0.0),
        vec![solid(Vec3::new(2.0, 1.0, 0.0), Vec3::new(1.0, 2.0, 4.0))],
    );
    run(&mut game, Actions::default(), 1.0);
    assert!(game.player.position.x > 0.5, "x {}", game.player.position.x);
    assert!(game.player.position.x < 1.2, "x {}", game.player.position.x);
}

fn crusher(offset_seconds: f32) -> CrusherSpawn {
    CrusherSpawn {
        center: Vec3::new(0.0, 7.5, 0.0),
        size: Vec3::new(3.0, 3.0, 8.0),
        color: [0.45, 0.47, 0.5],
        travel: 6.0,
        period_seconds: 2.0,
        offset_seconds,
    }
}

fn crusher_game(offset_seconds: f32, spawn: Vec3) -> Game {
    game_with_environment(spawn, |level| {
        level.crushers.push(crusher(offset_seconds));
    })
}

#[test]
fn crusher_kills_player_while_descending_or_at_bottom() {
    let mut game = crusher_game(0.4, Vec3::ZERO);
    run(&mut game, Actions::default(), 0.05);
    assert_eq!(game.player.health, 0.0);
    assert!(game.events.contains(&GameEvent::PlayerDied));
}

#[test]
fn crusher_does_not_kill_player_at_top() {
    let mut game = crusher_game(1.4, Vec3::ZERO);
    run(&mut game, Actions::default(), 0.5);
    assert_eq!(game.player.health, 100.0);
}

#[test]
fn crusher_position_follows_period_and_offset() {
    let machine = crusher(0.0);
    assert!((machine.center_at(0.0) - machine.center).length() < 1e-4);
    assert!((machine.center_at(0.3).y - (machine.center.y - 6.0)).abs() < 1e-3);
    assert!((machine.center_at(0.5).y - (machine.center.y - 6.0)).abs() < 1e-3);
    assert!((machine.center_at(0.85).y - (machine.center.y - 3.0)).abs() < 1e-3);
    assert!((machine.center_at(1.2).y - machine.center.y).abs() < 1e-3);
    assert!((machine.center_at(2.3).y - (machine.center.y - 6.0)).abs() < 1e-3);
    assert!((crusher(0.4).center_at(0.0).y - (machine.center.y - 6.0)).abs() < 1e-3);
}

#[test]
fn crusher_reports_slam_when_it_hits_the_bottom() {
    let mut game = crusher_game(0.0, Vec3::new(30.0, 0.0, 0.0));
    run(&mut game, Actions::default(), 0.5);
    let slams = game
        .events
        .iter()
        .filter(|event| matches!(event, GameEvent::CrusherSlammed { .. }))
        .count();
    assert_eq!(slams, 1);
}

#[test]
fn tube_travels_player_to_exit_along_arrow() {
    let mut game = game_with_environment(Vec3::ZERO, |level| {
        level.tubes.push(TubeSpawn {
            zone: zone(Vec3::new(0.0, 1.5, 0.0), Vec3::new(2.0, 3.0, 2.0)),
            exit: Vec3::new(20.0, 4.0, 0.0),
            exit_yaw_degrees: 90.0,
            exit_speed: 25.0,
        });
    });
    game.update(&Actions::default());
    assert!((game.player.position.x - 20.0).abs() < 1e-3);
    assert!((game.player.position.y - 4.01).abs() < 0.02);
    assert!((game.player.velocity.x - 25.0).abs() < 1e-3);
    assert!(game.player.velocity.y == 0.0);
    assert_eq!(game.player.previous_position, game.player.position);
    assert!((game.player.yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    assert!(
        game.events
            .iter()
            .any(|event| matches!(event, GameEvent::TubeTravelled { .. }))
    );
}
