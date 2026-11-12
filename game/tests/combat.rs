use game::{Actions, FireMode, GameEvent, WeaponKind};
use glam::Vec3;

mod common;
use common::*;

#[test]
fn hot_rivet_pins_a_surviving_enemy_without_pushing_through_the_wall() {
    let wall = solid(Vec3::new(0.0, 2.0, -6.0), Vec3::new(8.0, 4.0, 0.1));
    let mut game = game_with_enemies(vec![wall], vec![Vec3::new(0.0, 0.0, -4.0)], Vec3::ZERO);
    game.config.grunt.run_speed = 0.0;
    game.player.pitch = -0.1;
    game.update(&Actions {
        alt_held: true,
        ..Actions::default()
    });
    run(&mut game, Actions::default(), 0.12);
    assert!(game.projectiles.is_empty());
    assert_eq!(game.enemies.len(), 1);
    assert!(game.enemies[0].pinned_seconds > 0.0);
    assert!(game.enemies[0].position.z > -5.5);
    run(&mut game, Actions::default(), 1.6);
    assert_eq!(game.enemies[0].pinned_seconds, 0.0);
}

#[test]
fn switching_tools_does_not_reset_the_rivet_cooldown() {
    let mut game = game_with(Vec::new(), Vec3::ZERO);
    game.update(&Actions {
        fire_held: true,
        ..Actions::default()
    });
    game.update(&Actions {
        select_weapon: Some(WeaponKind::Sandblaster),
        ..Actions::default()
    });
    game.update(&Actions {
        select_weapon: Some(WeaponKind::RivetGun),
        fire_held: true,
        ..Actions::default()
    });
    assert_eq!(
        game.events
            .iter()
            .filter(|event| matches!(event, GameEvent::ShotFired { .. }))
            .count(),
        1
    );
}

#[test]
fn rivet_projectile_hits_a_thin_wall_over_the_entire_step() {
    let wall = solid(Vec3::new(0.0, 2.0, -3.0), Vec3::new(8.0, 4.0, 0.04));
    let mut game = game_with_targets(
        vec![wall],
        vec![target(Vec3::new(0.0, 2.1, -6.0), 10.0)],
        Vec3::ZERO,
    );
    game.config.weapon.projectile_speed = 1200.0;
    game.update(&Actions {
        alt_held: true,
        ..Actions::default()
    });
    assert_eq!(game.projectiles.len(), 1);
    game.update(&Actions::default());
    assert!(game.projectiles.is_empty());
    assert_eq!(game.targets[0].health, 10.0);
    assert!(
        game.events
            .iter()
            .any(|event| matches!(event, GameEvent::ShotImpact { .. }))
    );
}

#[test]
fn sandblast_has_one_shot_event_for_all_pellets() {
    let mut game = game_with_targets(
        Vec::new(),
        vec![target(Vec3::new(0.0, 2.1, -5.0), 30.0)],
        Vec3::ZERO,
    );
    game.update(&Actions {
        select_weapon: Some(WeaponKind::Sandblaster),
        fire_held: true,
        ..Actions::default()
    });
    assert!(game.targets[0].health < 28.0);
    assert_eq!(
        game.events
            .iter()
            .filter(|event| matches!(event, GameEvent::ShotFired { .. }))
            .count(),
        1
    );
}

#[test]
fn charged_plasma_pierces_enemies_but_not_a_wall() {
    let wall = solid(Vec3::new(0.0, 2.0, -10.0), Vec3::new(8.0, 5.0, 0.1));
    let mut game = game_with_enemies(
        vec![wall],
        vec![
            Vec3::new(0.0, 0.0, -4.0),
            Vec3::new(0.0, 0.0, -7.0),
            Vec3::new(0.0, 0.0, -12.0),
        ],
        Vec3::ZERO,
    );
    game.config.grunt.run_speed = 0.0;
    game.player.pitch = -0.1;
    run(
        &mut game,
        Actions {
            select_weapon: Some(WeaponKind::PlasmaCutter),
            alt_held: true,
            ..Actions::default()
        },
        0.8,
    );
    game.update(&Actions {
        alt_released: true,
        ..Actions::default()
    });
    assert_eq!(game.enemies.len(), 1);
    assert_eq!(game.enemies[0].id, 2);
    assert_eq!(
        game.events
            .iter()
            .filter(|event| matches!(event, GameEvent::EnemyKilled { .. }))
            .count(),
        2
    );
    assert!(game.events.iter().any(|event| matches!(
        event,
        GameEvent::ShotFired {
            mode: FireMode::Secondary,
            ..
        }
    )));
}

#[test]
fn losing_focus_cancels_charge_without_discharge() {
    let mut game = game_with(Vec::new(), Vec3::ZERO);
    run(
        &mut game,
        Actions {
            select_weapon: Some(WeaponKind::PlasmaCutter),
            alt_held: true,
            ..Actions::default()
        },
        0.8,
    );
    game.update(&Actions {
        cancel_attacks: true,
        ..Actions::default()
    });
    game.update(&Actions {
        alt_released: true,
        ..Actions::default()
    });
    assert_eq!(game.weapon.charge, 0.0);
    assert!(
        !game
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::ShotFired { .. }))
    );
}

#[test]
fn air_blast_redirects_the_flying_rivet_without_recreating_it() {
    let mut game = game_with(Vec::new(), Vec3::ZERO);
    game.update(&Actions {
        alt_held: true,
        ..Actions::default()
    });
    game.update(&Actions {
        select_weapon: Some(WeaponKind::Sandblaster),
        alt_held: true,
        ..Actions::default()
    });
    assert_eq!(game.projectiles.len(), 1);
    assert!(game.projectiles[0].velocity.length() > game.config.weapon.projectile_speed);
    assert!(game.player.velocity.z > 10.0);
}

#[test]
fn held_plasma_has_one_start_event_not_one_sound_per_damage_tick() {
    let mut game = game_with(Vec::new(), Vec3::ZERO);
    run(
        &mut game,
        Actions {
            select_weapon: Some(WeaponKind::PlasmaCutter),
            fire_held: true,
            ..Actions::default()
        },
        0.5,
    );
    assert!(game.weapon.beam.is_some());
    assert_eq!(
        game.events
            .iter()
            .filter(|event| matches!(event, GameEvent::ShotFired { .. }))
            .count(),
        1
    );
    game.update(&Actions::default());
    assert!(game.weapon.beam.is_none());
}
