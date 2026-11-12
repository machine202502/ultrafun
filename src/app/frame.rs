use std::time::Instant;

use glam::Vec3;

use super::{App, MAX_STEPS_PER_FRAME};
use crate::fx::{glowing_boxes, lamp_fixtures, muzzle_position};
use crate::overlay::{DebugReport, OverlayContent, ViewSample};
use crate::render::Camera;
use crate::ui::build_hud;

impl App {
    pub(super) fn frame(&mut self) {
        self.reload_changed_data();
        self.handle_remote_commands();
        self.run_level_previews();

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
            && !self.pause.visible
            && self.unsimulated_seconds >= game::STEP_SECONDS
            && steps < MAX_STEPS_PER_FRAME
        {
            let mut actions = self.input.take_actions(sensitivity, &self.settings.keys);
            if let Some(remote) = &mut self.remote {
                remote.merge_actions(&mut actions);
            }
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
        let position = player.interpolated_position(alpha);
        let eye = position + Vec3::Y * player.eye_height(movement);
        let direction = game::look_direction(yaw, pitch);
        let speed = Vec3::new(player.velocity.x, 0.0, player.velocity.z).length();
        let viewmodels = crate::animation::viewmodel(&self.game, speed, self.effects.recoil());
        let muzzle = self
            .window_state
            .as_ref()
            .and_then(|state| {
                viewmodels
                    .first()
                    .and_then(|model| state.renderer.model_muzzle(model))
            })
            .map(|local| {
                let right = direction.cross(Vec3::Y).normalize_or_zero();
                let up = right.cross(direction);
                eye + right * local.x + up * local.y - direction * local.z
            })
            .unwrap_or_else(|| muzzle_position(eye, direction));
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
        if let Some(audio) = &mut self.audio {
            audio.sync(
                &self.game,
                master_decibels,
                !self.menu.visible && !self.pause.visible,
            );
        }
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
                audio.handle(&event, listener);
                if new_line {
                    audio.play_radio();
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

        let hud = if self.preview_queue.is_some() {
            crate::ui::Hud {
                health_fraction: 1.0,
                dead: false,
                cheats: None,
                subtitle: None,
                keys: Vec::new(),
                report: None,
                weapon: None,
            }
        } else {
            build_hud(&self.game, &self.dispatcher, &self.texts)
        };
        let remote_pointer = self.remote_pointer();
        let dynamic_lights = self.effects.dynamic_lights(&self.game, alpha);
        let mut glowing = lamp_fixtures(&self.level_lights, self.lab.style());
        glowing.extend(glowing_boxes(&self.game));
        glowing.extend(self.effects.glowing_boxes());
        let Some(state) = &mut self.window_state else {
            return;
        };
        let dynamic = self.effects.dynamic_boxes(&self.game, alpha, muzzle);
        let mut world_models = crate::animation::enemies(&self.game, alpha, &self.effects);
        world_models.extend(self.effects.death_models());
        state.renderer.set_models(&world_models, &viewmodels);
        state
            .renderer
            .set_occluders(self.game.closed_doors().chain(self.game.script_blockers()));
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
                remote_pointer,
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
        self.capture_remote_screenshot();
        if save_lighting {
            self.save_lighting();
        }
    }
}
