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
    pub navigation: Vec<crate::NavNode>,
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
    #[serde(default)]
    pub material: Option<String>,
    pub color: [f32; 3],
}

pub const COPLANAR_PLANE_EPSILON: f32 = 0.001;
const FACE_OVERLAP_EPSILON: f32 = 0.0001;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoplanarOverlap {
    pub first: usize,
    pub second: usize,
    pub axis: usize,
    pub plane: f32,
}

struct Bounds {
    min: Vec3,
    max: Vec3,
}

impl Bounds {
    fn new(level_box: &LevelBox) -> Self {
        let half = level_box.size * 0.5;
        Bounds {
            min: level_box.center - half,
            max: level_box.center + half,
        }
    }
}

pub fn coplanar_face_overlaps(boxes: &[LevelBox]) -> Vec<CoplanarOverlap> {
    let bounds: Vec<Bounds> = boxes.iter().map(Bounds::new).collect();
    let mut overlaps = Vec::new();
    for first in 0..bounds.len() {
        for second in first + 1..bounds.len() {
            for axis in 0..3 {
                let Some(plane) =
                    same_direction_shared_plane(&bounds[first], &bounds[second], axis)
                else {
                    continue;
                };
                if faces_overlap(&bounds[first], &bounds[second], axis) {
                    overlaps.push(CoplanarOverlap {
                        first,
                        second,
                        axis,
                        plane,
                    });
                }
            }
        }
    }
    overlaps
}

fn same_direction_shared_plane(first: &Bounds, second: &Bounds, axis: usize) -> Option<f32> {
    if (first.max[axis] - second.max[axis]).abs() <= COPLANAR_PLANE_EPSILON {
        Some(first.max[axis])
    } else if (first.min[axis] - second.min[axis]).abs() <= COPLANAR_PLANE_EPSILON {
        Some(first.min[axis])
    } else {
        None
    }
}

fn faces_overlap(first: &Bounds, second: &Bounds, axis: usize) -> bool {
    let (u, v) = match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    overlap_length(first, second, u) > FACE_OVERLAP_EPSILON
        && overlap_length(first, second, v) > FACE_OVERLAP_EPSILON
}

fn overlap_length(first: &Bounds, second: &Bounds, axis: usize) -> f32 {
    (first.max[axis].min(second.max[axis]) - first.min[axis].max(second.min[axis])).max(0.0)
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
