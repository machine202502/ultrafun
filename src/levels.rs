use std::path::Path;

use anyhow::{Result, ensure};

use crate::assets::DataFiles;
use crate::level_loader::{BlenderLevelFile, SceneDecor, load_blender_level};
use crate::render::{Atmosphere, PointLight, RoomAtmosphere, RoomLights};

pub const LEVELS_DIRECTORY: &str = "assets/levels";
pub const TEST_ROOM_LEVEL: &str = "test_room";
pub const TEST_ROOM_FILE: &str = "test_room.ron";
pub const SWITCHABLE_LEVELS: [&str; 5] = [
    "utilization",
    "foundry",
    "cooling",
    "laboratory",
    TEST_ROOM_LEVEL,
];

pub struct LoadedLevel {
    pub level: game::Level,
    pub atmosphere: Atmosphere,
    pub lights: Vec<PointLight>,
    pub decor: Vec<SceneDecor>,
}

pub fn load_level(data: &DataFiles, levels: &DataFiles, name: &str) -> Result<LoadedLevel> {
    if name == TEST_ROOM_LEVEL {
        let RoomAtmosphere { atmosphere } = data.load(TEST_ROOM_FILE)?;
        let RoomLights { lights } = data.load(TEST_ROOM_FILE)?;
        return Ok(LoadedLevel {
            level: data.load(TEST_ROOM_FILE)?,
            atmosphere,
            lights,
            decor: Vec::new(),
        });
    }
    let file: BlenderLevelFile = levels.load(&format!("{name}.ron"))?;
    let (level, lights, decor) = load_blender_level(Path::new(LEVELS_DIRECTORY), &file)?;
    Ok(LoadedLevel {
        level,
        atmosphere: file.atmosphere,
        lights,
        decor,
    })
}

pub fn next_level(current: &str) -> &'static str {
    SWITCHABLE_LEVELS
        .iter()
        .position(|name| *name == current)
        .map_or(SWITCHABLE_LEVELS[0], |index| {
            SWITCHABLE_LEVELS[(index + 1) % SWITCHABLE_LEVELS.len()]
        })
}

pub fn validate_level(name: &str) -> Result<String> {
    if name == TEST_ROOM_LEVEL {
        let data = DataFiles::open(crate::app::DATA_DIRECTORY)?;
        let level: game::Level = data.load(TEST_ROOM_FILE)?;
        ensure!(
            game::coplanar_face_overlaps(&level.boxes).is_empty(),
            "z-fighting in level geometry: {} boxes, test room has no object names",
            level.boxes.len()
        );
        return Ok(format!(
            "level {name}: {} boxes checked, no z-fighting",
            level.boxes.len()
        ));
    }
    let level_files = DataFiles::open(LEVELS_DIRECTORY)?;
    let mut file: BlenderLevelFile = level_files.load(&format!("{name}.ron"))?;
    file.validate_geometry = true;
    let (level, _, _) = load_blender_level(Path::new(LEVELS_DIRECTORY), &file)?;
    let solid_count = level.boxes.len()
        + level
            .doors
            .iter()
            .map(|door| door.zones.len())
            .sum::<usize>()
        + level
            .breakables
            .iter()
            .map(|breakable| breakable.zones.len())
            .sum::<usize>()
        + level.switches.len()
        + level.hazards.len();
    Ok(format!(
        "level {name}: {solid_count} solid boxes checked, no z-fighting"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_switch_in_a_cycle() {
        assert_eq!(next_level("utilization"), "foundry");
        assert_eq!(next_level("foundry"), "cooling");
        assert_eq!(next_level("cooling"), "laboratory");
        assert_eq!(next_level("laboratory"), TEST_ROOM_LEVEL);
        assert_eq!(next_level(TEST_ROOM_LEVEL), "utilization");
        assert_eq!(next_level("unknown"), "utilization");
    }
}
