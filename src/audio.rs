use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use game::GameEvent;
use glam::Vec3;
use kira::sound::static_sound::StaticSoundData;
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend};

const AUDIO_DIRECTORY: &str = "assets/audio";
const SHOT_NAMES: [&str; 3] = ["shot_1", "shot_3", "shot_5"];
const SOUND_NAMES: [&str; 14] = [
    "shot_1",
    "shot_3",
    "shot_5",
    "impact",
    "enemy_hit",
    "enemy_killed",
    "part_collected",
    "player_damaged",
    "jump",
    "dash",
    "slam",
    "door",
    "chime",
    "radio",
];
const METERS_PER_DECIBEL: f32 = 2.0;
const QUIETEST_DECIBELS: f32 = -30.0;

pub struct Audio {
    manager: AudioManager,
    sounds: HashMap<&'static str, StaticSoundData>,
    shot_seed: u32,
    last_shot: usize,
}

impl Audio {
    pub fn new() -> Result<Self> {
        let manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .context("cannot open audio device")?;
        let sounds = SOUND_NAMES
            .iter()
            .map(|name| {
                let path = Path::new(AUDIO_DIRECTORY).join(format!("{name}.wav"));
                let sound = StaticSoundData::from_file(&path)
                    .with_context(|| format!("cannot load {}", path.display()))?;
                Ok((*name, sound))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            manager,
            sounds,
            shot_seed: 0x9e37_79b9,
            last_shot: usize::MAX,
        })
    }

    pub fn handle(&mut self, event: &GameEvent, listener: Vec3, master_decibels: f32) {
        let (name, source) = match event {
            GameEvent::ShotFired { .. } => (self.next_shot(), None),
            GameEvent::ShotImpact { position, .. } => ("impact", Some(*position)),
            GameEvent::EnemyHit { .. } => ("enemy_hit", None),
            GameEvent::EnemyKilled { position, .. } => ("enemy_killed", Some(*position)),
            GameEvent::PartCollected { .. } => ("part_collected", None),
            GameEvent::PlayerDamaged { .. } => ("player_damaged", None),
            GameEvent::Jumped => ("jump", None),
            GameEvent::Dashed => ("dash", None),
            GameEvent::Slammed { .. } => ("slam", None),
            GameEvent::ArenaStarted { .. } => ("door", None),
            GameEvent::ArenaCleared { .. } | GameEvent::LevelCompleted => ("chime", None),
            _ => return,
        };
        let falloff = source.map_or(0.0, |position| {
            distance_falloff_decibels(position.distance(listener))
        });
        self.play(name, master_decibels + falloff);
    }

    fn next_shot(&mut self) -> &'static str {
        self.shot_seed = self
            .shot_seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        let (name, index) = shot_variant(self.shot_seed, self.last_shot);
        self.last_shot = index;
        name
    }

    pub fn play_radio(&mut self, master_decibels: f32) {
        self.play("radio", master_decibels);
    }

    fn play(&mut self, name: &str, decibels: f32) {
        let Some(sound) = self.sounds.get(name) else {
            return;
        };
        if let Err(error) = self.manager.play(sound.volume(Decibels(decibels))) {
            log::warn!("cannot play {name}: {error}");
        }
    }
}

pub fn shot_variant(seed: u32, last: usize) -> (&'static str, usize) {
    let mut index = ((seed >> 16) as usize) % SHOT_NAMES.len();
    if SHOT_NAMES.len() > 1 && index == last {
        index = (index + 1 + ((seed >> 8) as usize) % (SHOT_NAMES.len() - 1)) % SHOT_NAMES.len();
    }
    (SHOT_NAMES[index], index)
}

pub fn distance_falloff_decibels(distance: f32) -> f32 {
    (-distance / METERS_PER_DECIBEL).max(QUIETEST_DECIBELS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falloff_is_one_decibel_per_two_meters_down_to_floor() {
        assert_eq!(distance_falloff_decibels(0.0), 0.0);
        assert_eq!(distance_falloff_decibels(10.0), -5.0);
        assert_eq!(distance_falloff_decibels(500.0), QUIETEST_DECIBELS);
    }

    #[test]
    fn every_sound_file_exists() {
        for name in SOUND_NAMES {
            let path = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(AUDIO_DIRECTORY)
                .join(format!("{name}.wav"));
            assert!(path.exists(), "missing {}", path.display());
        }
    }
}
#[cfg(test)]
mod shot_tests {
    use super::*;

    #[test]
    fn shots_use_every_variant_without_repeating() {
        let mut seed = 1u32;
        let mut last = usize::MAX;
        let mut seen = Vec::new();
        for _ in 0..200 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let (name, index) = shot_variant(seed, last);
            if last != usize::MAX {
                assert_ne!(index, last, "two identical shots in a row");
            }
            last = index;
            seen.push(name);
        }
        for name in SHOT_NAMES {
            assert!(seen.contains(&name), "variant {name} never plays");
        }
    }
}
