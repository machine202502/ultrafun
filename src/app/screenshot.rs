use super::App;

impl App {
    pub(super) fn run_level_previews(&mut self) {
        let Some(queue) = &mut self.preview_queue else {
            return;
        };
        self.menu.visible = false;
        self.pause.visible = false;
        if let (Some(name), Some(state)) = (self.preview_capturing.take(), &self.window_state) {
            let directory = std::path::Path::new("assets/levels/previews");
            let path = directory.join(format!("{name}.png"));
            let outcome = std::fs::create_dir_all(directory)
                .map_err(anyhow::Error::from)
                .and_then(|()| {
                    state
                        .renderer
                        .capture()
                        .and_then(|image| Ok(image.save(&path)?))
                });
            match outcome {
                Ok(()) => log::info!("level preview saved: {}", path.display()),
                Err(error) => log::error!("cannot save {}: {error:#}", path.display()),
            }
        }
        match queue.pop() {
            Some(name) => {
                self.load_named_level(&name);
                self.preview_capturing = Some(name);
            }
            None => self.exit_requested = true,
        }
    }

    pub(super) fn stage_screenshot(&mut self) -> bool {
        let Some(run) = &self.screenshots else {
            return false;
        };
        let Some((preset, level, shot)) = run
            .current()
            .map(|(preset, level, shot)| (preset, level.to_owned(), shot.clone()))
        else {
            self.exit_requested = true;
            return true;
        };
        self.load_named_level(&level);
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
        let Some((preset, level, shot)) = run
            .current()
            .map(|(preset, level, shot)| (preset, level.to_owned(), shot.clone()))
        else {
            return;
        };
        if !run.ready_to_capture() {
            return;
        }
        let path = run.path_for(self.lab.preset_name(preset), &level, &shot);
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
