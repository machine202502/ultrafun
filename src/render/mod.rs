mod level_mesh;
mod lightmap;
mod lights;
mod scene_pass;
mod shadows;
mod style;
mod upscale;

use std::sync::Arc;

use anyhow::{Context, Result};
use glam::Vec3;
use glam::camera::rh::{proj, view};
use serde::Deserialize;
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

use level_mesh::{GLOWING, LIT, build_box_vertices};
use lightmap::{BakeSettings, bake, empty_texture};
pub use lights::{PointLight, RoomLights};
use lights::{light_uniforms, nearest_lights, shaded_level_lights, unshadowed};
use scene_pass::{
    ScenePass, SceneUniforms, color_levels, create_level_buffer, create_vertex_buffer,
};
use shadows::ShadowMaps;
pub use style::{LightingFile, LightingStyle};
use upscale::{Upscaler, internal_size, read_texture};

const NEAR_PLANE: f32 = 0.05;
const FAR_PLANE: f32 = 1000.0;
const MIN_SHADOW_RESOLUTION: u32 = 32;
const MAX_SHADOW_RESOLUTION: u32 = 256;
const BOUNCE_SAMPLES: u32 = 64;
const MIN_BAKE_TEXEL: f32 = 0.25;

#[derive(Debug, Clone, Deserialize)]
pub struct Atmosphere {
    pub sky_color: [f32; 3],
    pub fog_start: f32,
    pub fog_end: f32,
    pub sun_direction: Vec3,
    #[serde(default = "full_sun_strength")]
    pub sun_strength: f32,
    pub ambient: f32,
}

fn full_sun_strength() -> f32 {
    1.0
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoomAtmosphere {
    pub atmosphere: Atmosphere,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderOptions {
    pub internal_height: Option<u32>,
    pub vertex_snap: bool,
    pub dither: bool,
    pub color_bits: u32,
}

pub struct Camera {
    pub eye: Vec3,
    pub direction: Vec3,
    pub horizontal_fov: f32,
}

pub struct Frame {
    pub encoder: wgpu::CommandEncoder,
    pub view: wgpu::TextureView,
    pub extra_commands: Vec<wgpu::CommandBuffer>,
    surface_texture: wgpu::SurfaceTexture,
}

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    upscaler: Upscaler,
    options: RenderOptions,
    style: LightingStyle,
    scene: ScenePass,
    shadows: ShadowMaps,
    shadows_outdated: bool,
    level_vertices: wgpu::Buffer,
    level_vertex_count: u32,
    level_boxes: Vec<game::LevelBox>,
    level_lights: Vec<PointLight>,
    lightmap: wgpu::TextureView,
    dynamic_vertices: wgpu::Buffer,
    dynamic_vertex_count: u32,
    atmosphere: Atmosphere,
}

pub struct LevelView<'a> {
    pub level: &'a game::Level,
    pub atmosphere: Atmosphere,
    pub lights: Vec<PointLight>,
}

impl Renderer {
    pub fn new(
        window: Arc<Window>,
        display: OwnedDisplayHandle,
        level: LevelView,
        options: RenderOptions,
        style: LightingStyle,
    ) -> Result<Self> {
        let instance = wgpu::Instance::new(
            wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(display)),
        );
        let size = window.inner_size();
        let surface = instance.create_surface(window)?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            ..Default::default()
        }))?;
        log::info!("graphics adapter: {:?}", adapter.get_info());
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))?;

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| !format.is_srgb())
            .context("surface has no non-sRGB format")?;
        let config = wgpu::SurfaceConfiguration {
            format,
            ..surface
                .get_default_config(&adapter, size.width.max(1), size.height.max(1))
                .context("surface is not supported by the adapter")?
        };
        surface.configure(&device, &config);

        let shadows = ShadowMaps::new(&device, shadow_resolution(&style));
        let level_boxes = level.level.boxes.clone();
        let (level_vertices, level_vertex_count, lightmap) =
            build_level(&device, &queue, &level_boxes, &level.lights, &style);
        let scene = ScenePass::new(&device, &shadows, &lightmap);
        let upscaler = Upscaler::new(
            &device,
            format,
            internal_size((config.width, config.height), options.internal_height),
        );
        let (dynamic_vertices, dynamic_vertex_count) =
            create_vertex_buffer(&device, "dynamic vertices", &[]);

        Ok(Self {
            device,
            queue,
            surface,
            config,
            upscaler,
            options,
            style,
            scene,
            shadows,
            shadows_outdated: true,
            level_vertices,
            level_vertex_count,
            level_boxes,
            level_lights: level.lights,
            lightmap,
            dynamic_vertices,
            dynamic_vertex_count,
            atmosphere: level.atmosphere,
        })
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.resize_low_resolution();
    }

    pub fn set_options(&mut self, options: RenderOptions) {
        self.options = options;
        self.resize_low_resolution();
    }

    pub fn set_lighting(&mut self, style: &LightingStyle) {
        if *style == self.style {
            return;
        }
        let rebake = style.bake_key() != self.style.bake_key();
        if style.shadow_key() != self.style.shadow_key() {
            let resolution = shadow_resolution(style);
            if resolution != self.shadows.resolution {
                self.shadows.resize(&self.device, resolution);
                self.scene
                    .rebind(&self.device, &self.shadows, &self.lightmap);
            }
            self.shadows_outdated = true;
        }
        self.style = style.clone();
        if rebake {
            self.rebuild_level();
        }
    }

    fn rebuild_level(&mut self) {
        (self.level_vertices, self.level_vertex_count, self.lightmap) = build_level(
            &self.device,
            &self.queue,
            &self.level_boxes,
            &self.level_lights,
            &self.style,
        );
        self.scene
            .rebind(&self.device, &self.shadows, &self.lightmap);
        self.shadows_outdated = true;
    }

    fn resize_low_resolution(&mut self) {
        let window = (self.config.width, self.config.height);
        self.upscaler.resize(
            &self.device,
            internal_size(window, self.options.internal_height),
        );
    }

    pub fn set_level(&mut self, level: LevelView) {
        self.level_boxes = level.level.boxes.clone();
        self.atmosphere = level.atmosphere;
        self.level_lights = level.lights;
        self.rebuild_level();
    }

    pub fn set_dynamic_boxes(&mut self, lit: &[game::LevelBox], glowing: &[game::LevelBox]) {
        let mut vertices = build_box_vertices(lit, LIT);
        vertices.extend(build_box_vertices(glowing, GLOWING));
        (self.dynamic_vertices, self.dynamic_vertex_count) =
            create_vertex_buffer(&self.device, "dynamic vertices", &vertices);
    }

    pub fn begin_frame(&mut self, camera: &Camera, dynamic_lights: &[PointLight]) -> Option<Frame> {
        let surface_texture = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture)
            | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return None;
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => return None,
        };
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut lights = shaded_level_lights(&self.level_lights, self.style.shadows);
        lights.extend(unshadowed(dynamic_lights));
        self.queue.write_buffer(
            &self.scene.uniform_buffer,
            0,
            bytemuck::bytes_of(&self.scene_uniforms(camera)),
        );
        self.queue.write_buffer(
            &self.scene.light_buffer,
            0,
            bytemuck::bytes_of(&light_uniforms(
                &nearest_lights(lights, camera.eye),
                self.style.lamp_brightness,
            )),
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        if self.shadows_outdated && self.style.shadows {
            self.shadows.render(
                &self.queue,
                &mut encoder,
                &self.level_lights,
                &self.level_vertices,
                self.level_vertex_count,
            );
            self.shadows_outdated = false;
        }
        {
            let [r, g, b] = self.atmosphere.sky_color.map(f64::from);
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.upscaler.color_view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: self.upscaler.depth_view(),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.scene.pipeline);
            pass.set_bind_group(0, &self.scene.bind_group, &[]);
            pass.set_vertex_buffer(0, self.level_vertices.slice(..));
            pass.draw(0..self.level_vertex_count, 0..1);
            if self.dynamic_vertex_count > 0 {
                pass.set_vertex_buffer(0, self.dynamic_vertices.slice(..));
                pass.draw(0..self.dynamic_vertex_count, 0..1);
            }
        }
        self.upscaler.blit(&mut encoder, &view);

        Some(Frame {
            encoder,
            view,
            extra_commands: Vec::new(),
            surface_texture,
        })
    }

    pub fn capture(&self) -> Result<image::RgbaImage> {
        read_texture(&self.device, &self.queue, self.upscaler.color_texture())
    }

    pub fn finish_frame(&self, frame: Frame) {
        self.queue.submit(
            frame
                .extra_commands
                .into_iter()
                .chain([frame.encoder.finish()]),
        );
        self.queue.present(frame.surface_texture);
    }

    fn scene_uniforms(&self, camera: &Camera) -> SceneUniforms {
        let (width, height) = self.upscaler.size();
        let aspect = self.config.width as f32 / self.config.height as f32;
        let vertical_fov = 2.0 * ((camera.horizontal_fov * 0.5).tan() / aspect).atan();
        let projection = proj::directx::perspective(vertical_fov, aspect, NEAR_PLANE, FAR_PLANE);
        let view = view::look_to_mat4(camera.eye, camera.direction, Vec3::Y);
        let atmosphere = &self.atmosphere;
        let style = &self.style;
        let [r, g, b] = atmosphere.sky_color;
        let [sky_r, sky_g, sky_b] = style.ambient_sky;
        let [ground_r, ground_g, ground_b] = style.ambient_ground;
        let flag = |value: bool| f32::from(u8::from(value));
        SceneUniforms {
            view_projection: (projection * view).to_cols_array_2d(),
            camera_position: camera.eye.extend(1.0).to_array(),
            sun_direction: atmosphere
                .sun_direction
                .normalize()
                .extend(atmosphere.sun_strength)
                .to_array(),
            fog_color_exposure: [r, g, b, style.exposure],
            fog_ambient_linear: [
                atmosphere.fog_start * style.fog_distance,
                atmosphere.fog_end * style.fog_distance,
                atmosphere.ambient * style.ambient_brightness,
                flag(style.linear_lighting),
            ],
            resolution_snap_dither: [
                width as f32,
                height as f32,
                flag(self.options.vertex_snap),
                if self.options.dither {
                    color_levels(self.options.color_bits)
                } else {
                    0.0
                },
            ],
            texel_bands_shadow: [
                style.light_texel_size,
                style.light_bands as f32,
                style.shadow_strength,
                flag(style.shadow_soft),
            ],
            ambient_sky_resolution: [sky_r, sky_g, sky_b, self.shadows.resolution as f32],
            ambient_ground_shadows: [ground_r, ground_g, ground_b, flag(style.shadows)],
            bounce_strength: [
                style.gi_strength * style.lamp_brightness,
                flag(style.gi),
                0.0,
                0.0,
            ],
            surface: [
                style.texture_strength,
                style.texture_pixels_per_meter.max(1.0),
                style.face_shading,
                style.saturation,
            ],
        }
    }
}

fn build_level(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    boxes: &[game::LevelBox],
    lights: &[PointLight],
    style: &LightingStyle,
) -> (wgpu::Buffer, u32, wgpu::TextureView) {
    let lightmap = style.gi.then(|| {
        let started = std::time::Instant::now();
        let settings = BakeSettings {
            texel_size: style.gi_texel_size.max(MIN_BAKE_TEXEL),
            samples: BOUNCE_SAMPLES,
            bounces: style.gi_bounces.max(1),
        };
        let lightmap = bake(boxes, lights, settings);
        log::info!(
            "baked {}x{} lightmap in {:.2?}",
            lightmap.width,
            lightmap.height,
            started.elapsed()
        );
        lightmap
    });
    let (buffer, count) = create_level_buffer(device, boxes, lightmap.as_ref());
    let view = lightmap.map_or_else(
        || empty_texture(device, queue),
        |lightmap| lightmap.upload(device, queue),
    );
    (buffer, count, view)
}

fn shadow_resolution(style: &LightingStyle) -> u32 {
    style
        .shadow_resolution
        .clamp(MIN_SHADOW_RESOLUTION, MAX_SHADOW_RESOLUTION)
}
