use std::path::Path;

use anyhow::Result;

use crate::assets::DataFiles;
use crate::level_loader::{BlenderLevelFile, load_blender_level};
use crate::render::{Atmosphere, PointLight, RoomAtmosphere, RoomLights};

pub const LEVELS_DIRECTORY: &str = "assets/levels";
pub const TEST_ROOM_LEVEL: &str = "test_room";
pub const TEST_ROOM_FILE: &str = "test_room.ron";
const SWITCHABLE_LEVELS: [&str; 2] = ["utilization", TEST_ROOM_LEVEL];

pub struct LoadedLevel {
    pub level: game::Level,
    pub atmosphere: Atmosphere,
    pub lights: Vec<PointLight>,
}

pub fn load_level(data: &DataFiles, levels: &DataFiles, name: &str) -> Result<LoadedLevel> {
    if name == TEST_ROOM_LEVEL {
        let RoomAtmosphere { atmosphere } = data.load(TEST_ROOM_FILE)?;
        let RoomLights { lights } = data.load(TEST_ROOM_FILE)?;
        return Ok(LoadedLevel {
            level: data.load(TEST_ROOM_FILE)?,
            atmosphere,
            lights,
        });
    }
    let file: BlenderLevelFile = levels.load(&format!("{name}.ron"))?;
    let (level, lights) = load_blender_level(Path::new(LEVELS_DIRECTORY), &file)?;
    Ok(LoadedLevel {
        level,
        atmosphere: file.atmosphere,
        lights,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_switch_in_a_cycle() {
        assert_eq!(next_level("utilization"), TEST_ROOM_LEVEL);
        assert_eq!(next_level(TEST_ROOM_LEVEL), "utilization");
        assert_eq!(next_level("unknown"), "utilization");
    }
}
