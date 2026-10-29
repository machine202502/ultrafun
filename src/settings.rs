use serde::Deserialize;
use winit::keyboard::KeyCode;

use crate::render::RenderOptions;

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub mouse_sensitivity: f32,
    pub horizontal_fov_degrees: f32,
    pub window_width: u32,
    pub window_height: u32,
    pub level: String,
    pub language: String,
    pub master_volume_db: f32,
    pub internal_height: Option<u32>,
    pub vertex_snap: bool,
    pub dither: bool,
    pub color_bits: u32,
    pub keys: KeyBindings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KeyBindings {
    pub forward: KeyCode,
    pub backward: KeyCode,
    pub left: KeyCode,
    pub right: KeyCode,
    pub jump: KeyCode,
    pub dash: KeyCode,
    pub slide: KeyCode,
    pub respawn: KeyCode,
    pub cheats: KeyCode,
}

impl Settings {
    pub fn render_options(&self) -> RenderOptions {
        RenderOptions {
            internal_height: self.internal_height,
            vertex_snap: self.vertex_snap,
            dither: self.dither,
            color_bits: self.color_bits,
        }
    }
}
