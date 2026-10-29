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
pub struct ShotList {
    pub level: String,
    pub shots: Vec<Shot>,
}

pub struct ScreenshotRun {
    output: PathBuf,
    shots: Vec<Shot>,
    preset_count: usize,
    job: usize,
    frames_waited: u32,
}

impl ScreenshotRun {
    pub fn load_shot_list() -> Result<ShotList> {
        let text = std::fs::read_to_string(SHOT_LIST)
            .with_context(|| format!("cannot read {SHOT_LIST}"))?;
        ron::from_str(&text).with_context(|| format!("cannot parse {SHOT_LIST}"))
    }

    pub fn new(output: PathBuf, shots: Vec<Shot>, preset_count: usize) -> Result<Self> {
        std::fs::create_dir_all(&output)
            .with_context(|| format!("cannot create {}", output.display()))?;
        Ok(Self {
            output,
            shots,
            preset_count,
            job: 0,
            frames_waited: 0,
        })
    }

    pub fn current(&self) -> Option<(usize, &Shot)> {
        let preset = self.job / self.shots.len().max(1);
        let shot = self.shots.get(self.job % self.shots.len().max(1))?;
        (preset < self.preset_count).then_some((preset, shot))
    }

    pub fn ready_to_capture(&mut self) -> bool {
        self.frames_waited += 1;
        self.frames_waited >= FRAMES_BEFORE_CAPTURE
    }

    pub fn path_for(&self, preset_name: &str, shot: &Shot) -> PathBuf {
        Path::new(&self.output).join(format!("{preset_name}__{}.png", shot.name))
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
    fn every_preset_gets_every_shot_then_run_ends() {
        let output = std::env::temp_dir().join("ultrafun_screenshot_test");
        let mut run = ScreenshotRun::new(output, vec![shot("a"), shot("b")], 2).unwrap();
        let mut jobs = Vec::new();
        while let Some((preset, shot)) = run.current() {
            jobs.push((preset, shot.name.clone()));
            run.advance();
        }
        assert_eq!(
            jobs,
            [(0, "a"), (0, "b"), (1, "a"), (1, "b")]
                .map(|(preset, name)| (preset, name.to_owned()))
        );
    }

    #[test]
    fn shipped_shot_list_parses() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(SHOT_LIST);
        let list: ShotList = ron::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert!(!list.shots.is_empty());
    }
}
