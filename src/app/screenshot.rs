use super::App;

impl App {
    pub(super) fn stage_screenshot(&mut self) -> bool {
        let Some(run) = &self.screenshots else {
            return false;
        };
        let Some((preset, shot)) = run.current().map(|(preset, shot)| (preset, shot.clone()))
        else {
            self.exit_requested = true;
            return true;
        };
        self.lab.select(preset);
        let player = &mut self.game.player;
        player.position = shot.position;
        player.previous_position = shot.position;
        player.yaw = shot.yaw_degrees.to_radians();
        player.pitch = shot.pitch_degrees.to_radians();
        self.unsimulated_seconds = 0.0;
        true
    }

    pub(super) fn capture_screenshot_if_ready(&mut self) {
        let (Some(run), Some(state)) = (&mut self.screenshots, &self.window_state) else {
            return;
        };
        let Some((preset, shot)) = run.current().map(|(preset, shot)| (preset, shot.clone()))
        else {
            return;
        };
        if !run.ready_to_capture() {
            return;
        }
        let path = run.path_for(self.lab.preset_name(preset), &shot);
        match state
            .renderer
            .capture()
            .and_then(|image| Ok(image.save(&path)?))
        {
            Ok(()) => log::info!("saved {}", path.display()),
            Err(error) => log::error!("cannot save {}: {error:#}", path.display()),
        }
        run.advance();
    }
}
