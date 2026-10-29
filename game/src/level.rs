use glam::Vec3;
use serde::Deserialize;

use crate::collision::SKIN;
use crate::hazards::{BeltSpawn, CrusherSpawn, HazardSpawn, PadSpawn, TubeSpawn};
use crate::script::{BreakableSpawn, DoorSpawn, KeySpawn, SpawnGroup, SwitchSpawn, TriggerSpawn};
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
    pub hazards: Vec<HazardSpawn>,
    #[serde(default)]
    pub pads: Vec<PadSpawn>,
    #[serde(default)]
    pub belts: Vec<BeltSpawn>,
    #[serde(default)]
    pub crushers: Vec<CrusherSpawn>,
    #[serde(default)]
    pub tubes: Vec<TubeSpawn>,
    #[serde(default)]
    pub doors: Vec<DoorSpawn>,
    #[serde(default)]
    pub keys: Vec<KeySpawn>,
    #[serde(default)]
    pub breakables: Vec<BreakableSpawn>,
    #[serde(default)]
    pub switches: Vec<SwitchSpawn>,
    #[serde(default)]
    pub triggers: Vec<TriggerSpawn>,
    #[serde(default)]
    pub spawns: Vec<SpawnGroup>,
    #[serde(default)]
    pub checkpoints: Vec<Zone>,
    #[serde(default)]
    pub next_level: Option<String>,
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
