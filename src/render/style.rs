use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightingStyle {
    pub linear_lighting: bool,
    pub exposure: f32,
    pub lamp_brightness: f32,
    pub ambient_sky: [f32; 3],
    pub ambient_ground: [f32; 3],
    pub ambient_brightness: f32,
    pub light_texel_size: f32,
    pub light_bands: u32,
    pub shadows: bool,
    pub shadow_resolution: u32,
    pub shadow_soft: bool,
    pub shadow_strength: f32,
    pub gi: bool,
    pub gi_strength: f32,
    pub gi_texel_size: f32,
    pub gi_bounces: u32,
    pub texture_strength: f32,
    pub texture_pixels_per_meter: f32,
    pub face_shading: f32,
    pub saturation: f32,
    pub fog_distance: f32,
    pub lamp_fixtures: bool,
    pub fixture_size: f32,
}

impl LightingStyle {
    pub fn shadow_key(&self) -> Option<u32> {
        self.shadows.then_some(self.shadow_resolution)
    }

    pub fn bake_key(&self) -> Option<(u32, u32)> {
        self.gi
            .then_some((self.gi_texel_size.to_bits(), self.gi_bounces))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightingPreset {
    pub name: String,
    pub style: LightingStyle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightingFile {
    pub active: String,
    pub presets: Vec<LightingPreset>,
}

impl LightingFile {
    pub fn active_index(&self) -> usize {
        self.presets
            .iter()
            .position(|preset| preset.name == self.active)
            .unwrap_or(0)
    }
}
