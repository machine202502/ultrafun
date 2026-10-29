use glam::Vec3;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct PointLight {
    pub position: Vec3,
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RoomLights {
    #[serde(default)]
    pub lights: Vec<PointLight>,
}

pub const MAX_LIGHTS: usize = 64;
pub const MAX_SHADOWED_LIGHTS: usize = 24;
const NO_SHADOW: f32 = -1.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadedLight {
    pub light: PointLight,
    pub shadow_index: Option<u32>,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuLight {
    position_range: [f32; 4],
    color_intensity: [f32; 4],
    shadow: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniforms {
    count: [u32; 4],
    lights: [GpuLight; MAX_LIGHTS],
}

pub fn shaded_level_lights(lights: &[PointLight], shadows: bool) -> Vec<ShadedLight> {
    lights
        .iter()
        .zip(0..)
        .map(|(light, index)| ShadedLight {
            light: *light,
            shadow_index: (shadows && (index as usize) < MAX_SHADOWED_LIGHTS).then_some(index),
        })
        .collect()
}

pub fn unshadowed(lights: &[PointLight]) -> impl Iterator<Item = ShadedLight> + '_ {
    lights.iter().map(|light| ShadedLight {
        light: *light,
        shadow_index: None,
    })
}

pub fn nearest_lights(lights: Vec<ShadedLight>, eye: Vec3) -> Vec<ShadedLight> {
    let mut sorted = lights;
    sorted.sort_by(|a, b| {
        a.light
            .position
            .distance_squared(eye)
            .total_cmp(&b.light.position.distance_squared(eye))
    });
    sorted.truncate(MAX_LIGHTS);
    sorted
}

pub fn light_uniforms(lights: &[ShadedLight], brightness: f32) -> LightUniforms {
    let mut uniforms = LightUniforms {
        count: [lights.len() as u32, 0, 0, 0],
        lights: [GpuLight {
            position_range: [0.0; 4],
            color_intensity: [0.0; 4],
            shadow: [NO_SHADOW, 0.0, 0.0, 0.0],
        }; MAX_LIGHTS],
    };
    for (slot, shaded) in uniforms.lights.iter_mut().zip(lights) {
        let light = shaded.light;
        let [r, g, b] = light.color;
        *slot = GpuLight {
            position_range: light.position.extend(light.range).to_array(),
            color_intensity: [r, g, b, light.intensity * brightness],
            shadow: [
                shaded.shadow_index.map_or(NO_SHADOW, |index| index as f32),
                0.0,
                0.0,
                0.0,
            ],
        };
    }
    uniforms
}

#[cfg(test)]
mod tests {
    use super::*;

    fn light_at(x: f32) -> PointLight {
        PointLight {
            position: Vec3::new(x, 0.0, 0.0),
            color: [1.0; 3],
            intensity: 1.0,
            range: 1.0,
        }
    }

    #[test]
    fn nearest_lights_are_kept_when_over_limit() {
        let lights: Vec<ShadedLight> = unshadowed(
            &(0..MAX_LIGHTS + 10)
                .rev()
                .map(|index| light_at(index as f32))
                .collect::<Vec<_>>(),
        )
        .collect();
        let kept = nearest_lights(lights, Vec3::ZERO);
        assert_eq!(kept.len(), MAX_LIGHTS);
        assert!(
            kept.iter()
                .all(|shaded| shaded.light.position.x < MAX_LIGHTS as f32)
        );
    }

    #[test]
    fn only_first_level_lights_get_shadow_maps() {
        let lights: Vec<PointLight> = (0..MAX_SHADOWED_LIGHTS + 2)
            .map(|index| light_at(index as f32))
            .collect();
        let shaded = shaded_level_lights(&lights, true);
        assert_eq!(shaded[0].shadow_index, Some(0));
        assert_eq!(shaded[MAX_SHADOWED_LIGHTS].shadow_index, None);
        assert!(
            shaded_level_lights(&lights, false)
                .iter()
                .all(|light| light.shadow_index.is_none())
        );
    }
}
