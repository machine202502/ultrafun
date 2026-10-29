use glam::Vec3;
use parry3d::query::Ray;

use crate::collision::CollisionWorld;
use crate::{GameEvent, HealthConfig, Player};

const PART_REST_OFFSET: f32 = 0.05;

#[derive(Debug, Clone)]
pub struct RepairPart {
    pub position: Vec3,
    pub previous_position: Vec3,
    pub velocity: Vec3,
    pub remaining_seconds: f32,
    pub resting: bool,
}

impl RepairPart {
    pub fn interpolated_position(&self, alpha: f32) -> Vec3 {
        self.previous_position.lerp(self.position, alpha)
    }
}

pub fn spawn_parts(at: Vec3, seed: u32, config: &HealthConfig, parts: &mut Vec<RepairPart>) {
    let count = config.parts_per_kill.max(1);
    for index in 0..count {
        let angle = index as f32 / count as f32 * std::f32::consts::TAU + seed as f32 * 0.7;
        let outward = Vec3::new(angle.cos(), 0.0, angle.sin());
        parts.push(RepairPart {
            position: at,
            previous_position: at,
            velocity: (outward * 0.7 + Vec3::Y) * config.part_launch_speed,
            remaining_seconds: config.part_lifetime_seconds,
            resting: false,
        });
    }
}

pub fn update_parts(
    parts: &mut Vec<RepairPart>,
    player: &mut Player,
    collect_point: Vec3,
    config: &HealthConfig,
    world: &CollisionWorld,
    events: &mut Vec<GameEvent>,
    dt: f32,
) {
    let player_alive = player.health > 0.0;
    parts.retain_mut(|part| {
        part.previous_position = part.position;
        part.remaining_seconds -= dt;
        let to_player = collect_point - part.position;
        let distance = to_player.length();

        if player_alive && distance <= config.part_collect_radius {
            player.health = (player.health + config.part_heal).min(config.max_health);
            events.push(GameEvent::PartCollected {
                heal: config.part_heal,
            });
            return false;
        }

        if player_alive && distance <= config.part_attract_radius {
            part.resting = false;
            part.velocity = to_player / distance * config.part_attract_speed;
            part.position += part.velocity * dt;
        } else if !part.resting {
            part.velocity.y -= config.part_gravity * dt;
            fall(part, world, dt);
        }
        part.remaining_seconds > 0.0
    });
}

fn fall(part: &mut RepairPart, world: &CollisionWorld, dt: f32) {
    let motion = part.velocity * dt;
    let length = motion.length();
    if length <= f32::EPSILON {
        return;
    }
    let direction = motion / length;
    match world.ray(&Ray::new(part.position, direction), length) {
        Some((distance, _)) => {
            part.position += direction * (distance - PART_REST_OFFSET).max(0.0);
            part.velocity = Vec3::ZERO;
            part.resting = true;
        }
        None => part.position += motion,
    }
}
