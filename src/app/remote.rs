use super::App;
use crate::remote_debug::{RemoteAppCommand, RemoteDebug, RemoteState};

impl App {
    pub(super) fn handle_remote_commands(&mut self) {
        let commands = self.remote.as_mut().map_or(Vec::new(), RemoteDebug::poll);
        for command in commands {
            match command {
                RemoteAppCommand::State => self.send_remote_state(),
                RemoteAppCommand::Reload => {
                    self.reload_changed_data();
                    self.remote_send("data reloaded");
                }
                RemoteAppCommand::SetLevel { name } => {
                    self.start_level(&name);
                    self.menu.visible = false;
                    self.pause.visible = false;
                    self.remote_send(&format!("level set to {name}"));
                }
            }
        }
    }

    pub(super) fn remote_pointer(&self) -> Option<(f32, f32)> {
        self.remote.as_ref().and_then(RemoteDebug::pointer)
    }

    pub(super) fn capture_remote_screenshot(&mut self) {
        let Some(path) = self
            .remote
            .as_mut()
            .and_then(RemoteDebug::take_screenshot_path)
        else {
            return;
        };
        let outcome = match &self.window_state {
            None => Err(anyhow::anyhow!("no window")),
            Some(state) => state
                .renderer
                .capture()
                .and_then(|image| Ok(image.save(&path)?)),
        };
        match outcome {
            Ok(()) => self.remote_send(&format!("screenshot saved: {path}")),
            Err(error) => self.remote_send(&format!("screenshot failed: {error:#}")),
        }
    }

    fn send_remote_state(&mut self) {
        let player = &self.game.player;
        let state = RemoteState {
            level: self.level_name.clone(),
            position: player.position.to_array(),
            eye: self.game.eye_position().to_array(),
            look_direction: game::look_direction(player.yaw, player.pitch).to_array(),
            yaw_degrees: player.yaw.to_degrees(),
            pitch_degrees: player.pitch.to_degrees(),
            health: player.health,
            elapsed_seconds: self.game.elapsed_seconds(),
            cheats: self.game.cheats,
            selected_weapon: format!("{:?}", self.game.weapon.selected),
            charge: self.game.charge_fraction(),
            beam: self.game.weapon.beam.is_some(),
            projectiles: self.game.projectiles.len(),
            enemies: self.game.enemies.len(),
        };
        match ron::to_string(&state) {
            Ok(text) => self.remote_send(&text),
            Err(error) => self.remote_send(&format!("cannot serialize state: {error}")),
        }
    }

    fn remote_send(&mut self, text: &str) {
        if let Some(remote) = &mut self.remote {
            remote.send(text);
        }
    }
}
