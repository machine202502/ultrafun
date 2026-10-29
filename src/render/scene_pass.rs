use wgpu::util::DeviceExt;

use super::level_mesh::{Vertex, build_level_vertices};
use super::lightmap::Lightmap;
use super::lights::LightUniforms;
use super::shadows::ShadowMaps;
use super::upscale::{DEPTH_FORMAT, LOW_RES_FORMAT};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SceneUniforms {
    pub view_projection: [[f32; 4]; 4],
    pub camera_position: [f32; 4],
    pub sun_direction: [f32; 4],
    pub fog_color_exposure: [f32; 4],
    pub fog_ambient_linear: [f32; 4],
    pub resolution_snap_dither: [f32; 4],
    pub texel_bands_shadow: [f32; 4],
    pub ambient_sky_resolution: [f32; 4],
    pub ambient_ground_shadows: [f32; 4],
    pub bounce_strength: [f32; 4],
    pub surface: [f32; 4],
}

pub struct ScenePass {
    pub pipeline: wgpu::RenderPipeline,
    pub uniform_buffer: wgpu::Buffer,
    pub light_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    lightmap_sampler: wgpu::Sampler,
}

impl ScenePass {
    pub fn new(device: &wgpu::Device, shadows: &ShadowMaps, lightmap: &wgpu::TextureView) -> Self {
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene uniforms"),
            size: size_of::<SceneUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let light_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lights"),
            size: size_of::<LightUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene uniforms"),
            entries: &[
                uniform_entry(0),
                uniform_entry(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                uniform_entry(3),
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let lightmap_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("lightmap"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind_group = create_bind_group(
            device,
            &bind_group_layout,
            [&uniform_buffer, &light_buffer],
            shadows,
            (lightmap, &lightmap_sampler),
        );
        let pipeline = create_scene_pipeline(device, &bind_group_layout);
        Self {
            pipeline,
            uniform_buffer,
            light_buffer,
            bind_group,
            bind_group_layout,
            lightmap_sampler,
        }
    }

    pub fn rebind(
        &mut self,
        device: &wgpu::Device,
        shadows: &ShadowMaps,
        lightmap: &wgpu::TextureView,
    ) {
        self.bind_group = create_bind_group(
            device,
            &self.bind_group_layout,
            [&self.uniform_buffer, &self.light_buffer],
            shadows,
            (lightmap, &self.lightmap_sampler),
        );
    }
}

fn create_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    [uniform_buffer, light_buffer]: [&wgpu::Buffer; 2],
    shadows: &ShadowMaps,
    (lightmap, lightmap_sampler): (&wgpu::TextureView, &wgpu::Sampler),
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scene uniforms"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: light_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&shadows.array_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: shadows.matrix_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(lightmap),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::Sampler(lightmap_sampler),
            },
        ],
    })
}

fn create_scene_pipeline(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("scene.wgsl"));
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("scene"),
        bind_group_layouts: &[Some(bind_group_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("scene"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(Vertex::layout())],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: LOW_RES_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}

pub fn create_level_buffer(
    device: &wgpu::Device,
    boxes: &[game::LevelBox],
    lightmap: Option<&Lightmap>,
) -> (wgpu::Buffer, u32) {
    create_vertex_buffer(
        device,
        "level vertices",
        &build_level_vertices(boxes, lightmap),
    )
}

pub fn create_vertex_buffer(
    device: &wgpu::Device,
    label: &str,
    vertices: &[Vertex],
) -> (wgpu::Buffer, u32) {
    let (contents, count) = vertex_bytes(vertices);
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents: &contents,
        usage: wgpu::BufferUsages::VERTEX,
    });
    (buffer, count)
}

fn vertex_bytes(vertices: &[Vertex]) -> (Vec<u8>, u32) {
    padded_vertex_bytes(bytemuck::cast_slice(vertices), vertices.len() as u32)
}

fn padded_vertex_bytes(bytes: &[u8], count: u32) -> (Vec<u8>, u32) {
    if bytes.is_empty() {
        (vec![0u8; 64], 0)
    } else {
        (bytes.to_vec(), count)
    }
}

pub fn color_levels(bits: u32) -> f32 {
    ((1u32 << bits.clamp(1, 8)) - 1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_bits_become_levels_per_channel() {
        assert_eq!(color_levels(5), 31.0);
        assert_eq!(color_levels(6), 63.0);
        assert_eq!(color_levels(20), 255.0);
    }
}
#[cfg(test)]
mod buffer_tests {
    use super::*;

    #[test]
    fn empty_vertex_lists_get_a_map_aligned_placeholder() {
        let (contents, count) = padded_vertex_bytes(&[], 0);
        assert_eq!(count, 0);
        assert!(
            contents.len() >= 48,
            "placeholder too small: {}",
            contents.len()
        );
        assert_eq!(contents.len() % 8, 0, "placeholder is not 8-byte aligned");
    }

    #[test]
    fn filled_vertex_lists_keep_their_bytes() {
        let data = [1u8, 2, 3, 4];
        let (contents, count) = padded_vertex_bytes(&data, 1);
        assert_eq!(count, 1);
        assert_eq!(contents, data);
    }
}
