mod scene;

use std::path::Path;

use anyhow::{Context, Result, bail};
use game::{ArenaSpawn, EnemySpawn, Level, LevelBox, MachineSpawn, TargetSpawn, Zone};
use glam::{EulerRot, Quat, Vec3};
use serde::Deserialize;

use crate::render::{Atmosphere, PointLight};

use scene::{LINEAR_TO_DISPLAY_GAMMA, SceneCube, SceneObjects, ScenePoint, read_scene};

const PLAYER_START: &str = "player_start";
const ENEMY: &str = "enemy";
const TARGET: &str = "target";
const EXIT: &str = "exit";
const ARENA_PREFIX: &str = "arena_";

#[derive(Debug, Clone, Deserialize)]
pub struct BlenderLevelFile {
    pub model: String,
    pub atmosphere: Atmosphere,
    pub target_health: f32,
    pub light_scale: f32,
    pub default_light_range: f32,
    #[serde(default)]
    pub arenas: Vec<ArenaRules>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArenaRules {
    pub trigger: String,
    pub doors: Vec<String>,
    #[serde(default)]
    pub waves: Vec<String>,
    #[serde(default)]
    pub machine: Option<MachineRules>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MachineRules {
    pub object: String,
    pub spawn_point: String,
    pub health: f32,
    pub spawn_interval_seconds: f32,
    pub max_alive: usize,
}

pub fn load_blender_level(
    directory: &Path,
    file: &BlenderLevelFile,
) -> Result<(Level, Vec<PointLight>)> {
    let path = directory.join(&file.model);
    let bytes = std::fs::read(&path).with_context(|| format!("cannot read {}", path.display()))?;
    let objects = read_scene(&bytes).with_context(|| format!("cannot load {}", path.display()))?;
    Ok((build_level(&objects, file)?, level_lights(&objects, file)))
}

pub fn level_lights(objects: &SceneObjects, file: &BlenderLevelFile) -> Vec<PointLight> {
    objects
        .lights
        .iter()
        .map(|light| PointLight {
            position: light.position,
            color: light
                .color
                .map(|channel| channel.powf(LINEAR_TO_DISPLAY_GAMMA)),
            intensity: light.candela * file.light_scale,
            range: light.range.unwrap_or(file.default_light_range),
        })
        .collect()
}

pub fn facing_yaw_degrees(rotation: Quat) -> f32 {
    let arrow = rotation * Vec3::Y;
    let flat = Vec3::new(arrow.x, 0.0, arrow.z);
    if flat.length() < 1e-3 {
        return 0.0;
    }
    flat.x.atan2(-flat.z).to_degrees()
}

pub fn build_level(objects: &SceneObjects, file: &BlenderLevelFile) -> Result<Level> {
    let start = single_point(objects, PLAYER_START)?;
    let boxes = objects
        .cubes
        .iter()
        .filter(|(name, _)| {
            !name.starts_with(ARENA_PREFIX) && name.as_str() != TARGET && name.as_str() != EXIT
        })
        .flat_map(|(_, cubes)| cubes)
        .map(level_box)
        .collect();
    let targets = cubes(objects, TARGET)
        .iter()
        .map(|cube| TargetSpawn {
            center: cube.center,
            size: cube.size,
            health: file.target_health,
        })
        .collect();
    let enemies = points(objects, ENEMY)
        .iter()
        .map(|point| EnemySpawn {
            position: point.position,
        })
        .collect();
    let arenas = file
        .arenas
        .iter()
        .map(|rules| build_arena(objects, rules))
        .collect::<Result<_>>()?;
    Ok(Level {
        spawn_position: start.position,
        spawn_yaw_degrees: facing_yaw_degrees(start.rotation),
        boxes,
        targets,
        enemies,
        arenas,
        exit: cubes(objects, EXIT).first().map(zone),
    })
}

fn build_arena(objects: &SceneObjects, rules: &ArenaRules) -> Result<ArenaSpawn> {
    let doors = rules
        .doors
        .iter()
        .map(|name| required_cubes(objects, name))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .map(zone)
        .collect();
    let waves = rules
        .waves
        .iter()
        .map(|name| {
            Ok(required_points(objects, name)?
                .iter()
                .map(|point| point.position)
                .collect())
        })
        .collect::<Result<_>>()?;
    let machine = rules
        .machine
        .as_ref()
        .map(|machine| -> Result<MachineSpawn> {
            let object = single_cube(objects, &machine.object)?;
            Ok(MachineSpawn {
                center: object.center,
                size: object.size,
                health: machine.health,
                spawn_point: single_point(objects, &machine.spawn_point)?.position,
                spawn_interval_seconds: machine.spawn_interval_seconds,
                max_alive: machine.max_alive,
            })
        })
        .transpose()?;
    Ok(ArenaSpawn {
        trigger: zone(single_cube(objects, &rules.trigger)?),
        doors,
        waves,
        machine,
    })
}

fn level_box(cube: &SceneCube) -> LevelBox {
    let (x, y, z) = cube.rotation.to_euler(EulerRot::XYZ);
    LevelBox {
        center: cube.center,
        size: cube.size,
        rotation_degrees: Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees()),
        color: cube.color,
    }
}

fn zone(cube: &SceneCube) -> Zone {
    Zone {
        center: cube.center,
        size: cube.size,
    }
}

fn cubes<'a>(objects: &'a SceneObjects, name: &str) -> &'a [SceneCube] {
    objects.cubes.get(name).map_or(&[], Vec::as_slice)
}

fn points<'a>(objects: &'a SceneObjects, name: &str) -> &'a [ScenePoint] {
    objects.points.get(name).map_or(&[], Vec::as_slice)
}

fn required_cubes<'a>(objects: &'a SceneObjects, name: &str) -> Result<&'a [SceneCube]> {
    match cubes(objects, name) {
        [] => bail!("level has no cube named \"{name}\""),
        found => Ok(found),
    }
}

fn required_points<'a>(objects: &'a SceneObjects, name: &str) -> Result<&'a [ScenePoint]> {
    match points(objects, name) {
        [] => bail!("level has no empty named \"{name}\""),
        found => Ok(found),
    }
}

fn single_cube<'a>(objects: &'a SceneObjects, name: &str) -> Result<&'a SceneCube> {
    Ok(&required_cubes(objects, name)?[0])
}

fn single_point<'a>(objects: &'a SceneObjects, name: &str) -> Result<&'a ScenePoint> {
    Ok(&required_points(objects, name)?[0])
}

#[cfg(test)]
mod tests;
