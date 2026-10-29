use anyhow::Result;

use super::{
    App, DATA_DIRECTORY, GRUNT_FILE, HEALTH_FILE, LIGHTING_FILE, MOVEMENT_FILE, SETTINGS_FILE,
    WEAPONS_FILE,
};
use crate::levels::{LoadedLevel, TEST_ROOM_FILE, TEST_ROOM_LEVEL, load_level, next_level};
use crate::render::LevelView;
use crate::settings::Settings;

impl App {
    pub(super) fn reload_changed_data(&mut self) {
        let level_changed = self
            .levels
            .changed_file_names()
            .iter()
            .any(|file_name| file_name.starts_with(&self.level_name));
        if level_changed {
            let result = self.reload_level();
            self.report_reload(&self.level_name, result);
        }
        for file_name in self.data.changed_file_names() {
            let result = match file_name.as_str() {
                TEST_ROOM_FILE if self.level_name == TEST_ROOM_LEVEL => self.reload_level(),
                SETTINGS_FILE => self.reload_settings(),
                LIGHTING_FILE => self
                    .data
                    .load(LIGHTING_FILE)
                    .map(|lighting| self.lab.reload(lighting)),
                MOVEMENT_FILE => self
                    .data
                    .load(MOVEMENT_FILE)
                    .map(|movement| self.game.config.movement = movement),
                WEAPONS_FILE => self
                    .data
                    .load(WEAPONS_FILE)
                    .map(|weapon| self.game.config.weapon = weapon),
                HEALTH_FILE => self
                    .data
                    .load(HEALTH_FILE)
                    .map(|health| self.game.config.health = health),
                GRUNT_FILE => self
                    .data
                    .load(GRUNT_FILE)
                    .map(|grunt| self.game.config.grunt = grunt),
                _ => continue,
            };
            self.report_reload(&file_name, result);
        }
    }

    fn report_reload(&self, what: &str, result: Result<()>) {
        match result {
            Ok(()) => log::info!("reloaded {what}"),
            Err(error) => log::error!("{error:#}"),
        }
    }

    fn reload_settings(&mut self) -> Result<()> {
        let settings: Settings = self.data.load(SETTINGS_FILE)?;
        if let Some(state) = &mut self.window_state {
            state.renderer.set_options(settings.render_options());
        }
        self.settings = settings;
        Ok(())
    }

    fn reload_level(&mut self) -> Result<()> {
        let LoadedLevel {
            level,
            atmosphere,
            lights,
        } = load_level(&self.data, &self.levels, &self.level_name)?;
        if let Some(state) = &mut self.window_state {
            state.renderer.set_level(LevelView {
                level: &level,
                atmosphere: atmosphere.clone(),
                lights: lights.clone(),
            });
        }
        self.game.set_level(level);
        self.atmosphere = atmosphere;
        self.level_lights = lights;
        Ok(())
    }

    pub(super) fn switch_level(&mut self) {
        let next = next_level(&self.level_name);
        let previous = std::mem::replace(&mut self.level_name, next.to_owned());
        if let Err(error) = self.reload_level() {
            log::error!("{error:#}");
            self.level_name = previous;
        }
    }

    pub(super) fn save_lighting(&mut self) {
        let file = self.lab.file_with_current_saved();
        let path = std::path::Path::new(DATA_DIRECTORY).join(LIGHTING_FILE);
        let result = ron::ser::to_string_pretty(&file, ron::ser::PrettyConfig::default())
            .map_err(anyhow::Error::from)
            .and_then(|text| std::fs::write(&path, text).map_err(anyhow::Error::from));
        match result {
            Ok(()) => log::info!("saved lighting preset \"{}\"", file.active),
            Err(error) => log::error!("cannot save {}: {error:#}", path.display()),
        }
    }
}
