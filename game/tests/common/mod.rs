#![allow(dead_code)]

use game::{
    Actions, ArenaSpawn, EnemySpawn, Game, GameConfig, GruntConfig, HealthConfig, Level, LevelBox,
    MovementConfig, STEP_SECONDS, TargetSpawn, WeaponConfig,
};
use glam::Vec3;

pub fn config() -> MovementConfig {
    MovementConfig {
        standing_height: 2.5,
        slide_height: 1.2,
        radius: 0.5,
        eye_height: 2.1,
        slide_eye_height: 0.9,
        step_height: 0.5,
        max_walkable_slope_degrees: 45.0,
        max_pitch_degrees: 89.0,
        run_speed: 16.5,
        ground_acceleration_seconds: 0.1,
        air_acceleration: 40.0,
        ground_excess_friction: 30.0,
        air_excess_friction: 4.0,
        gravity: 40.0,
        jump_speed: 16.5,
        max_fall_speed: 60.0,
        coyote_seconds: 0.1,
        jump_buffer_seconds: 0.1,
        stamina_max: 3.0,
        stamina_regen_per_second: 0.7,
        dash_cost: 1.0,
        dash_jump_cost: 1.0,
        dash_speed: 49.5,
        dash_seconds: 0.2,
        slide_speed: 24.0,
        slide_friction: 10.0,
        slide_max_speed: 72.0,
        wall_jumps_max: 3,
        wall_jump_away_speed: 15.0,
        wall_jump_up_speed: 15.0,
        slide_wall_jump_up_speed: 8.0,
        wall_max_tilt_degrees: 20.0,
        wall_contact_distance: 0.2,
        wall_cling_start_fall_speed: 2.0,
        wall_cling_fall_speed_growth: 15.0,
        slam_speed: 100.0,
        slam_bounce_seconds: 0.1,
        slam_bounce_max_height: 15.0,
    }
}

pub fn solid(center: Vec3, size: Vec3) -> LevelBox {
    LevelBox {
        center,
        size,
        rotation_degrees: Vec3::ZERO,
        color: [0.5; 3],
    }
}

pub fn floor() -> LevelBox {
    solid(Vec3::new(0.0, -0.5, 0.0), Vec3::new(400.0, 1.0, 400.0))
}

pub fn game_with(boxes: Vec<LevelBox>, spawn: Vec3) -> Game {
    game_with_targets(boxes, Vec::new(), spawn)
}

pub fn game_with_targets(boxes: Vec<LevelBox>, targets: Vec<TargetSpawn>, spawn: Vec3) -> Game {
    game_with_level(boxes, targets, Vec::new(), spawn)
}

pub fn game_with_enemies(boxes: Vec<LevelBox>, enemies: Vec<Vec3>, spawn: Vec3) -> Game {
    let enemies = enemies
        .into_iter()
        .map(|position| EnemySpawn { position })
        .collect();
    game_with_level(boxes, Vec::new(), enemies, spawn)
}

pub fn game_with_level(
    mut boxes: Vec<LevelBox>,
    targets: Vec<TargetSpawn>,
    enemies: Vec<EnemySpawn>,
    spawn: Vec3,
) -> Game {
    boxes.push(floor());
    let level = Level {
        spawn_position: spawn,
        spawn_yaw_degrees: 0.0,
        boxes,
        targets,
        enemies,
        arenas: Vec::new(),
        exit: None,
    };
    Game::new(level, game_config())
}

pub fn game_config() -> GameConfig {
    GameConfig {
        movement: config(),
        weapon: weapon_config(),
        health: health_config(),
        grunt: grunt_config(),
    }
}

pub fn health_config() -> HealthConfig {
    HealthConfig {
        max_health: 100.0,
        restart_delay_seconds: 1.5,
        parts_per_kill: 4,
        part_heal: 5.0,
        part_launch_speed: 6.0,
        part_gravity: 40.0,
        part_attract_radius: 6.0,
        part_attract_speed: 30.0,
        part_collect_radius: 1.2,
        part_lifetime_seconds: 4.0,
    }
}

pub fn grunt_config() -> GruntConfig {
    GruntConfig {
        height: 2.0,
        radius: 0.5,
        max_health: 2.0,
        run_speed: 12.0,
        attack_range: 2.0,
        attack_reach_leeway: 0.5,
        windup_seconds: 0.4,
        damage: 15.0,
        recover_seconds: 1.0,
        gravity: 40.0,
        max_fall_speed: 60.0,
        step_height: 0.5,
        max_walkable_slope_degrees: 45.0,
    }
}

pub fn settled_game(boxes: Vec<LevelBox>) -> Game {
    let mut game = game_with(boxes, Vec3::ZERO);
    run(&mut game, Actions::default(), 0.2);
    assert!(game.player.grounded);
    game
}

pub fn steps(seconds: f32) -> usize {
    (seconds / STEP_SECONDS).round() as usize
}

pub fn run(game: &mut Game, actions: Actions, seconds: f32) {
    for _ in 0..steps(seconds) {
        game.update(&actions);
    }
}

pub fn weapon_config() -> WeaponConfig {
    WeaponConfig {
        damage: 1.0,
        cooldown_seconds: 0.3,
        range: 500.0,
    }
}

pub fn target(center: Vec3, health: f32) -> TargetSpawn {
    TargetSpawn {
        center,
        size: Vec3::splat(2.0),
        health,
    }
}

pub fn game_with_arena(boxes: Vec<LevelBox>, arena: ArenaSpawn, spawn: Vec3) -> Game {
    let mut game = game_with_level(boxes, Vec::new(), Vec::new(), spawn);
    let mut level = game.level().clone();
    level.arenas.push(arena);
    game.set_level(level);
    game
}
