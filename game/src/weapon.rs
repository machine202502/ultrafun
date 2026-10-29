use glam::Vec3;
use parry3d::math::Pose;
use parry3d::query::{Ray, RayCast};
use parry3d::shape::Cuboid;
use serde::Deserialize;

use crate::TargetSpawn;
use crate::collision::CollisionWorld;
use crate::enemy::{Enemy, GruntConfig};
use crate::script::Switch;

#[derive(Debug, Clone, Deserialize)]
pub struct WeaponConfig {
    pub damage: f32,
    pub cooldown_seconds: f32,
    pub range: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetKind {
    Practice,
    Machine,
}

#[derive(Debug, Clone)]
pub struct Target {
    pub id: u32,
    pub center: Vec3,
    pub size: Vec3,
    pub max_health: f32,
    pub health: f32,
    pub kind: TargetKind,
}

impl Target {
    pub fn from_spawns(spawns: &[TargetSpawn]) -> Vec<Target> {
        spawns
            .iter()
            .zip(0..)
            .map(|(spawn, id)| Target {
                id,
                center: spawn.center,
                size: spawn.size,
                max_health: spawn.health,
                health: spawn.health,
                kind: TargetKind::Practice,
            })
            .collect()
    }

    fn ray_distance(&self, ray: &Ray, max_distance: f32) -> Option<f32> {
        Cuboid::new(self.size * 0.5).cast_ray(
            &Pose::from_translation(self.center),
            ray,
            max_distance,
            true,
        )
    }
}

pub enum ShotHit {
    Nothing,
    Wall { distance: f32, normal: Vec3 },
    Target { index: usize, distance: f32 },
    Switch { index: usize, distance: f32 },
    Enemy { index: usize, distance: f32 },
}

impl ShotHit {
    pub fn distance(&self, range: f32) -> f32 {
        match self {
            ShotHit::Nothing => range,
            ShotHit::Wall { distance, .. }
            | ShotHit::Target { distance, .. }
            | ShotHit::Switch { distance, .. }
            | ShotHit::Enemy { distance, .. } => *distance,
        }
    }
}

#[derive(Clone, Copy)]
enum HitKind {
    Target(usize),
    Switch(usize),
    Enemy(usize),
}

pub fn trace_shot(
    ray: &Ray,
    range: f32,
    world: &CollisionWorld,
    targets: &[Target],
    switches: &[Switch],
    enemies: &[Enemy],
    grunt: &GruntConfig,
) -> ShotHit {
    let wall = world.ray(ray, range);
    let limit = wall.map_or(range, |(distance, _)| distance);
    let hits = targets
        .iter()
        .enumerate()
        .filter_map(|(index, target)| {
            Some((HitKind::Target(index), target.ray_distance(ray, limit)?))
        })
        .chain(switches.iter().enumerate().filter_map(|(index, switch)| {
            Some((HitKind::Switch(index), switch.ray_distance(ray, limit)?))
        }))
        .chain(enemies.iter().enumerate().filter_map(|(index, enemy)| {
            Some((
                HitKind::Enemy(index),
                enemy.ray_distance(ray, limit, grunt)?,
            ))
        }));
    match hits.min_by(|(_, a), (_, b)| a.total_cmp(b)) {
        Some((HitKind::Target(index), distance)) => ShotHit::Target { index, distance },
        Some((HitKind::Switch(index), distance)) => ShotHit::Switch { index, distance },
        Some((HitKind::Enemy(index), distance)) => ShotHit::Enemy { index, distance },
        None => match wall {
            Some((distance, normal)) => ShotHit::Wall { distance, normal },
            None => ShotHit::Nothing,
        },
    }
}
