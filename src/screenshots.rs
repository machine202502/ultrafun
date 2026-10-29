use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use glam::Vec3;
use serde::Deserialize;

const SHOT_LIST: &str = "tools/screenshots.ron";
const FRAMES_BEFORE_CAPTURE: u32 = 3;

#[derive(Debug, Clone, Deserialize)]
pub struct Shot {
    pub name: String,
    pub position: Vec3,
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShotLevel {
    pub level: String,
    pub shots: Vec<Shot>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShotList {
    pub levels: Vec<ShotLevel>,
}

pub struct ScreenshotArgs {
    pub output: PathBuf,
    pub preset: Option<String>,
    pub level: Option<String>,
    pub shot: Option<String>,
}

impl ShotList {
    pub fn filtered(self, level: Option<&str>, shot: Option<&str>) -> ShotList {
        ShotList {
            levels: self
                .levels
                .into_iter()
                .filter(|entry| level.is_none_or(|name| entry.level == name))
                .map(|entry| ShotLevel {
                    level: entry.level,
                    shots: entry
                        .shots
                        .into_iter()
                        .filter(|shot_entry| {
                            shot.is_none_or(|prefix| shot_entry.name.starts_with(prefix))
                        })
                        .collect(),
                })
                .filter(|entry| !entry.shots.is_empty())
                .collect(),
        }
    }
}

pub struct ScreenshotRun {
    output: PathBuf,
    shots: Vec<(String, Shot)>,
    presets: Vec<usize>,
    job: usize,
    frames_waited: u32,
}

impl ScreenshotRun {
    pub fn load_shot_list() -> Result<ShotList> {
        let text = std::fs::read_to_string(SHOT_LIST)
            .with_context(|| format!("cannot read {SHOT_LIST}"))?;
        ron::from_str(&text).with_context(|| format!("cannot parse {SHOT_LIST}"))
    }

    pub fn new(output: PathBuf, levels: Vec<ShotLevel>, presets: Vec<usize>) -> Result<Self> {
        std::fs::create_dir_all(&output)
            .with_context(|| format!("cannot create {}", output.display()))?;
        let shots = levels
            .into_iter()
            .flat_map(|level| {
                level
                    .shots
                    .into_iter()
                    .map(move |shot| (level.level.clone(), shot))
            })
            .collect();
        Ok(Self {
            output,
            shots,
            presets,
            job: 0,
            frames_waited: 0,
        })
    }

    pub fn current(&self) -> Option<(usize, &str, &Shot)> {
        let count = self.shots.len().max(1);
        let preset = *self.presets.get(self.job / count)?;
        let (level, shot) = self.shots.get(self.job % count)?;
        Some((preset, level.as_str(), shot))
    }

    pub fn ready_to_capture(&mut self) -> bool {
        self.frames_waited += 1;
        self.frames_waited >= FRAMES_BEFORE_CAPTURE
    }

    pub fn path_for(&self, preset_name: &str, level: &str, shot: &Shot) -> PathBuf {
        Path::new(&self.output).join(format!("{preset_name}__{level}__{}.png", shot.name))
    }

    pub fn advance(&mut self) {
        self.job += 1;
        self.frames_waited = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(name: &str) -> Shot {
        Shot {
            name: name.into(),
            position: Vec3::ZERO,
            yaw_degrees: 0.0,
            pitch_degrees: 0.0,
        }
    }

    #[test]
    fn every_preset_gets_every_shot_of_every_level_then_run_ends() {
        let output = std::env::temp_dir().join("ultrafun_screenshot_test");
        let levels = vec![
            ShotLevel {
                level: "a".into(),
                shots: vec![shot("1")],
            },
            ShotLevel {
                level: "b".into(),
                shots: vec![shot("2"), shot("3")],
            },
        ];
        let mut run = ScreenshotRun::new(output, levels, vec![0, 1]).unwrap();
        let mut jobs = Vec::new();
        while let Some((preset, level, shot)) = run.current() {
            jobs.push((preset, level.to_owned(), shot.name.clone()));
            run.advance();
        }
        assert_eq!(
            jobs,
            [
                (0, "a", "1"),
                (0, "b", "2"),
                (0, "b", "3"),
                (1, "a", "1"),
                (1, "b", "2"),
                (1, "b", "3"),
            ]
            .map(|(preset, level, name)| (preset, level.to_owned(), name.to_owned()))
        );
    }

    #[test]
    fn shipped_shot_list_parses() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(SHOT_LIST);
        let list: ShotList = ron::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert!(list.levels.len() >= 2);
        assert!(list.levels.iter().all(|level| !level.shots.is_empty()));
    }
}
