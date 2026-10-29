use game::{Actions, Game, GameEvent, RepairPart};
use glam::{Vec2, Vec3};

mod common;
use common::*;

fn flat_distance(a: Vec3, b: Vec3) -> f32 {
    Vec2::new(a.x - b.x, a.z - b.z).length()
}

fn part_at(position: Vec3) -> RepairPart {
    RepairPart {
        position,
        previous_position: position,
        velocity: Vec3::ZERO,
        remaining_seconds: 4.0,
        resting: true,
    }
}

fn parts_around(game: &Game, distance: f32) -> Vec<RepairPart> {
    (0..4)
        .map(|index| {
            let angle = index as f32 * std::f32::consts::FRAC_PI_2;
            part_at(game.player.position + Vec3::new(angle.cos(), 0.5, angle.sin()) * distance)
        })
        .collect()
}

#[test]
fn grunt_chases_player() {
    let mut game = game_with_enemies(Vec::new(), vec![Vec3::new(0.0, 0.0, -20.0)], Vec3::ZERO);
    run(&mut game, Actions::default(), 1.0);
    let distance = flat_distance(game.enemies[0].position, game.player.position);
    assert!(distance < 10.0, "distance {distance}");
}

#[test]
fn grunt_hits_only_after_windup() {
    let mut game = game_with_enemies(Vec::new(), vec![Vec3::new(0.0, 0.0, -1.5)], Vec3::ZERO);
    run(&mut game, Actions::default(), 0.3);
    assert_eq!(game.player.health, 100.0);
    run(&mut game, Actions::default(), 0.2);
    assert_eq!(game.player.health, 85.0);
}

#[test]
fn running_away_during_windup_avoids_hit() {
    let mut game = game_with_enemies(Vec::new(), vec![Vec3::new(0.0, 0.0, -1.5)], Vec3::ZERO);
    let back = Actions {
        move_input: Vec2::NEG_Y,
        ..Actions::default()
    };
    run(&mut game, back, 0.45);
    assert_eq!(game.player.health, 100.0);
}

#[test]
fn dashing_player_is_immune() {
    let mut game = game_with_enemies(Vec::new(), vec![Vec3::new(0.0, 0.0, -1.5)], Vec3::ZERO);
    game.config.grunt.attack_range = 50.0;
    run(&mut game, Actions::default(), 0.3);
    game.update(&Actions {
        dash_pressed: true,
        ..Actions::default()
    });
    run(&mut game, Actions::default(), 0.15);
    assert_eq!(game.player.health, 100.0);
}

#[test]
fn two_shots_kill_grunt_and_drop_parts() {
    let mut game = game_with_enemies(Vec::new(), vec![Vec3::new(0.0, 0.0, -10.0)], Vec3::ZERO);
    game.player.pitch = -0.15;
    run(
        &mut game,
        Actions {
            fire_held: true,
            ..Actions::default()
        },
        0.4,
    );

    assert!(game.enemies.is_empty());
    assert_eq!(game.parts.len(), 4);
    assert!(
        game.events
            .iter()
            .any(|event| matches!(event, GameEvent::EnemyKilled { id: 0, .. }))
    );
}

#[test]
fn parts_heal_player_nearby() {
    let mut game = game_with_enemies(Vec::new(), Vec::new(), Vec3::ZERO);
    run(&mut game, Actions::default(), 0.1);
    game.player.health = 50.0;
    game.parts = parts_around(&game, 3.0);
    run(&mut game, Actions::default(), 0.5);
    assert_eq!(game.player.health, 70.0);
    assert!(game.parts.is_empty());
}

#[test]
fn healing_does_not_exceed_max_health() {
    let mut game = game_with_enemies(Vec::new(), Vec::new(), Vec3::ZERO);
    run(&mut game, Actions::default(), 0.1);
    game.player.health = 98.0;
    game.parts = parts_around(&game, 3.0);
    run(&mut game, Actions::default(), 0.5);
    assert_eq!(game.player.health, 100.0);
}

#[test]
fn distant_parts_do_not_heal_and_expire() {
    let mut game = game_with_enemies(Vec::new(), Vec::new(), Vec3::ZERO);
    run(&mut game, Actions::default(), 0.1);
    game.player.health = 50.0;
    game.parts = parts_around(&game, 20.0);
    run(&mut game, Actions::default(), 4.5);
    assert_eq!(game.player.health, 50.0);
    assert!(game.parts.is_empty());
}

#[test]
fn player_death_restarts_after_delay() {
    let mut game = game_with_enemies(Vec::new(), vec![Vec3::new(0.0, 0.0, -1.5)], Vec3::ZERO);
    game.player.health = 10.0;
    run(&mut game, Actions::default(), 0.5);
    assert_eq!(game.player.health, 0.0);
    assert!(game.restart_countdown_seconds.is_some());
    assert!(game.events.contains(&GameEvent::PlayerDied));
    run(&mut game, Actions::default(), 1.6);
    assert_eq!(game.player.health, 100.0);
    assert!(game.restart_countdown_seconds.is_none());
    assert_eq!(game.enemies.len(), 1);
}

#[test]
fn grunts_do_not_overlap() {
    let spawns = vec![Vec3::new(0.0, 0.0, -20.0), Vec3::new(0.1, 0.0, -20.0)];
    let mut game = game_with_enemies(Vec::new(), spawns, Vec3::ZERO);
    run(&mut game, Actions::default(), 0.5);
    let distance = flat_distance(game.enemies[0].position, game.enemies[1].position);
    assert!(distance >= 0.99, "distance {distance}");
}
#[test]
fn grunt_below_a_platform_cannot_hit_through_the_floor() {
    let platform = solid(Vec3::new(0.0, 7.75, 0.0), Vec3::new(6.0, 0.5, 6.0));
    let mut game = game_with_enemies(
        vec![platform],
        vec![Vec3::new(0.0, 0.0, 0.0)],
        Vec3::new(0.0, 8.0, 0.0),
    );
    run(&mut game, Actions::default(), 2.0);
    assert_eq!(game.player.health, 100.0);
}

#[test]
fn grunt_cannot_hit_through_a_wall() {
    let wall = solid(Vec3::new(0.0, 1.5, -1.0), Vec3::new(20.0, 3.0, 0.4));
    let mut game = game_with_enemies(vec![wall], vec![Vec3::new(0.0, 0.0, -2.5)], Vec3::ZERO);
    run(&mut game, Actions::default(), 1.5);
    assert_eq!(game.player.health, 100.0);
}

#[test]
fn grunt_still_hits_what_it_can_see() {
    let mut game = game_with_enemies(Vec::new(), vec![Vec3::new(0.0, 0.0, -1.5)], Vec3::ZERO);
    run(&mut game, Actions::default(), 0.6);
    assert!(game.player.health < 100.0, "health {}", game.player.health);
}
#[test]
fn shooting_an_enemy_pushes_it_back() {
    let mut game = game_with_enemies(Vec::new(), vec![Vec3::new(0.0, 0.0, -3.0)], Vec3::ZERO);
    game.player.pitch = -0.2;
    let fire = Actions {
        fire_held: true,
        ..Actions::default()
    };
    game.update(&fire);

    let pushed = game.enemies[0].knockback.length();
    assert!(pushed > 1.0, "knockback {pushed}");
    let before = game.enemies[0].position.z;
    run(&mut game, Actions::default(), 0.2);
    assert!(
        game.enemies[0].position.z < before,
        "enemy did not move back: before {before}, after {}",
        game.enemies[0].position.z
    );
}
