use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};

use anyhow::{Context, Result};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::de::DeserializeOwned;

pub struct DataFiles {
    directory: PathBuf,
    changes: Receiver<notify::Result<Event>>,
    _watcher: RecommendedWatcher,
}

impl DataFiles {
    pub fn open(directory: impl Into<PathBuf>) -> Result<Self> {
        let directory = directory.into();
        let (sender, changes) = channel();
        let mut watcher = notify::recommended_watcher(sender)?;
        watcher
            .watch(&directory, RecursiveMode::NonRecursive)
            .with_context(|| format!("cannot watch {}", directory.display()))?;
        Ok(Self {
            directory,
            changes,
            _watcher: watcher,
        })
    }

    pub fn load<T: DeserializeOwned>(&self, file_name: &str) -> Result<T> {
        load_ron(&self.directory.join(file_name))
    }

    pub fn changed_file_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .changes
            .try_iter()
            .filter_map(Result::ok)
            .filter(|event| event.kind.is_modify() || event.kind.is_create())
            .flat_map(|event| event.paths)
            .filter_map(|path| Some(path.file_name()?.to_string_lossy().into_owned()))
            .collect();
        names.sort();
        names.dedup();
        names
    }
}

fn load_ron<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    ron::from_str(&text).with_context(|| format!("cannot parse {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_path(file_name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/data")
            .join(file_name)
    }

    #[test]
    fn shipped_data_files_parse() {
        load_ron::<game::Level>(&data_path("test_room.ron")).unwrap();
        load_ron::<crate::render::RoomAtmosphere>(&data_path("test_room.ron")).unwrap();
        let room_lights: crate::render::RoomLights = load_ron(&data_path("test_room.ron")).unwrap();
        assert_eq!(room_lights.lights.len(), 3);
        load_ron::<game::MovementConfig>(&data_path("movement.ron")).unwrap();
        load_ron::<game::WeaponConfig>(&data_path("weapons.ron")).unwrap();
        load_ron::<game::HealthConfig>(&data_path("health.ron")).unwrap();
        load_ron::<game::GruntConfig>(&data_path("grunt.ron")).unwrap();
        load_ron::<crate::settings::Settings>(&data_path("settings_default.ron")).unwrap();
        let lighting: crate::render::LightingFile = load_ron(&data_path("lighting.ron")).unwrap();
        assert!(lighting.presets.len() >= 4);
        assert_eq!(
            lighting.presets[lighting.active_index()].name,
            lighting.active
        );
        let english: std::collections::HashMap<String, String> =
            load_ron(&Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/text/en.ron")).unwrap();
        assert!(english.contains_key("dispatcher.reject_announced"));
    }

    #[test]
    fn test_room_arena_starts_when_walking_north_through_doorway() {
        let config = game::GameConfig {
            movement: load_ron(&data_path("movement.ron")).unwrap(),
            weapon: load_ron(&data_path("weapons.ron")).unwrap(),
            health: load_ron(&data_path("health.ron")).unwrap(),
            grunt: load_ron(&data_path("grunt.ron")).unwrap(),
        };
        let level = load_ron(&data_path("test_room.ron")).unwrap();
        let mut game = game::Game::new(level, config);
        let forward = game::Actions {
            move_input: glam::Vec2::Y,
            ..game::Actions::default()
        };
        for _ in 0..(4.0 / game::STEP_SECONDS) as usize {
            game.update(&forward);
        }
        assert_eq!(game.arenas[0].phase, game::ArenaPhase::Active);
        assert!(game.player.position.z < -33.0);
        assert_eq!(game.closed_doors().count(), 1);
    }
}
