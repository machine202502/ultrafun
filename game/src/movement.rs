use glam::{Vec2, Vec3};

use crate::collision::CollisionWorld;
use crate::motion::{can_stand, move_player, wall_normal};
use crate::player::{MoveState, flat_direction};
use crate::{Actions, GameEvent, MovementConfig, Player, STEP_SECONDS};

pub fn step_player(
    player: &mut Player,
    actions: &Actions,
    config: &MovementConfig,
    world: &CollisionWorld,
    events: &mut Vec<GameEvent>,
) {
    let dt = STEP_SECONDS;
    player.previous_position = player.position;
    player.look(actions.look, config.max_pitch());
    update_timers(player, actions, config, dt);

    let wish = flat_direction(player.yaw, actions.move_input);
    let aim = if wish == Vec3::ZERO {
        flat_direction(player.yaw, Vec2::Y)
    } else {
        wish
    };

    if actions.dash_pressed {
        try_start_dash(player, config, aim, events);
    }
    if actions.slide_pressed && !player.grounded {
        try_start_slam(player, config);
    }
    if actions.slide_held && player.grounded {
        try_start_slide(player, config, aim);
    }

    match player.state {
        MoveState::Dash { .. } => update_dash(player, config, dt),
        MoveState::Slide { .. } => update_slide(player, actions, config, world, dt),
        MoveState::Slam { .. } => player.velocity = Vec3::NEG_Y * config.slam_speed,
        MoveState::Normal => update_normal(player, config, wish, dt),
    }

    try_jump(player, actions, config, world, events);
    if player.state == MoveState::Normal && !player.grounded {
        apply_wall_cling(player, config, world, wish, dt);
    }

    let was_grounded = player.grounded;
    move_player(player, config, world, dt);

    if player.grounded {
        player.wall_jumps_used = 0;
        player.wall_cling_seconds = 0.0;
        if !was_grounded {
            land(player, config, events);
        }
    } else {
        if was_grounded && player.velocity.y <= 0.0 {
            player.coyote_seconds = config.coyote_seconds;
        }
        if matches!(player.state, MoveState::Slide { .. }) {
            player.state = MoveState::Normal;
        }
    }

    if player.is_low
        && !matches!(player.state, MoveState::Slide { .. })
        && can_stand(player, config, world)
    {
        player.is_low = false;
    }
}

fn update_timers(player: &mut Player, actions: &Actions, config: &MovementConfig, dt: f32) {
    player.coyote_seconds = (player.coyote_seconds - dt).max(0.0);
    player.slam_bounce_seconds = (player.slam_bounce_seconds - dt).max(0.0);
    player.jump_buffer_seconds = if actions.jump_pressed {
        config.jump_buffer_seconds
    } else {
        (player.jump_buffer_seconds - dt).max(0.0)
    };
    if !matches!(player.state, MoveState::Slide { .. }) {
        player.stamina =
            (player.stamina + config.stamina_regen_per_second * dt).min(config.stamina_max);
    }
}

fn try_start_dash(
    player: &mut Player,
    config: &MovementConfig,
    direction: Vec3,
    events: &mut Vec<GameEvent>,
) {
    if matches!(player.state, MoveState::Slam { .. }) || player.stamina < config.dash_cost {
        return;
    }
    player.stamina -= config.dash_cost;
    player.state = MoveState::Dash {
        direction,
        remaining_seconds: config.dash_seconds,
    };
    player.velocity = direction * config.dash_speed;
    events.push(GameEvent::Dashed);
}

fn try_start_slam(player: &mut Player, config: &MovementConfig) {
    if matches!(player.state, MoveState::Normal | MoveState::Dash { .. }) {
        player.state = MoveState::Slam {
            start_height: player.position.y,
        };
        player.velocity = Vec3::NEG_Y * config.slam_speed;
    }
}

fn try_start_slide(player: &mut Player, config: &MovementConfig, aim: Vec3) {
    let direction = match player.state {
        MoveState::Normal => aim,
        MoveState::Dash { direction, .. } => direction,
        MoveState::Slide { .. } | MoveState::Slam { .. } => return,
    };
    let speed = player
        .horizontal_speed()
        .clamp(config.slide_speed, config.slide_max_speed);
    player.state = MoveState::Slide { direction, speed };
    player.is_low = true;
}

fn update_dash(player: &mut Player, config: &MovementConfig, dt: f32) {
    let MoveState::Dash {
        direction,
        remaining_seconds,
    } = player.state
    else {
        return;
    };
    let remaining_seconds = remaining_seconds - dt;
    if remaining_seconds <= 0.0 {
        player.velocity = direction * config.run_speed;
        player.state = MoveState::Normal;
    } else {
        player.velocity = direction * config.dash_speed;
        player.state = MoveState::Dash {
            direction,
            remaining_seconds,
        };
    }
}

fn update_slide(
    player: &mut Player,
    actions: &Actions,
    config: &MovementConfig,
    world: &CollisionWorld,
    dt: f32,
) {
    let MoveState::Slide { direction, speed } = player.state else {
        return;
    };
    if !actions.slide_held && can_stand(player, config, world) {
        player.state = MoveState::Normal;
        update_normal(player, config, Vec3::ZERO, dt);
        return;
    }
    let speed = if speed > config.slide_speed {
        (speed - config.slide_friction * dt).max(config.slide_speed)
    } else {
        speed
    };
    player.state = MoveState::Slide { direction, speed };
    let fall = apply_gravity(player.velocity.y, config, dt);
    player.velocity = direction * speed + Vec3::Y * fall;
}

fn update_normal(player: &mut Player, config: &MovementConfig, wish: Vec3, dt: f32) {
    let flat = Vec3::new(player.velocity.x, 0.0, player.velocity.z);
    let flat = if player.grounded {
        ground_velocity(flat, wish, config, dt)
    } else {
        air_velocity(flat, wish, config, dt)
    };
    player.velocity = flat + Vec3::Y * apply_gravity(player.velocity.y, config, dt);
}

fn ground_velocity(flat: Vec3, wish: Vec3, config: &MovementConfig, dt: f32) -> Vec3 {
    let speed = flat.length();
    if speed > config.run_speed {
        return flat * ((speed - config.ground_excess_friction * dt).max(config.run_speed) / speed);
    }
    let target = wish * config.run_speed;
    let difference = target - flat;
    let max_change = config.ground_acceleration() * dt;
    if difference.length() <= max_change {
        target
    } else {
        flat + difference.normalize() * max_change
    }
}

fn air_velocity(flat: Vec3, wish: Vec3, config: &MovementConfig, dt: f32) -> Vec3 {
    let along_wish = flat.dot(wish);
    let added = (config.run_speed - along_wish).clamp(0.0, config.air_acceleration * dt);
    let flat = flat + wish * added;
    let speed = flat.length();
    if speed > config.run_speed {
        flat * ((speed - config.air_excess_friction * dt).max(config.run_speed) / speed)
    } else {
        flat
    }
}

fn apply_gravity(vertical_speed: f32, config: &MovementConfig, dt: f32) -> f32 {
    (vertical_speed - config.gravity * dt).max(-config.max_fall_speed)
}

fn try_jump(
    player: &mut Player,
    actions: &Actions,
    config: &MovementConfig,
    world: &CollisionWorld,
    events: &mut Vec<GameEvent>,
) {
    if player.jump_buffer_seconds <= 0.0 {
        return;
    }
    let on_ground = player.grounded || player.coyote_seconds > 0.0;
    match player.state {
        MoveState::Slam { .. } => return,
        MoveState::Dash { direction, .. } if on_ground => {
            let flat_speed = if player.stamina >= config.dash_jump_cost {
                player.stamina -= config.dash_jump_cost;
                config.dash_speed
            } else {
                config.run_speed
            };
            player.velocity = direction * flat_speed + Vec3::Y * jump_speed(player, config);
        }
        _ if on_ground => player.velocity.y = jump_speed(player, config),
        _ => {
            if !try_wall_jump(player, actions, config, world) {
                return;
            }
        }
    }
    player.state = MoveState::Normal;
    player.jump_buffer_seconds = 0.0;
    events.push(GameEvent::Jumped);
    player.coyote_seconds = 0.0;
    player.slam_bounce_seconds = 0.0;
    player.grounded = false;
}

fn jump_speed(player: &Player, config: &MovementConfig) -> f32 {
    if player.slam_bounce_seconds > 0.0 {
        (2.0 * config.gravity * player.slam_bounce_height).sqrt()
    } else {
        config.jump_speed
    }
}

fn try_wall_jump(
    player: &mut Player,
    actions: &Actions,
    config: &MovementConfig,
    world: &CollisionWorld,
) -> bool {
    if player.wall_jumps_used >= config.wall_jumps_max {
        return false;
    }
    let Some(normal) = wall_normal(player, config, world) else {
        return false;
    };
    if actions.slide_held {
        player.velocity.y = config.slide_wall_jump_up_speed;
    } else {
        player.velocity =
            normal * config.wall_jump_away_speed + Vec3::Y * config.wall_jump_up_speed;
    }
    player.wall_jumps_used += 1;
    true
}

fn apply_wall_cling(
    player: &mut Player,
    config: &MovementConfig,
    world: &CollisionWorld,
    wish: Vec3,
    dt: f32,
) {
    if player.velocity.y >= 0.0 {
        return;
    }
    let Some(normal) = wall_normal(player, config, world) else {
        return;
    };
    if wish.dot(normal) >= 0.0 {
        return;
    }
    let limit = config.wall_cling_start_fall_speed
        + config.wall_cling_fall_speed_growth * player.wall_cling_seconds;
    player.velocity.y = player.velocity.y.max(-limit);
    player.wall_cling_seconds += dt;
}

fn land(player: &mut Player, config: &MovementConfig, events: &mut Vec<GameEvent>) {
    if let MoveState::Slam { start_height } = player.state {
        let fall_height = (start_height - player.position.y).max(0.0);
        events.push(GameEvent::Slammed {
            position: player.position,
            fall_height,
        });
        player.slam_bounce_seconds = config.slam_bounce_seconds;
        player.slam_bounce_height =
            fall_height.clamp(config.jump_height(), config.slam_bounce_max_height);
        player.state = MoveState::Normal;
        player.velocity = Vec3::ZERO;
    }
}
