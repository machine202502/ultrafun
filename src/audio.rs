use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result};
use game::GameEvent;
use glam::Vec3;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::{AudioManager, AudioManagerSettings, Decibels, DefaultBackend, Tween};

const AUDIO_DIRECTORY: &str = "assets/audio";
const SHOT_NAMES: [&str; 3] = ["shot_1", "shot_3", "shot_5"];
const SOUND_NAMES: [&str; 20] = [
    "rivet_hot",
    "sand_shot",
    "air_blast",
    "plasma_discharge",
    "plasma_loop",
    "charge_loop",
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
const MUSIC_NAMES: [&str; 2] = ["music_calm", "music_battle"];
const MUSIC_HEADROOM_DECIBELS: f32 = -8.0;
const METERS_PER_DECIBEL: f32 = 2.0;
const QUIETEST_DECIBELS: f32 = -30.0;

pub struct Audio<B: kira::backend::Backend = DefaultBackend> {
    manager: AudioManager<B>,
    sounds: HashMap<&'static str, StaticSoundData>,
    music: HashMap<&'static str, StaticSoundData>,
    music_handle: Option<StaticSoundHandle>,
    music_playing: Option<&'static str>,
    shot_seed: u32,
    last_shot: usize,
    attack_loop: Option<StaticSoundHandle>,
    attack_playing: Option<&'static str>,
    master_decibels: Option<f32>,
    impact_budget: u8,
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
        let music = MUSIC_NAMES
            .iter()
            .map(|name| {
                let path = Path::new(AUDIO_DIRECTORY).join(format!("{name}.wav"));
                let sound = StaticSoundData::from_file(&path)
                    .with_context(|| format!("cannot load {}", path.display()))?
                    .loop_region(..);
                Ok((*name, sound))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            manager,
            sounds,
            music,
            music_handle: None,
            music_playing: None,
            shot_seed: 0x9e37_79b9,
            last_shot: usize::MAX,
            attack_loop: None,
            attack_playing: None,
            master_decibels: None,
            impact_budget: 4,
        })
    }
}

impl<B: kira::backend::Backend> Audio<B> {
    pub fn handle(&mut self, event: &GameEvent, listener: Vec3) {
        if matches!(
            event,
            GameEvent::ShotImpact { .. } | GameEvent::EnemyHit { .. }
        ) {
            if self.impact_budget == 0 {
                return;
            }
            self.impact_budget -= 1;
        }
        let (name, source) = match event {
            GameEvent::ShotFired { weapon, mode, .. } => {
                use game::{FireMode::*, WeaponKind::*};
                let name = match (weapon, mode) {
                    (RivetGun, Primary) => self.next_shot(),
                    (RivetGun, Secondary) => "rivet_hot",
                    (Sandblaster, Primary) => "sand_shot",
                    (Sandblaster, Secondary) => "air_blast",
                    (PlasmaCutter, Secondary) => "plasma_discharge",
                    (PlasmaCutter, Primary) => return,
                };
                (name, None)
            }
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
        self.play(name, falloff);
    }

    pub fn sync(&mut self, game: &game::Game, master_decibels: f32, playing: bool) {
        self.impact_budget = 4;
        if self.master_decibels != Some(master_decibels) {
            self.manager
                .main_track()
                .set_volume(Decibels(master_decibels), Tween::default());
            self.master_decibels = Some(master_decibels);
        }
        let desired =
            if !playing || game.restart_countdown_seconds.is_some() || game.result.is_some() {
                None
            } else if game.weapon.charge > 0.0 {
                Some("charge_loop")
            } else if game.weapon.beam.is_some() {
                Some("plasma_loop")
            } else {
                None
            };
        if self.attack_playing != desired {
            if let Some(mut handle) = self.attack_loop.take() {
                handle.stop(Tween::default());
            }
            self.attack_playing = None;
            if let Some(name) = desired
                && let Some(sound) = self.sounds.get(name)
                && let Ok(handle) = self
                    .manager
                    .play(sound.loop_region(..).volume(Decibels(-12.0)))
            {
                self.attack_loop = Some(handle);
                self.attack_playing = Some(name);
            }
        }
        let battle = game
            .arenas
            .iter()
            .any(|arena| arena.phase == game::ArenaPhase::Active);
        self.switch_music(if battle { "music_battle" } else { "music_calm" });
    }

    fn switch_music(&mut self, name: &'static str) {
        if self.music_playing == Some(name) {
            return;
        }
        let Some(sound) = self.music.get(name) else {
            return;
        };
        let fade = Tween {
            duration: Duration::from_millis(150),
            ..Tween::default()
        };
        match self.manager.play(
            sound
                .volume(Decibels(MUSIC_HEADROOM_DECIBELS))
                .fade_in_tween(Some(fade)),
        ) {
            Ok(handle) => {
                if let Some(mut old) = self.music_handle.take() {
                    old.stop(fade);
                }
                self.music_handle = Some(handle);
                self.music_playing = Some(name);
            }
            Err(error) => log::warn!("cannot play {name}: {error}"),
        }
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

    pub fn play_radio(&mut self) {
        self.play("radio", 0.0);
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
    fn music_crossfades_do_not_accumulate_infinite_loops() {
        use kira::backend::mock::{MockBackend, MockBackendSettings};
        let manager = AudioManager::<MockBackend>::new(AudioManagerSettings {
            backend_settings: MockBackendSettings { sample_rate: 1000 },
            ..AudioManagerSettings::default()
        })
        .unwrap();
        let sound = StaticSoundData::from_file(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/audio/music_calm.wav"),
        )
        .unwrap()
        .loop_region(..);
        let mut audio = Audio {
            manager,
            sounds: HashMap::new(),
            music: HashMap::from([("music_calm", sound.clone()), ("music_battle", sound)]),
            music_handle: None,
            music_playing: None,
            shot_seed: 0,
            last_shot: usize::MAX,
            attack_loop: None,
            attack_playing: None,
            master_decibels: None,
            impact_budget: 4,
        };
        for name in [
            "music_calm",
            "music_battle",
            "music_calm",
            "music_battle",
            "music_calm",
        ] {
            audio.switch_music(name);
            for _ in 0..8 {
                audio.manager.backend_mut().on_start_processing();
                audio.manager.backend_mut().process();
            }
        }
        audio.manager.backend_mut().on_start_processing();
        assert_eq!(audio.manager.main_track().num_sounds(), 1);
        audio.switch_music("music_calm");
        assert_eq!(audio.manager.main_track().num_sounds(), 1);
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
