use glam::Vec3;
use serde::Deserialize;

use crate::collision::SKIN;
use crate::{ArenaSpawn, Zone};
use crate::{MovementConfig, Player};

#[derive(Debug, Clone, Deserialize)]
pub struct Level {
    pub spawn_position: Vec3,
    #[serde(default)]
    pub spawn_yaw_degrees: f32,
    pub boxes: Vec<LevelBox>,
    #[serde(default)]
    pub targets: Vec<TargetSpawn>,
    #[serde(default)]
    pub enemies: Vec<EnemySpawn>,
    #[serde(default)]
    pub arenas: Vec<ArenaSpawn>,
    #[serde(default)]
    pub exit: Option<Zone>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TargetSpawn {
    pub center: Vec3,
    pub size: Vec3,
    pub health: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LevelBox {
    pub center: Vec3,
    pub size: Vec3,
    #[serde(default)]
    pub rotation_degrees: Vec3,
    pub color: [f32; 3],
}

impl Level {
    pub fn spawn_player(&self, config: &MovementConfig, health: f32) -> Player {
        Player::spawned_at(
            self.spawn_position + Vec3::Y * SKIN,
            self.spawn_yaw_degrees.to_radians(),
            config.stamina_max,
            health,
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnemySpawn {
    pub position: Vec3,
}
