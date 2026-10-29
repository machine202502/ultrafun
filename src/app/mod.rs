mod frame;
mod reload;
mod screenshot;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Fullscreen, Window, WindowId};

use crate::assets::DataFiles;
use crate::audio::Audio;
use crate::dispatcher::Dispatcher;
use crate::fx::Effects;
use crate::input::Input;
use crate::levels::{LEVELS_DIRECTORY, LoadedLevel, load_level};
use crate::lighting_lab::LightingLab;
use crate::overlay::{DebugReport, Overlay};
use crate::render::{Atmosphere, LevelView, LightingFile, PointLight, Renderer};
use crate::screenshots::ScreenshotRun;
use crate::settings::Settings;
use crate::text::Texts;

const DATA_DIRECTORY: &str = "assets/data";
const SETTINGS_FILE: &str = "settings_default.ron";
const MOVEMENT_FILE: &str = "movement.ron";
const WEAPONS_FILE: &str = "weapons.ron";
const HEALTH_FILE: &str = "health.ron";
const GRUNT_FILE: &str = "grunt.ron";
const LIGHTING_FILE: &str = "lighting.ron";
const WINDOW_TITLE: &str = "1681";
const MAX_STEPS_PER_FRAME: u32 = 8;

pub struct App {
    data: DataFiles,
    levels: DataFiles,
    level_name: String,
    settings: Settings,
    atmosphere: Atmosphere,
    level_lights: Vec<PointLight>,
    game: game::Game,
    input: Input,
    mouse_captured: bool,
    last_frame: Instant,
    unsimulated_seconds: f32,
    report: DebugReport,
    effects: Effects,
    dispatcher: Dispatcher,
    audio: Option<Audio>,
    texts: Texts,
    lab: LightingLab,
    screenshots: Option<ScreenshotRun>,
    exit_requested: bool,
    window_state: Option<WindowState>,
}

struct WindowState {
    window: Arc<Window>,
    renderer: Renderer,
    overlay: Overlay,
}

impl App {
    pub fn new(screenshot_output: Option<PathBuf>) -> Result<Self> {
        let data = DataFiles::open(DATA_DIRECTORY)?;
        let levels = DataFiles::open(LEVELS_DIRECTORY)?;
        let settings: Settings = data.load(SETTINGS_FILE)?;
        let shot_list = screenshot_output
            .as_ref()
            .map(|_| ScreenshotRun::load_shot_list())
            .transpose()?;
        let level_name = shot_list
            .as_ref()
            .map_or_else(|| settings.level.clone(), |list| list.level.clone());
        let texts = Texts::load(&settings.language)?;
        let lighting: LightingFile = data.load(LIGHTING_FILE)?;
        anyhow::ensure!(
            !lighting.presets.is_empty(),
            "{LIGHTING_FILE} has no presets"
        );
        let LoadedLevel {
            level,
            atmosphere,
            lights: level_lights,
        } = load_level(&data, &levels, &level_name)?;
        let config = game::GameConfig {
            movement: data.load(MOVEMENT_FILE)?,
            weapon: data.load(WEAPONS_FILE)?,
            health: data.load(HEALTH_FILE)?,
            grunt: data.load(GRUNT_FILE)?,
        };
        Ok(Self {
            data,
            levels,
            level_name,
            settings,
            atmosphere,
            level_lights,
            game: game::Game::new(level, config),
            input: Input::default(),
            mouse_captured: false,
            last_frame: Instant::now(),
            unsimulated_seconds: 0.0,
            report: DebugReport::default(),
            effects: Effects::default(),
            dispatcher: Dispatcher::default(),
            audio: Audio::new()
                .inspect_err(|error| log::warn!("playing without sound: {error:#}"))
                .ok(),
            texts,
            screenshots: screenshot_output
                .zip(shot_list)
                .map(|(output, list)| {
                    ScreenshotRun::new(output, list.shots, lighting.presets.len())
                })
                .transpose()?,
            exit_requested: false,
            lab: LightingLab::new(lighting),
            window_state: None,
        })
    }

    fn create_window_state(&self, event_loop: &ActiveEventLoop) -> Result<WindowState> {
        let attributes = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(
                self.settings.window_width,
                self.settings.window_height,
            ));
        let window = Arc::new(event_loop.create_window(attributes)?);
        let renderer = Renderer::new(
            window.clone(),
            event_loop.owned_display_handle(),
            LevelView {
                level: self.game.level(),
                atmosphere: self.atmosphere.clone(),
                lights: self.level_lights.clone(),
            },
            self.settings.render_options(),
            self.lab.style().clone(),
        )?;
        let overlay = Overlay::new(&window, &renderer.device, renderer.format());
        Ok(WindowState {
            window,
            renderer,
            overlay,
        })
    }

    fn key_pressed(&mut self, key: KeyCode) {
        match key {
            KeyCode::Escape => self.set_mouse_captured(false),
            KeyCode::F1 => {
                if let Some(state) = &mut self.window_state {
                    state.overlay.debug_visible = !state.overlay.debug_visible;
                }
            }
            KeyCode::F2 => self.switch_level(),
            KeyCode::F3 => {
                self.lab.visible = !self.lab.visible;
                if self.lab.visible {
                    self.set_mouse_captured(false);
                }
            }
            KeyCode::F11 => {
                if let Some(state) = &self.window_state {
                    let fullscreen = match state.window.fullscreen() {
                        Some(_) => None,
                        None => Some(Fullscreen::Borderless(None)),
                    };
                    state.window.set_fullscreen(fullscreen);
                }
            }
            _ => self.input.key_pressed(key, &self.settings.keys),
        }
    }

    fn set_mouse_captured(&mut self, captured: bool) {
        let Some(state) = &self.window_state else {
            return;
        };
        let window = &state.window;
        let grab = if captured {
            window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined))
        } else {
            window.set_cursor_grab(CursorGrabMode::None)
        };
        if let Err(error) = grab {
            log::warn!("cannot change mouse capture: {error}");
        }
        window.set_cursor_visible(!captured);
        self.mouse_captured = captured;
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window_state.is_some() {
            return;
        }
        match self.create_window_state(event_loop) {
            Ok(state) => self.window_state = Some(state),
            Err(error) => {
                log::error!("{error:#}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if let Some(state) = &mut self.window_state {
            state
                .overlay
                .on_window_event(&state.window, &event, self.lab.visible);
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(state) = &mut self.window_state {
                    state.renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::Focused(false) => {
                self.input.release_all();
                self.set_mouse_captured(false);
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match (state, self.mouse_captured) {
                (ElementState::Pressed, false) if self.lab.visible => {}
                (ElementState::Pressed, false) => self.set_mouse_captured(true),
                (ElementState::Pressed, true) => self.input.set_fire_held(true),
                (ElementState::Released, _) => self.input.set_fire_held(false),
            },
            WindowEvent::KeyboardInput { event, .. } if !event.repeat => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => self.key_pressed(key),
                        ElementState::Released => self.input.key_released(key),
                    }
                }
            }
            WindowEvent::RedrawRequested => self.frame(),
            _ => {}
        }
    }

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta: (dx, dy) } = event
            && self.mouse_captured
        {
            self.input.add_mouse_motion(dx, dy);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.exit_requested {
            event_loop.exit();
            return;
        }
        if let Some(state) = &self.window_state {
            state.window.request_redraw();
        }
    }
}
