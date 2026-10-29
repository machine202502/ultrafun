use std::time::Instant;

use glam::Vec3;

use super::{App, MAX_STEPS_PER_FRAME};
use crate::fx::{glowing_boxes, lamp_fixtures, muzzle_position, viewmodel_boxes};
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
        if self.menu.visible {
            self.unsimulated_seconds = 0.0;
            self.set_mouse_captured(false);
        }
        if self.pause.visible {
            self.unsimulated_seconds = 0.0;
        }
        while !staging_screenshot
            && !self.menu.visible
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
        let alpha = self.unsimulated_seconds / game::STEP_SECONDS;
        let player = &self.game.player;
        let movement = &self.game.config.movement;
        let (yaw, pitch) =
            player.looked(self.input.pending_look(sensitivity), movement.max_pitch());
        let kick = self.effects.view_kick();
        let yaw = yaw + kick.x;
        let pitch = (pitch + kick.y).clamp(-movement.max_pitch(), movement.max_pitch());
        let position = player.interpolated_position(alpha);
        let eye = position + Vec3::Y * player.eye_height(movement);
        let direction = game::look_direction(yaw, pitch);
        let muzzle = muzzle_position(eye, direction);
        let listener = self.game.eye_position();
        let master_decibels = self.settings.master_volume_db;
        if self.menu.visible {
            self.game.events.clear();
        }
        let events = if self.menu.visible {
            Vec::new()
        } else {
            std::mem::take(&mut self.game.events)
        };
        for event in events {
            log::debug!("{event:?}");
            let floor = match &event {
                game::GameEvent::EnemyHit { position, .. } => {
                    Some(self.game.ground_height(*position))
                }
                _ => None,
            };
            self.effects.handle(&event, muzzle, floor);
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

        let camera = Camera {
            eye,
            direction,
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
        let mut glowing = lamp_fixtures(&self.level_lights, self.lab.style());
        glowing.extend(glowing_boxes(&self.game));
        glowing.extend(self.effects.glowing_boxes());
        let Some(state) = &mut self.window_state else {
            return;
        };
        let mut dynamic = self.effects.dynamic_boxes(&self.game, alpha);
        let speed = Vec3::new(player.velocity.x, 0.0, player.velocity.z).length();
        dynamic.extend(viewmodel_boxes(
            eye,
            direction,
            speed,
            self.game.elapsed_seconds(),
            self.effects.recoil(),
        ));
        state.renderer.set_dynamic_boxes(&dynamic, &glowing);
        state.renderer.set_lighting(self.lab.style());
        let Some(mut frame) = state.renderer.begin_frame(&camera, &dynamic_lights) else {
            return;
        };
        let (save_lighting, menu_action, pause_action) = state.overlay.draw(
            &state.window,
            &state.renderer.device,
            &state.renderer.queue,
            &mut frame,
            OverlayContent {
                report: &self.report,
                hud: &hud,
                lab: &mut self.lab,
                menu: Some(&mut self.menu),
                pause: Some(&mut self.pause),
            },
        );
        state.renderer.finish_frame(frame);
        let menu_action = self.pending_menu.take().or(menu_action);
        let pause_action = self.pending_pause.take().or(pause_action);
        match menu_action {
            Some(crate::menu::MenuAction::Load(name)) => {
                log::info!("menu load {name}");
                self.start_level(&name);
                self.menu.visible = false;
                self.pause.visible = false;
                self.set_mouse_captured(true);
            }
            Some(crate::menu::MenuAction::Exit) => self.exit_requested = true,
            None => {}
        }
        match pause_action {
            Some(crate::menu::PauseAction::Resume) => {
                log::info!("pause resume");
                self.pause.visible = false;
                self.set_mouse_captured(true);
            }
            Some(crate::menu::PauseAction::MainMenu) => {
                log::info!("pause main menu");
                self.pause.visible = false;
                self.menu.visible = true;
            }
            None => {}
        }

        self.capture_screenshot_if_ready();
        if save_lighting {
            self.save_lighting();
        }
    }
}
