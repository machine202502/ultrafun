use std::time::Instant;

use glam::Vec3;

use super::{App, MAX_STEPS_PER_FRAME};
use crate::fx::{lamp_fixtures, muzzle_position};
use crate::overlay::{DebugReport, OverlayContent, ViewSample};
use crate::render::Camera;
use crate::ui::build_hud;

impl App {
    pub(super) fn frame(&mut self) {
        self.reload_changed_data();

        let now = Instant::now();
        let frame_seconds = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;
        self.unsimulated_seconds += frame_seconds;

        let sensitivity = self.settings.mouse_sensitivity;
        let staging_screenshot = self.stage_screenshot();
        let mut steps = 0;
        while !staging_screenshot
            && self.unsimulated_seconds >= game::STEP_SECONDS
            && steps < MAX_STEPS_PER_FRAME
        {
            let actions = self.input.take_actions(sensitivity, &self.settings.keys);
            self.game.update(&actions);
            self.unsimulated_seconds -= game::STEP_SECONDS;
            steps += 1;
        }
        if self.unsimulated_seconds >= game::STEP_SECONDS {
            self.unsimulated_seconds = 0.0;
        }
        let muzzle = muzzle_position(&self.game);
        let listener = self.game.eye_position();
        let master_decibels = self.settings.master_volume_db;
        for event in self.game.events.drain(..) {
            log::debug!("{event:?}");
            self.effects.handle(&event, muzzle);
            let new_line = self.dispatcher.handle(&event);
            if let Some(audio) = &mut self.audio {
                audio.handle(&event, listener, master_decibels);
                if new_line {
                    audio.play_radio(master_decibels);
                }
            }
        }
        self.effects.advance(frame_seconds);
        self.dispatcher.advance(frame_seconds);

        let alpha = self.unsimulated_seconds / game::STEP_SECONDS;
        let player = &self.game.player;
        let movement = &self.game.config.movement;
        let (yaw, pitch) =
            player.looked(self.input.pending_look(sensitivity), movement.max_pitch());
        let position = player.interpolated_position(alpha);
        let camera = Camera {
            eye: position + Vec3::Y * player.eye_height(movement),
            direction: game::look_direction(yaw, pitch),
            horizontal_fov: self.settings.horizontal_fov_degrees.to_radians(),
        };
        self.report = DebugReport::new(
            &self.game,
            ViewSample {
                frame_seconds,
                steps_last_frame: steps,
                position,
                yaw,
                pitch,
            },
        );

        let hud = build_hud(&self.game, &self.dispatcher, &self.texts);
        let dynamic_lights = self.effects.dynamic_lights(&self.game, alpha);
        let fixtures = lamp_fixtures(&self.level_lights, self.lab.style());
        let Some(state) = &mut self.window_state else {
            return;
        };
        state
            .renderer
            .set_dynamic_boxes(&self.effects.dynamic_boxes(&self.game, alpha), &fixtures);
        state.renderer.set_lighting(self.lab.style());
        let Some(mut frame) = state.renderer.begin_frame(&camera, &dynamic_lights) else {
            return;
        };
        let save_lighting = state.overlay.draw(
            &state.window,
            &state.renderer.device,
            &state.renderer.queue,
            &mut frame,
            OverlayContent {
                report: &self.report,
                hud: &hud,
                lab: &mut self.lab,
            },
        );
        state.renderer.finish_frame(frame);
        self.capture_screenshot_if_ready();
        if save_lighting {
            self.save_lighting();
        }
    }
}
