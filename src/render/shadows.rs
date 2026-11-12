use glam::Vec3;
use glam::camera::rh::{proj, view};

use super::level_mesh::Vertex;
use super::lights::{MAX_SHADOWED_LIGHTS, PointLight};
use super::upscale::DEPTH_FORMAT;

pub const SHADOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;
pub const FACES_PER_LIGHT: usize = 6;
pub const MAX_SHADOW_FACES: usize = MAX_SHADOWED_LIGHTS * FACES_PER_LIGHT;
const NEAR_PLANE: f32 = 0.05;
const FACE_FIELD_OF_VIEW_DEGREES: f32 = 100.0;
const FACE_UNIFORM_STRIDE: u64 = 256;
const FACE_DIRECTIONS: [(Vec3, Vec3); FACES_PER_LIGHT] = [
    (Vec3::X, Vec3::Y),
    (Vec3::NEG_X, Vec3::Y),
    (Vec3::Y, Vec3::Z),
    (Vec3::NEG_Y, Vec3::NEG_Z),
    (Vec3::Z, Vec3::Y),
    (Vec3::NEG_Z, Vec3::Y),
];

type Matrix = [[f32; 4]; 4];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FaceUniforms {
    view_projection: Matrix,
    light_position_range: [f32; 4],
}

pub struct ShadowMaps {
    pub resolution: u32,
    pub array_view: wgpu::TextureView,
    pub matrix_buffer: wgpu::Buffer,
    layer_views: Vec<wgpu::TextureView>,
    depth_view: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    face_buffer: wgpu::Buffer,
    face_bind_group: wgpu::BindGroup,
}

impl ShadowMaps {
    pub fn new(device: &wgpu::Device, resolution: u32) -> Self {
        let face_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow face"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(size_of::<FaceUniforms>() as u64),
                },
                count: None,
            }],
        });
        let face_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow faces"),
            size: FACE_UNIFORM_STRIDE * MAX_SHADOW_FACES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let face_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow face"),
            layout: &face_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &face_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(size_of::<FaceUniforms>() as u64),
                }),
            }],
        });
        let matrix_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shadow matrices"),
            size: (size_of::<Matrix>() * MAX_SHADOW_FACES) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (array_view, layer_views, depth_view) = create_targets(device, resolution);
        Self {
            resolution,
            array_view,
            matrix_buffer,
            layer_views,
            depth_view,
            pipeline: create_shadow_pipeline(device, &face_layout),
            face_buffer,
            face_bind_group,
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, resolution: u32) {
        self.resolution = resolution;
        (self.array_view, self.layer_views, self.depth_view) = create_targets(device, resolution);
    }

    pub fn render(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        lights: &[PointLight],
        geometry: &[(&wgpu::Buffer, u32)],
    ) {
        let shadowed = &lights[..lights.len().min(MAX_SHADOWED_LIGHTS)];
        let mut matrices = vec![[[0.0; 4]; 4]; MAX_SHADOW_FACES];
        for (light_index, light) in shadowed.iter().enumerate() {
            for (face_index, matrix) in face_matrices(light).into_iter().enumerate() {
                let layer = light_index * FACES_PER_LIGHT + face_index;
                matrices[layer] = matrix;
                let uniforms = FaceUniforms {
                    view_projection: matrix,
                    light_position_range: light.position.extend(light.range).to_array(),
                };
                queue.write_buffer(
                    &self.face_buffer,
                    layer as u64 * FACE_UNIFORM_STRIDE,
                    bytemuck::bytes_of(&uniforms),
                );
            }
        }
        queue.write_buffer(&self.matrix_buffer, 0, bytemuck::cast_slice(&matrices));

        for layer in 0..shadowed.len() * FACES_PER_LIGHT {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow face"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.layer_views[layer],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
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
            pass.set_pipeline(&self.pipeline);
            let offset = (layer as u64 * FACE_UNIFORM_STRIDE) as u32;
            pass.set_bind_group(0, &self.face_bind_group, &[offset]);
            for (vertices, vertex_count) in geometry {
                if *vertex_count > 0 {
                    pass.set_vertex_buffer(0, vertices.slice(..));
                    pass.draw(0..*vertex_count, 0..1);
                }
            }
        }
    }
}

fn face_matrices(light: &PointLight) -> [Matrix; FACES_PER_LIGHT] {
    let projection = proj::directx::perspective(
        FACE_FIELD_OF_VIEW_DEGREES.to_radians(),
        1.0,
        NEAR_PLANE,
        light.range.max(NEAR_PLANE * 2.0),
    );
    FACE_DIRECTIONS.map(|(direction, up)| {
        (projection * view::look_to_mat4(light.position, direction, up)).to_cols_array_2d()
    })
}

fn create_targets(
    device: &wgpu::Device,
    resolution: u32,
) -> (wgpu::TextureView, Vec<wgpu::TextureView>, wgpu::TextureView) {
    let size = |layers| wgpu::Extent3d {
        width: resolution,
        height: resolution,
        depth_or_array_layers: layers,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("shadow maps"),
        size: size(MAX_SHADOW_FACES as u32),
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SHADOW_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let array_view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..wgpu::TextureViewDescriptor::default()
    });
    let layer_views = (0..MAX_SHADOW_FACES as u32)
        .map(|layer| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2),
                base_array_layer: layer,
                array_layer_count: Some(1),
                ..wgpu::TextureViewDescriptor::default()
            })
        })
        .collect();
    let depth_view = device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow depth"),
            size: size(1),
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default());
    (array_view, layer_views, depth_view)
}

fn create_shadow_pipeline(
    device: &wgpu::Device,
    face_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("shadow.wgsl"));
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("shadow"),
        bind_group_layouts: &[Some(face_layout)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("shadow"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[Some(Vertex::position_layout())],
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
                format: SHADOW_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}
