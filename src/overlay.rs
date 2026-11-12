use glam::Vec3;
use winit::event::WindowEvent;
use winit::window::Window;

use crate::lighting_lab::LightingLab;
use crate::render::Frame;
use crate::ui::{Hud, draw_hud};

#[derive(Default)]
pub struct DebugReport {
    pub frame_seconds: f32,
    pub steps_last_frame: u32,
    pub position: Vec3,
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub horizontal_speed: f32,
    pub vertical_speed: f32,
    pub stamina: f32,
    pub state: String,
    pub grounded: bool,
    pub wall_jumps_used: u32,
    pub enemies_alive: usize,
}

pub struct OverlayContent<'a> {
    pub report: &'a DebugReport,
    pub hud: &'a Hud,
    pub lab: &'a mut LightingLab,
    pub menu: Option<&'a mut crate::menu::Menu>,
    pub pause: Option<&'a mut crate::menu::PauseMenu>,
    pub remote_pointer: Option<(f32, f32)>,
}

pub struct Overlay {
    pub debug_visible: bool,
    context: egui::Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
}

impl Overlay {
    pub fn new(window: &Window, device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let context = egui::Context::default();
        install_fonts(&context);
        let max_texture_side = device.limits().max_texture_dimension_2d as usize;
        let state = egui_winit::State::new(
            context.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            Some(max_texture_side),
        );
        let renderer =
            egui_wgpu::Renderer::new(device, format, egui_wgpu::RendererOptions::default());
        Self {
            debug_visible: false,
            context,
            state,
            renderer,
        }
    }

    pub fn on_window_event(&mut self, window: &Window, event: &WindowEvent, interactive: bool) {
        if self.debug_visible || interactive {
            let _ = self.state.on_window_event(window, event);
        }
    }

    pub fn draw(
        &mut self,
        window: &Window,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        frame: &mut Frame,
        content: OverlayContent,
    ) -> (
        bool,
        Option<crate::menu::MenuAction>,
        Option<crate::menu::PauseAction>,
    ) {
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return (false, None, None);
        }
        let debug_visible = self.debug_visible;
        let OverlayContent {
            report,
            hud,
            lab,
            mut menu,
            mut pause,
            remote_pointer,
        } = content;
        let mut save_lighting = false;
        let mut menu_action = None;
        let mut pause_action = None;
        let input = self.state.take_egui_input(window);
        let menu_visible = menu.as_ref().is_some_and(|menu| menu.visible);
        let mut output = self.context.run_ui(input, |ui| {
            if let Some(menu) = menu.as_deref_mut() {
                menu_action = menu.draw(ui);
            }
            if menu_visible {
                return;
            }
            if let Some(pause) = pause.as_deref_mut() {
                pause_action = pause.draw(ui);
            }
            draw_hud(ui, hud);
            if let Some((x, y)) = remote_pointer {
                draw_remote_pointer(ui, egui::pos2(x, y));
            }
            if lab.visible {
                save_lighting |= lab.ui(ui.ctx());
            }
            if !debug_visible {
                return;
            }
            egui::Window::new("debug").show(ui.ctx(), |ui| {
                let fps = 1.0 / report.frame_seconds.max(f32::EPSILON);
                ui.label(format!(
                    "fps {fps:.0} ({:.2} ms)",
                    report.frame_seconds * 1000.0
                ));
                ui.label(format!(
                    "logic steps last frame {}",
                    report.steps_last_frame
                ));
                let p = report.position;
                ui.label(format!("position {:.2} {:.2} {:.2}", p.x, p.y, p.z));
                ui.label(format!(
                    "yaw {:.1} pitch {:.1}",
                    report.yaw_degrees, report.pitch_degrees
                ));
                ui.separator();
                ui.label(format!(
                    "speed horizontal {:.2} vertical {:.2}",
                    report.horizontal_speed, report.vertical_speed
                ));
                ui.label(format!("stamina {:.2}", report.stamina));
                ui.label(format!(
                    "state {} grounded {}",
                    report.state, report.grounded
                ));
                ui.label(format!("wall jumps used {}", report.wall_jumps_used));
                ui.label(format!("enemies alive {}", report.enemies_alive));
            });
        });
        self.state
            .handle_platform_output(window, output.platform_output);

        let primitives = self
            .context
            .tessellate(output.shapes, output.pixels_per_point);
        for (id, deltas) in output.textures_delta.set.drain() {
            for delta in deltas {
                self.renderer.update_texture(device, queue, id, &delta);
            }
        }
        let size = window.inner_size();
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.width, size.height],
            pixels_per_point: output.pixels_per_point,
        };
        let commands =
            self.renderer
                .update_buffers(device, queue, &mut frame.encoder, &primitives, &screen);
        frame.extra_commands.extend(commands);
        {
            let pass = frame
                .encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("overlay"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &frame.view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
            self.renderer
                .render(&mut pass.forget_lifetime(), &primitives, &screen);
        }
        for id in output.textures_delta.free.drain() {
            self.renderer.free_texture(&id);
        }
        (save_lighting, menu_action, pause_action)
    }
}

pub struct ViewSample {
    pub frame_seconds: f32,
    pub steps_last_frame: u32,
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

impl DebugReport {
    pub fn new(game: &game::Game, view: ViewSample) -> Self {
        let player = &game.player;
        Self {
            frame_seconds: view.frame_seconds,
            steps_last_frame: view.steps_last_frame,
            position: view.position,
            yaw_degrees: view.yaw.to_degrees(),
            pitch_degrees: view.pitch.to_degrees(),
            horizontal_speed: player.horizontal_speed(),
            vertical_speed: player.velocity.y,
            stamina: player.stamina,
            state: movement_state_name(&player.state).to_owned(),
            grounded: player.grounded,
            wall_jumps_used: player.wall_jumps_used,
            enemies_alive: game.enemies.len(),
        }
    }
}

fn install_fonts(context: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let mut loaded = false;
    for (family, name, path) in [
        (
            egui::FontFamily::Proportional,
            "russo_one",
            "assets/fonts/RussoOne-Regular.ttf",
        ),
        (
            egui::FontFamily::Monospace,
            "pt_mono",
            "assets/fonts/PTMono-Regular.ttf",
        ),
    ] {
        match std::fs::read(path) {
            Ok(bytes) => {
                fonts
                    .font_data
                    .insert(name.to_owned(), egui::FontData::from_owned(bytes).into());
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .insert(0, name.to_owned());
                loaded = true;
            }
            Err(error) => log::warn!("font {path} unavailable: {error}"),
        }
    }
    if loaded {
        context.set_fonts(fonts);
    }
}

fn movement_state_name(state: &game::MoveState) -> &'static str {
    match state {
        game::MoveState::Normal => "normal",
        game::MoveState::Dash { .. } => "dash",
        game::MoveState::Slide { .. } => "slide",
        game::MoveState::Slam { .. } => "slam",
    }
}

fn draw_remote_pointer(ui: &mut egui::Ui, position: egui::Pos2) {
    let color = egui::Color32::from_rgb(120, 220, 255);
    let painter = ui.painter();
    painter.circle_filled(position, 5.0, color);
    painter.line_segment(
        [
            position + egui::vec2(-12.0, 0.0),
            position + egui::vec2(-7.0, 0.0),
        ],
        egui::Stroke::new(2.0, color),
    );
    painter.line_segment(
        [
            position + egui::vec2(7.0, 0.0),
            position + egui::vec2(12.0, 0.0),
        ],
        egui::Stroke::new(2.0, color),
    );
    painter.line_segment(
        [
            position + egui::vec2(0.0, -12.0),
            position + egui::vec2(0.0, -7.0),
        ],
        egui::Stroke::new(2.0, color),
    );
    painter.line_segment(
        [
            position + egui::vec2(0.0, 7.0),
            position + egui::vec2(0.0, 12.0),
        ],
        egui::Stroke::new(2.0, color),
    );
}
