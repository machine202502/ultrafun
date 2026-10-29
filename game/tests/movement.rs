use game::{Actions, Game, GameEvent, LevelBox, MoveState, STEP_SECONDS, look_direction};
use glam::{Vec2, Vec3};

mod common;
use common::*;

fn press(game: &mut Game, actions: Actions) {
    game.update(&actions);
}

fn forward() -> Actions {
    Actions {
        move_input: Vec2::Y,
        ..Actions::default()
    }
}

fn jump() -> Actions {
    Actions {
        jump_pressed: true,
        ..Actions::default()
    }
}

fn dash() -> Actions {
    Actions {
        dash_pressed: true,
        ..Actions::default()
    }
}

fn slide_held() -> Actions {
    Actions {
        slide_held: true,
        ..Actions::default()
    }
}

fn max_height_over(game: &mut Game, seconds: f32) -> f32 {
    let mut max_height = game.player.position.y;
    for _ in 0..steps(seconds) {
        game.update(&Actions::default());
        max_height = max_height.max(game.player.position.y);
    }
    max_height
}

#[test]
fn look_directions_follow_yaw() {
    assert!(look_direction(0.0, 0.0).abs_diff_eq(Vec3::NEG_Z, 1e-6));
    assert!(look_direction(std::f32::consts::FRAC_PI_2, 0.0).abs_diff_eq(Vec3::X, 1e-6));
}

#[test]
fn pitch_is_clamped() {
    let mut game = settled_game(Vec::new());
    let look_down = Actions {
        look: Vec2::new(0.0, 100.0),
        ..Actions::default()
    };
    press(&mut game, look_down);
    assert_eq!(game.player.pitch, -game.config.movement.max_pitch());
}

#[test]
fn respawn_returns_to_spawn_without_interpolation() {
    let mut game = settled_game(Vec::new());
    run(&mut game, forward(), 0.5);
    press(
        &mut game,
        Actions {
            respawn: true,
            ..Actions::default()
        },
    );
    assert!(game.player.position.abs_diff_eq(Vec3::ZERO, 0.02));
    assert_eq!(game.player.previous_position, game.player.position);
}

#[test]
fn running_reaches_run_speed() {
    let mut game = settled_game(Vec::new());
    run(&mut game, forward(), 0.3);
    assert!((game.player.horizontal_speed() - 16.5).abs() < 0.01);
}

#[test]
fn jump_reaches_expected_height() {
    let mut game = settled_game(Vec::new());
    press(&mut game, jump());
    let expected = game.config.movement.jump_height();
    let reached = max_height_over(&mut game, 1.5);
    assert!(
        (reached - expected).abs() < 0.15,
        "reached {reached}, expected {expected}"
    );
    assert!(game.player.grounded);
}

#[test]
fn dash_covers_expected_distance() {
    let mut game = settled_game(Vec::new());
    let start = game.player.position;
    press(&mut game, dash());
    run(&mut game, Actions::default(), 0.2 - STEP_SECONDS);
    let distance = (game.player.position - start).length();
    assert!((distance - 9.9).abs() < 0.5, "distance {distance}");
    assert_eq!(game.player.state, MoveState::Normal);
}

#[test]
fn dash_fails_without_stamina() {
    let mut game = settled_game(Vec::new());
    for _ in 0..3 {
        press(&mut game, dash());
        run(&mut game, Actions::default(), 0.21);
    }
    assert!(game.player.stamina < 1.0);
    press(&mut game, dash());
    assert!(!matches!(game.player.state, MoveState::Dash { .. }));
}

#[test]
fn stamina_regenerates_over_time() {
    let mut game = settled_game(Vec::new());
    game.player.stamina = 0.0;
    run(&mut game, Actions::default(), 1.0);
    assert!((game.player.stamina - 0.7).abs() < 0.02);
}

#[test]
fn stamina_does_not_regenerate_while_sliding() {
    let mut game = settled_game(Vec::new());
    game.player.stamina = 1.0;
    run(&mut game, slide_held(), 1.0);
    assert!((game.player.stamina - 1.0).abs() < 0.02);
}

#[test]
fn slide_starts_at_slide_speed_and_lowers_player() {
    let mut game = settled_game(Vec::new());
    run(&mut game, slide_held(), 0.1);
    assert!(matches!(game.player.state, MoveState::Slide { .. }));
    assert!(game.player.is_low);
    assert!((game.player.horizontal_speed() - 24.0).abs() < 0.01);
}

#[test]
fn slide_jump_keeps_slide_speed() {
    let mut game = settled_game(Vec::new());
    run(&mut game, slide_held(), 0.1);
    press(
        &mut game,
        Actions {
            jump_pressed: true,
            slide_held: true,
            ..Actions::default()
        },
    );
    run(&mut game, Actions::default(), 0.05);
    assert!(!game.player.grounded);
    assert!(game.player.horizontal_speed() > 23.0);
}

#[test]
fn dash_jump_keeps_dash_speed_and_costs_two_stamina() {
    let mut game = settled_game(Vec::new());
    press(&mut game, dash());
    press(&mut game, jump());
    run(&mut game, Actions::default(), 0.05);
    assert!(game.player.horizontal_speed() > 48.0);
    assert!(game.player.stamina < 1.1);
}

#[test]
fn wall_jumps_are_limited_and_reset_on_ground() {
    let wall = solid(Vec3::new(1.5, 10.0, 0.0), Vec3::new(1.0, 20.0, 10.0));
    let mut game = game_with(vec![wall], Vec3::new(0.4, 5.0, 0.0));
    let mut successful_jumps = 0;
    for _ in 0..4 {
        game.player.position = Vec3::new(0.4, 5.0, 0.0);
        game.player.velocity = Vec3::ZERO;
        press(&mut game, jump());
        if game.player.velocity.x < -10.0 {
            successful_jumps += 1;
        }
    }
    assert_eq!(successful_jumps, 3);
    game.player.position = Vec3::new(-3.0, 0.5, 0.0);
    run(&mut game, Actions::default(), 0.5);
    assert!(game.player.grounded);
    assert_eq!(game.player.wall_jumps_used, 0);
}

#[test]
fn wall_cling_slows_falling() {
    let wall = solid(Vec3::new(1.5, 20.0, 0.0), Vec3::new(1.0, 40.0, 10.0));
    let mut game = game_with(vec![wall], Vec3::new(0.4, 20.0, 0.0));
    game.player.yaw = std::f32::consts::FRAC_PI_2;
    run(&mut game, forward(), 0.1);
    assert!(
        game.player.velocity.y > -4.0,
        "velocity {}",
        game.player.velocity.y
    );
}

#[test]
fn slam_falls_fast_and_bounces_from_start_height() {
    let mut game = game_with(Vec::new(), Vec3::new(0.0, 10.0, 0.0));
    press(
        &mut game,
        Actions {
            slide_pressed: true,
            slide_held: true,
            ..Actions::default()
        },
    );
    assert_eq!(game.player.velocity.y, -100.0);
    for _ in 0..steps(1.0) {
        if game.player.grounded {
            break;
        }
        press(&mut game, Actions::default());
    }
    assert!(game.player.grounded);
    assert!(
        game.events
            .iter()
            .any(|event| matches!(event, GameEvent::Slammed { .. }))
    );
    press(&mut game, jump());
    let reached = max_height_over(&mut game, 2.0);
    assert!((reached - 10.0).abs() < 0.4, "reached {reached}");
}

#[test]
fn dash_does_not_pass_through_thin_wall() {
    let wall = solid(Vec3::new(0.0, 5.0, -5.0), Vec3::new(20.0, 10.0, 0.1));
    let mut game = settled_game(vec![wall]);
    press(&mut game, dash());
    run(&mut game, Actions::default(), 0.3);
    assert!(
        game.player.position.z > -4.5,
        "z {}",
        game.player.position.z
    );
}

#[test]
fn slam_does_not_pass_through_thin_floor() {
    let platform = solid(Vec3::new(0.0, 20.0, 0.0), Vec3::new(10.0, 0.1, 10.0));
    let mut game = game_with(vec![platform], Vec3::new(0.0, 60.0, 0.0));
    press(
        &mut game,
        Actions {
            slide_pressed: true,
            ..Actions::default()
        },
    );
    run(&mut game, Actions::default(), 1.0);
    assert!(
        game.player.position.y > 20.0,
        "y {}",
        game.player.position.y
    );
}

#[test]
fn stairs_are_climbable() {
    let mut stairs: Vec<LevelBox> = (1..=4)
        .map(|index| {
            let height = 0.4 * index as f32;
            solid(
                Vec3::new(0.0, height * 0.5, -2.0 - index as f32),
                Vec3::new(4.0, height, 1.0),
            )
        })
        .collect();
    stairs.push(solid(Vec3::new(0.0, 1.0, -26.5), Vec3::new(4.0, 2.0, 40.0)));
    let mut game = settled_game(stairs);
    run(&mut game, forward(), 1.5);
    assert!(
        (game.player.position.y - 2.0).abs() < 0.1,
        "y {}",
        game.player.position.y
    );
}

#[test]
fn steep_ramp_is_not_climbable_by_running() {
    let ramp = LevelBox {
        rotation_degrees: Vec3::new(50.0, 0.0, 0.0),
        ..solid(Vec3::new(0.0, 3.7, -8.0), Vec3::new(4.0, 0.5, 10.0))
    };
    let mut game = settled_game(vec![ramp]);
    run(&mut game, forward(), 2.0);
    assert!(game.player.position.y < 1.0, "y {}", game.player.position.y);
}

#[test]
fn gentle_ramp_is_climbable() {
    let ramp = LevelBox {
        rotation_degrees: Vec3::new(30.0, 0.0, 0.0),
        ..solid(Vec3::new(0.0, 2.4, -8.0), Vec3::new(4.0, 0.5, 10.0))
    };
    let mut game = settled_game(vec![ramp]);
    run(&mut game, forward(), 0.6);
    assert!(game.player.position.y > 3.0, "y {}", game.player.position.y);
}

#[test]
fn player_stays_low_under_low_ceiling() {
    let ceiling = solid(Vec3::new(0.0, 2.7, 0.0), Vec3::new(10.0, 2.6, 10.0));
    let mut game = game_with(vec![ceiling], Vec3::ZERO);
    game.player.is_low = true;
    run(&mut game, Actions::default(), 0.3);
    assert!(game.player.is_low);
    assert!(game.player.position.y < 0.1);
}

#[test]
fn player_exactly_touching_floor_is_not_pushed_through_it() {
    for is_low in [false, true] {
        let mut game = game_with(Vec::new(), Vec3::ZERO);
        game.player.position = Vec3::ZERO;
        game.player.is_low = is_low;
        run(&mut game, Actions::default(), 0.2);
        assert!(
            game.player.position.y > -0.01,
            "is_low {is_low} y {}",
            game.player.position.y
        );
        assert!(game.player.grounded);
    }
}

#[test]
fn jump_and_dash_emit_events() {
    let mut game = settled_game(Vec::new());
    game.events.clear();
    press(&mut game, jump());
    assert!(game.events.contains(&GameEvent::Jumped));
    press(&mut game, dash());
    assert!(game.events.contains(&GameEvent::Dashed));
}
#[test]
fn falling_out_of_the_map_kills_the_player() {
    let mut game = game_with_enemies(Vec::new(), Vec::new(), Vec3::new(0.0, -100.0, 0.0));
    run(&mut game, Actions::default(), 0.05);
    assert_eq!(game.player.health, 0.0);
}

#[test]
fn jumping_down_a_floor_inside_the_map_does_not_kill() {
    let floor = solid(Vec3::new(0.0, -1.0, 0.0), Vec3::new(40.0, 1.0, 40.0));
    let mut game = game_with_enemies(vec![floor], Vec::new(), Vec3::new(0.0, 12.0, 0.0));
    run(&mut game, Actions::default(), 1.5);
    assert!(game.player.health > 0.0, "health {}", game.player.health);
}
#[test]
fn cheats_fly_through_walls_and_up() {
    let wall = solid(Vec3::new(0.0, 2.0, -2.0), Vec3::new(20.0, 4.0, 0.5));
    let mut game = game_with_enemies(vec![wall], Vec::new(), Vec3::ZERO);

    let toggle = Actions {
        toggle_cheats: true,
        ..Actions::default()
    };
    game.update(&toggle);
    assert!(game.cheats);

    let forward = Actions {
        move_input: Vec2::Y,
        ..Actions::default()
    };
    run(&mut game, forward, 0.5);
    assert!(
        game.player.position.z < -3.0,
        "player did not pass through the wall: {}",
        game.player.position
    );

    game.player.pitch = 1.2;
    run(&mut game, forward, 0.3);
    assert!(
        game.player.position.y > 1.0,
        "player did not fly up: {}",
        game.player.position
    );
}

#[test]
fn cheats_can_be_switched_off_again() {
    let floor = solid(Vec3::new(0.0, -1.0, 0.0), Vec3::new(40.0, 1.0, 40.0));
    let mut game = game_with_enemies(vec![floor], Vec::new(), Vec3::new(0.0, 5.0, 0.0));
    let toggle = Actions {
        toggle_cheats: true,
        ..Actions::default()
    };
    game.update(&toggle);
    let up = Actions {
        move_input: Vec2::Y,
        ..Actions::default()
    };
    game.player.pitch = 1.4;
    run(&mut game, up, 0.3);
    assert!(
        game.player.position.y > 6.0,
        "did not fly: {}",
        game.player.position.y
    );

    game.update(&toggle);
    assert!(!game.cheats);
    game.player.pitch = 0.0;
    run(&mut game, Actions::default(), 1.5);
    assert!(
        (game.player.position.y - 0.01).abs() < 0.2,
        "did not fall back to the floor: {}",
        game.player.position.y
    );
}
