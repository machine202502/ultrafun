use std::ops::Range;
use std::path::Path;

use anyhow::Result;
use glam::{Mat3, Mat4};
use wgpu::util::DeviceExt;

use crate::model::Model;

use super::atlas::Atlas;
use super::level_mesh::Vertex;

pub const MODEL_NAMES: [&str; 4] = ["grunt", "rivet_gun", "sandblaster", "plasma_cutter"];

pub struct ModelInstance {
    pub model: usize,
    pub transform: Mat4,
    pub joints: Vec<(String, Mat4)>,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    transform: [[f32; 4]; 4],
    normal: [[f32; 4]; 3],
}

const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
    6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4,
    10 => Float32x4, 11 => Float32x4, 12 => Float32x4
];

impl Instance {
    pub fn new(transform: Mat4) -> Self {
        let normal = Mat3::from_mat4(transform).inverse().transpose();
        Self {
            transform: transform.to_cols_array_2d(),
            normal: [normal.x_axis, normal.y_axis, normal.z_axis]
                .map(|column| column.extend(0.0).to_array()),
        }
    }

    pub fn identity_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("identity transform"),
            contents: bytemuck::bytes_of(&Self::new(Mat4::IDENTITY)),
            usage: wgpu::BufferUsages::VERTEX,
        })
    }

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &INSTANCE_ATTRIBUTES,
        }
    }
}

struct Part {
    node: usize,
    indices: Range<u32>,
}

struct GpuModel {
    source: Model,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    parts: Vec<Part>,
}

struct Draw {
    model: usize,
    part: usize,
    instance: u32,
}

pub struct ModelPass {
    models: Vec<GpuModel>,
    instances: wgpu::Buffer,
    capacity: u64,
    world: Vec<Draw>,
    viewmodel: Vec<Draw>,
}

impl ModelPass {
    pub fn load(device: &wgpu::Device, atlas: &Atlas) -> Result<Self> {
        let models = MODEL_NAMES
            .iter()
            .map(|name| {
                let path = Path::new("assets/models").join(format!("{name}.glb"));
                Model::load(&path).map(|model| upload_model(device, model, atlas))
            })
            .collect::<Result<Vec<_>>>()?;
        let instances = Instance::identity_buffer(device);
        Ok(Self {
            models,
            instances,
            capacity: 0,
            world: Vec::new(),
            viewmodel: Vec::new(),
        })
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        world: &[ModelInstance],
        viewmodel: &[ModelInstance],
    ) {
        self.world.clear();
        self.viewmodel.clear();
        let mut instances = Vec::new();
        for (objects, draws) in [(world, &mut self.world), (viewmodel, &mut self.viewmodel)] {
            for object in objects {
                let Some(model) = self.models.get(object.model) else {
                    continue;
                };
                let first = instances.len() as u32;
                instances.extend(
                    model
                        .source
                        .transforms(object.transform, &object.joints)
                        .into_iter()
                        .map(Instance::new),
                );
                draws.extend(model.parts.iter().enumerate().map(|(part, mesh)| Draw {
                    model: object.model,
                    part,
                    instance: first + mesh.node as u32,
                }));
            }
        }
        let bytes = bytemuck::cast_slice(&instances);
        if bytes.is_empty() {
            return;
        }
        if bytes.len() as u64 > self.capacity {
            self.capacity = (bytes.len() as u64).next_power_of_two();
            self.instances = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("model transforms"),
                size: self.capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.instances, 0, bytes);
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, viewmodel: bool) {
        pass.set_vertex_buffer(1, self.instances.slice(..));
        let draws = if viewmodel {
            &self.viewmodel
        } else {
            &self.world
        };
        for draw in draws {
            let model = &self.models[draw.model];
            pass.set_vertex_buffer(0, model.vertices.slice(..));
            pass.set_index_buffer(model.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(
                model.parts[draw.part].indices.clone(),
                0,
                draw.instance..draw.instance + 1,
            );
        }
    }

    pub fn muzzle(&self, instance: &ModelInstance) -> Option<glam::Vec3> {
        let model = &self.models.get(instance.model)?.source;
        let index = model.nodes.iter().position(|node| node.name == "muzzle")?;
        Some(
            model.transforms(instance.transform, &instance.joints)[index]
                .transform_point3(glam::Vec3::ZERO),
        )
    }

    pub fn has_viewmodel(&self) -> bool {
        !self.viewmodel.is_empty()
    }
}

fn upload_model(device: &wgpu::Device, source: Model, atlas: &Atlas) -> GpuModel {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut parts = Vec::new();
    for (node, part) in source.nodes.iter().enumerate() {
        for primitive in &part.primitives {
            let tile = atlas.tile(primitive.material.as_deref());
            let offset = vertices.len() as u32;
            let start = indices.len() as u32;
            vertices.extend(primitive.vertices.iter().map(|vertex| Vertex {
                position: vertex.position.to_array(),
                normal: vertex.normal.to_array(),
                color: [
                    primitive.color[0],
                    primitive.color[1],
                    primitive.color[2],
                    f32::from(primitive.emissive),
                ],
                lightmap_uv: [-1.0; 2],
                atlas_uv: vertex.uv.to_array(),
                atlas_tile: tile.rectangle(),
            }));
            indices.extend(primitive.indices.iter().map(|index| index + offset));
            parts.push(Part {
                node,
                indices: start..indices.len() as u32,
            });
        }
    }
    let upload = |label, contents, usage| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents,
            usage,
        })
    };
    GpuModel {
        source,
        vertices: upload(
            "model vertices",
            bytemuck::cast_slice(&vertices),
            wgpu::BufferUsages::VERTEX,
        ),
        indices: upload(
            "model indices",
            bytemuck::cast_slice(&indices),
            wgpu::BufferUsages::INDEX,
        ),
        parts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn normal_transform_preserves_perpendicularity_under_nonuniform_scale() {
        let transform = Mat4::from_rotation_y(0.7) * Mat4::from_scale(Vec3::new(2.0, 3.0, 0.5));
        let instance = Instance::new(transform);
        let normal = Mat3::from_cols(
            Vec3::from_slice(&instance.normal[0]),
            Vec3::from_slice(&instance.normal[1]),
            Vec3::from_slice(&instance.normal[2]),
        ) * Vec3::new(1.0, 1.0, 0.0).normalize();
        let tangent = transform.transform_vector3(Vec3::new(1.0, -1.0, 0.0));
        assert!(normal.dot(tangent).abs() < 1e-5);
    }
}
