use glam::Vec3;
use parry3d::shape::Capsule;

use crate::collision::{CollisionWorld, SKIN};
use crate::player::MoveState;
use crate::{MovementConfig, Player};

const MAX_SLIDE_ITERATIONS: usize = 4;
const GROUND_PROBE_DISTANCE: f32 = 0.05;
const MIN_MOTION: f32 = 1e-6;
const STEP_UP_MIN_GAIN: f32 = 1e-3;

pub struct Body {
    pub shape: Capsule,
    pub half_height: f32,
    pub radius: f32,
    pub step_height: f32,
    pub min_ground_normal_y: f32,
}

impl Body {
    pub fn new(height: f32, radius: f32, step_height: f32, max_slope_degrees: f32) -> Self {
        let half_height = height * 0.5;
        Self {
            shape: Capsule::new_y(half_height - radius, radius),
            half_height,
            radius,
            step_height,
            min_ground_normal_y: max_slope_degrees.to_radians().cos(),
        }
    }

    pub fn center(&self, feet: Vec3) -> Vec3 {
        feet + Vec3::Y * self.half_height
    }
}

pub struct BodyState<'a> {
    pub position: &'a mut Vec3,
    pub velocity: &'a mut Vec3,
    pub grounded: &'a mut bool,
}

pub fn player_body(config: &MovementConfig, is_low: bool) -> Body {
    let height = if is_low {
        config.slide_height
    } else {
        config.standing_height
    };
    Body::new(
        height,
        config.radius,
        config.step_height,
        config.max_walkable_slope_degrees,
    )
}

pub fn can_stand(player: &Player, config: &MovementConfig, world: &CollisionWorld) -> bool {
    let standing = player_body(config, false);
    !world.overlaps(&standing.shape, standing.center(player.position))
}

pub fn move_player(player: &mut Player, config: &MovementConfig, world: &CollisionWorld, dt: f32) {
    let body = player_body(config, player.is_low);
    let snap = !matches!(player.state, MoveState::Slam { .. });
    let state = BodyState {
        position: &mut player.position,
        velocity: &mut player.velocity,
        grounded: &mut player.grounded,
    };
    move_body(&body, state, snap, world, dt);
}

pub fn carry_player(
    player: &mut Player,
    config: &MovementConfig,
    world: &CollisionWorld,
    motion: Vec3,
) {
    let body = player_body(config, player.is_low);
    let state = BodyState {
        position: &mut player.position,
        velocity: &mut player.velocity,
        grounded: &mut player.grounded,
    };
    carry_body(&body, state, world, motion);
}

pub fn carry_body(body: &Body, state: BodyState, world: &CollisionWorld, motion: Vec3) {
    let center = body.center(*state.position);
    let (center_after, _) = slide_move(world, body, center, Vec3::ZERO, motion, *state.grounded);
    *state.position = center_after - Vec3::Y * body.half_height;
}

pub fn move_body(body: &Body, state: BodyState, snap: bool, world: &CollisionWorld, dt: f32) {
    let mut center = body.center(*state.position);
    depenetrate(world, body, &mut center);

    let motion = *state.velocity * dt;
    let (mut center_after, mut velocity) = slide_move(
        world,
        body,
        center,
        *state.velocity,
        motion,
        *state.grounded,
    );

    let wanted = horizontal_length(motion);
    if *state.grounded && wanted > MIN_MOTION {
        let gained = horizontal_length(center_after - center);
        if gained < wanted * 0.99
            && let Some((stepped, stepped_velocity)) =
                step_up_move(world, body, center, *state.velocity, motion)
            && horizontal_length(stepped - center) > gained + STEP_UP_MIN_GAIN
        {
            center_after = stepped;
            velocity = stepped_velocity;
        }
    }

    let probe = if *state.grounded && snap {
        body.step_height
    } else {
        GROUND_PROBE_DISTANCE
    };
    *state.grounded = false;
    if velocity.y <= 0.0
        && let Some(gap) = ground_gap(world, body, center_after, probe)
    {
        center_after.y += SKIN - gap;
        velocity.y = 0.0;
        *state.grounded = true;
    }

    *state.position = center_after - Vec3::Y * body.half_height;
    *state.velocity = velocity;
}

pub fn wall_normal(
    player: &Player,
    config: &MovementConfig,
    world: &CollisionWorld,
) -> Option<Vec3> {
    let body = player_body(config, player.is_low);
    world
        .touches(
            &body.shape,
            body.center(player.position),
            config.wall_contact_distance,
        )
        .into_iter()
        .filter(|touch| touch.normal.y.abs() <= config.max_wall_normal_y())
        .min_by(|a, b| a.distance.total_cmp(&b.distance))
        .map(|touch| Vec3::new(touch.normal.x, 0.0, touch.normal.z).normalize())
}

fn depenetrate(world: &CollisionWorld, body: &Body, center: &mut Vec3) {
    for touch in world.touches(&body.shape, *center, 0.0) {
        if touch.distance < 0.0 && -touch.distance < body.radius {
            *center += touch.normal * (SKIN - touch.distance);
        }
    }
}

fn slide_move(
    world: &CollisionWorld,
    body: &Body,
    start: Vec3,
    velocity: Vec3,
    motion: Vec3,
    grounded: bool,
) -> (Vec3, Vec3) {
    let mut center = start;
    let mut velocity = velocity;
    let mut remaining = motion;
    for _ in 0..MAX_SLIDE_ITERATIONS {
        let length = remaining.length();
        if length < MIN_MOTION {
            break;
        }
        let Some(hit) = world.cast(&body.shape, center, remaining) else {
            center += remaining;
            break;
        };

        let travel = (hit.fraction * length - SKIN).max(0.0);
        center += remaining / length * travel;
        remaining *= 1.0 - hit.fraction;
        let normal = blocking_normal(hit.normal, grounded, body);
        remaining -= normal * remaining.dot(normal).min(0.0);
        velocity -= normal * velocity.dot(normal).min(0.0);
    }
    (center, velocity)
}

fn blocking_normal(normal: Vec3, grounded: bool, body: &Body) -> Vec3 {
    let too_steep = normal.y > 0.0 && normal.y < body.min_ground_normal_y;
    if grounded && too_steep {
        Vec3::new(normal.x, 0.0, normal.z).normalize_or(normal)
    } else {
        normal
    }
}

fn step_up_move(
    world: &CollisionWorld,
    body: &Body,
    start: Vec3,
    velocity: Vec3,
    motion: Vec3,
) -> Option<(Vec3, Vec3)> {
    let up = Vec3::Y * body.step_height;
    let raised = match world.cast(&body.shape, start, up) {
        Some(hit) => start + Vec3::Y * (hit.fraction * body.step_height - SKIN).max(0.0),
        None => start + up,
    };
    let flat_velocity = Vec3::new(velocity.x, 0.0, velocity.z);
    let flat_motion = Vec3::new(motion.x, 0.0, motion.z);
    let (moved, moved_velocity) = slide_move(world, body, raised, flat_velocity, flat_motion, true);
    let drop_length = moved.y - start.y + SKIN * 2.0;
    let hit = world.cast(&body.shape, moved, Vec3::NEG_Y * drop_length)?;
    if hit.surface_normal.y < body.min_ground_normal_y {
        return None;
    }
    let landed = moved - Vec3::Y * (hit.fraction * drop_length - SKIN).max(0.0);
    Some((landed, moved_velocity))
}

fn ground_gap(world: &CollisionWorld, body: &Body, center: Vec3, probe: f32) -> Option<f32> {
    let length = probe + SKIN;
    let hit = world.cast(&body.shape, center, Vec3::NEG_Y * length)?;
    (hit.surface_normal.y >= body.min_ground_normal_y).then_some(hit.fraction * length)
}

fn horizontal_length(vector: Vec3) -> f32 {
    Vec3::new(vector.x, 0.0, vector.z).length()
}
