mod frame;
mod reload;
mod remote;
mod screenshot;

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
use crate::remote_debug::{REMOTE_DEBUG_PORT, RemoteDebug};
use crate::render::{Atmosphere, LevelView, LightingFile, PointLight, Renderer};
use crate::screenshots::ScreenshotRun;
use crate::settings::Settings;
use crate::text::Texts;

pub(crate) const DATA_DIRECTORY: &str = "assets/data";
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
    textures: DataFiles,
    models: DataFiles,
    level_name: String,
    settings: Settings,
    atmosphere: Atmosphere,
    level_lights: Vec<PointLight>,
    game: game::Game,
    decor: Vec<crate::level_loader::SceneDecor>,
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
    preview_queue: Option<Vec<String>>,
    preview_capturing: Option<String>,
    remote: Option<RemoteDebug>,
    exit_requested: bool,
    menu: crate::menu::Menu,
    pause: crate::menu::PauseMenu,
    pending_menu: Option<crate::menu::MenuAction>,
    pending_pause: Option<crate::menu::PauseAction>,
    cursor_logical: Option<egui::Pos2>,
    window_state: Option<WindowState>,
}

struct WindowState {
    window: Arc<Window>,
    renderer: Renderer,
    overlay: Overlay,
}

impl App {
    pub fn new(
        screenshots: Option<crate::screenshots::ScreenshotArgs>,
        remote_debug: bool,
        make_previews: bool,
    ) -> Result<Self> {
        let data = DataFiles::open(DATA_DIRECTORY)?;
        let levels = DataFiles::open(LEVELS_DIRECTORY)?;
        let settings: Settings = data.load(SETTINGS_FILE)?;
        let shot_list = screenshots
            .as_ref()
            .map(|_| ScreenshotRun::load_shot_list())
            .transpose()?;
        let level_name = shot_list.as_ref().map_or_else(
            || settings.level.clone(),
            |list| {
                list.levels
                    .first()
                    .map_or_else(|| settings.level.clone(), |level| level.level.clone())
            },
        );
        let texts = Texts::load(&settings.language)?;
        let menu = crate::menu::Menu::new(
            &crate::levels::SWITCHABLE_LEVELS,
            &texts,
            screenshots.is_none(),
        );
        let pause = crate::menu::PauseMenu::new(&texts);
        let lighting: LightingFile = data.load(LIGHTING_FILE)?;
        anyhow::ensure!(
            !lighting.presets.is_empty(),
            "{LIGHTING_FILE} has no presets"
        );
        let LoadedLevel {
            level,
            atmosphere,
            lights: level_lights,
            decor,
        } = load_level(&data, &levels, &level_name)?;
        let config = game::GameConfig {
            movement: data.load(MOVEMENT_FILE)?,
            weapon: data.load(WEAPONS_FILE)?,
            health: data.load(HEALTH_FILE)?,
            grunt: data.load(GRUNT_FILE)?,
        };
        let remote = match remote_debug {
            true => match RemoteDebug::start(REMOTE_DEBUG_PORT) {
                Ok(remote) => {
                    log::info!("remote debug listening on 127.0.0.1:{}", remote.port());
                    Some(remote)
                }
                Err(error) => {
                    log::warn!("remote debug not started: {error}");
                    None
                }
            },
            false => None,
        };
        Ok(Self {
            data,
            levels,
            textures: DataFiles::open("assets/textures")?,
            models: DataFiles::open("assets/models")?,
            level_name,
            settings,
            atmosphere,
            level_lights,
            game: game::Game::new(level, config),
            decor,
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
            screenshots: screenshots
                .zip(shot_list)
                .map(|(arguments, list)| {
                    let presets: Vec<usize> = lighting
                        .presets
                        .iter()
                        .enumerate()
                        .filter(|(_, preset)| {
                            arguments
                                .preset
                                .as_ref()
                                .is_none_or(|name| preset.name == *name)
                        })
                        .map(|(index, _)| index)
                        .collect();
                    anyhow::ensure!(
                        !presets.is_empty(),
                        "no lighting preset named {:?}",
                        arguments.preset
                    );
                    let list = list.filtered(arguments.level.as_deref(), arguments.shot.as_deref());
                    anyhow::ensure!(!list.levels.is_empty(), "no shots match the filters");
                    ScreenshotRun::new(arguments.output, list.levels, presets)
                })
                .transpose()?,
            exit_requested: false,
            menu,
            pause,
            pending_menu: None,
            pending_pause: None,
            cursor_logical: None,
            lab: LightingLab::new(lighting),
            preview_queue: make_previews.then(|| {
                crate::levels::SWITCHABLE_LEVELS
                    .iter()
                    .rev()
                    .map(|name| (*name).to_owned())
                    .collect()
            }),
            preview_capturing: None,
            remote,
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
                decor: &self.decor,
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
        if self.menu.visible {
            match key {
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    self.menu.move_selection(-1);
                    return;
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    self.menu.move_selection(1);
                    return;
                }
                KeyCode::Enter | KeyCode::Space => {
                    self.pending_menu = self.menu.activate();
                    return;
                }
                _ => {}
            }
        }
        if self.pause.visible {
            match key {
                KeyCode::ArrowUp | KeyCode::KeyW => {
                    self.pause.move_selection(-1);
                    return;
                }
                KeyCode::ArrowDown | KeyCode::KeyS => {
                    self.pause.move_selection(1);
                    return;
                }
                KeyCode::Enter | KeyCode::Space => {
                    self.pending_pause = self.pause.activate();
                    return;
                }
                _ => {}
            }
        }
        match key {
            KeyCode::Enter if self.game.result.is_some() => self.go_to_next_level(),
            KeyCode::Escape => {
                if self.menu.visible {
                    return;
                }
                self.pause.visible = !self.pause.visible;
                self.set_mouse_captured(!self.pause.visible);
                if self.pause.visible {
                    self.warp_cursor_to_center();
                }
            }
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

    fn warp_cursor_to_center(&mut self) {
        let Some(state) = &self.window_state else {
            return;
        };
        let scale = state.window.scale_factor();
        let size = state.window.inner_size();
        let center = winit::dpi::LogicalPosition::new(
            size.width as f64 / scale / 2.0,
            size.height as f64 / scale / 2.0,
        );
        if let Err(error) = state.window.set_cursor_position(center) {
            log::warn!("cannot warp cursor: {error}");
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
            state.overlay.on_window_event(
                &state.window,
                &event,
                self.lab.visible || self.menu.visible,
            );
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
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self
                    .window_state
                    .as_ref()
                    .map_or(1.0, |state| state.window.scale_factor() as f32);
                self.cursor_logical = Some(egui::pos2(
                    position.x as f32 / scale,
                    position.y as f32 / scale,
                ));
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } if state == ElementState::Pressed
                && !self.mouse_captured
                && (self.menu.visible || self.pause.visible) =>
            {
                if let (Some(cursor), Some(window_state)) =
                    (self.cursor_logical, &self.window_state)
                {
                    let scale = window_state.window.scale_factor() as f32;
                    let size = window_state.window.inner_size();
                    let screen = egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(size.width as f32 / scale, size.height as f32 / scale),
                    );
                    if self.menu.visible {
                        if let Some(action) = self.menu.hit_test(cursor, screen) {
                            self.pending_menu = Some(action);
                        }
                    } else if let Some(action) = self.pause.hit_test(cursor, screen) {
                        self.pending_pause = Some(action);
                    }
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match (state, self.mouse_captured) {
                (ElementState::Pressed, false)
                    if self.lab.visible || self.menu.visible || self.pause.visible => {}
                (ElementState::Pressed, false) => self.set_mouse_captured(true),
                (ElementState::Pressed, true) => self.input.set_fire_held(true),
                (ElementState::Released, _) => self.input.set_fire_held(false),
            },
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Right,
                ..
            } => {
                if self.mouse_captured || state == ElementState::Released {
                    self.input.set_alt_held(state == ElementState::Pressed);
                }
            }
            WindowEvent::MouseWheel { delta, .. } if self.mouse_captured => {
                let delta = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => f64::from(y),
                    winit::event::MouseScrollDelta::PixelDelta(position) => position.y,
                };
                if delta.abs() > 0.0 {
                    self.input.cycle_weapon(if delta > 0.0 { -1 } else { 1 });
                }
            }
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
