use glam::Vec3;
use parry3d::math::Pose;
use parry3d::query::{Ray, RayCast};
use serde::Deserialize;

use crate::collision::{CollisionWorld, SKIN};
use crate::motion::{Body, BodyState, move_body};
use crate::player::MoveState;
use crate::{EnemySpawn, GameEvent, Player};

#[derive(Debug, Clone, Deserialize)]
pub struct GruntConfig {
    pub height: f32,
    pub radius: f32,
    pub max_health: f32,
    pub run_speed: f32,
    pub attack_range: f32,
    pub attack_reach_leeway: f32,
    pub windup_seconds: f32,
    pub damage: f32,
    pub recover_seconds: f32,
    pub gravity: f32,
    pub max_fall_speed: f32,
    pub step_height: f32,
    pub max_walkable_slope_degrees: f32,
}

impl GruntConfig {
    fn body(&self) -> Body {
        Body::new(
            self.height,
            self.radius,
            self.step_height,
            self.max_walkable_slope_degrees,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttackState {
    Ready,
    Windup { remaining_seconds: f32 },
    Recover { remaining_seconds: f32 },
}

#[derive(Debug, Clone)]
pub struct Enemy {
    pub id: u32,
    pub position: Vec3,
    pub previous_position: Vec3,
    pub velocity: Vec3,
    pub grounded: bool,
    pub health: f32,
    pub attack: AttackState,
}

impl Enemy {
    pub fn new(id: u32, position: Vec3, config: &GruntConfig) -> Enemy {
        let position = position + Vec3::Y * SKIN;
        Enemy {
            id,
            position,
            previous_position: position,
            velocity: Vec3::ZERO,
            grounded: false,
            health: config.max_health,
            attack: AttackState::Ready,
        }
    }

    pub fn from_spawns(spawns: &[EnemySpawn], config: &GruntConfig) -> Vec<Enemy> {
        spawns
            .iter()
            .zip(0..)
            .map(|(spawn, id)| Enemy::new(id, spawn.position, config))
            .collect()
    }

    pub fn interpolated_position(&self, alpha: f32) -> Vec3 {
        self.previous_position.lerp(self.position, alpha)
    }

    pub fn ray_distance(&self, ray: &Ray, max_distance: f32, config: &GruntConfig) -> Option<f32> {
        let body = config.body();
        body.shape.cast_ray(
            &Pose::from_translation(body.center(self.position)),
            ray,
            max_distance,
            true,
        )
    }
}

pub fn update_enemies(
    enemies: &mut [Enemy],
    player: &mut Player,
    config: &GruntConfig,
    world: &CollisionWorld,
    events: &mut Vec<GameEvent>,
    dt: f32,
) {
    let body = config.body();
    for enemy in enemies.iter_mut() {
        enemy.previous_position = enemy.position;
        let to_player = player.position - enemy.position;
        let flat_to_player = Vec3::new(to_player.x, 0.0, to_player.z);
        let distance = flat_to_player.length();

        enemy.attack = next_attack_state(enemy.attack, distance, player, config, events, dt);

        let chasing = !matches!(enemy.attack, AttackState::Windup { .. })
            && distance > config.attack_range * 0.75;
        let flat_velocity = if chasing {
            flat_to_player.normalize_or_zero() * config.run_speed
        } else {
            Vec3::ZERO
        };
        let fall = (enemy.velocity.y - config.gravity * dt).max(-config.max_fall_speed);
        enemy.velocity = flat_velocity + Vec3::Y * fall;

        let state = BodyState {
            position: &mut enemy.position,
            velocity: &mut enemy.velocity,
            grounded: &mut enemy.grounded,
        };
        move_body(&body, state, true, world, dt);
    }
    separate(enemies, config.radius);
}

fn next_attack_state(
    attack: AttackState,
    distance: f32,
    player: &mut Player,
    config: &GruntConfig,
    events: &mut Vec<GameEvent>,
    dt: f32,
) -> AttackState {
    match attack {
        AttackState::Ready if distance <= config.attack_range => AttackState::Windup {
            remaining_seconds: config.windup_seconds,
        },
        AttackState::Ready => AttackState::Ready,
        AttackState::Windup { remaining_seconds } if remaining_seconds > dt => {
            AttackState::Windup {
                remaining_seconds: remaining_seconds - dt,
            }
        }
        AttackState::Windup { .. } => {
            if distance <= config.attack_range + config.attack_reach_leeway {
                damage_player(player, config.damage, events);
            }
            AttackState::Recover {
                remaining_seconds: config.recover_seconds,
            }
        }
        AttackState::Recover { remaining_seconds } if remaining_seconds > dt => {
            AttackState::Recover {
                remaining_seconds: remaining_seconds - dt,
            }
        }
        AttackState::Recover { .. } => AttackState::Ready,
    }
}

fn damage_player(player: &mut Player, amount: f32, events: &mut Vec<GameEvent>) {
    let immune = matches!(player.state, MoveState::Dash { .. }) || player.health <= 0.0;
    if immune {
        return;
    }
    player.health = (player.health - amount).max(0.0);
    events.push(GameEvent::PlayerDamaged { amount });
}

fn separate(enemies: &mut [Enemy], radius: f32) {
    let min_distance = radius * 2.0;
    for first in 0..enemies.len() {
        for second in first + 1..enemies.len() {
            let delta = enemies[second].position - enemies[first].position;
            let flat = Vec3::new(delta.x, 0.0, delta.z);
            let distance = flat.length();
            if distance >= min_distance {
                continue;
            }
            let direction = if distance > f32::EPSILON {
                flat / distance
            } else {
                Vec3::X
            };
            let push = direction * (min_distance - distance) * 0.5;
            enemies[first].position -= push;
            enemies[second].position += push;
        }
    }
}
