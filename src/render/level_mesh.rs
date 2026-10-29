use game::LevelBox;
use glam::{EulerRot, Quat, Vec3};

use super::lightmap::Lightmap;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    color: [f32; 4],
    lightmap_uv: [f32; 2],
}

const POSITION_ATTRIBUTE: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
    0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2
];

impl Vertex {
    pub fn position_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &POSITION_ATTRIBUTE,
        }
    }

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

pub const CUBE_FACES: [(Vec3, Vec3, Vec3); 6] = [
    (Vec3::X, Vec3::Y, Vec3::Z),
    (Vec3::NEG_X, Vec3::Y, Vec3::Z),
    (Vec3::Y, Vec3::X, Vec3::Z),
    (Vec3::NEG_Y, Vec3::X, Vec3::Z),
    (Vec3::Z, Vec3::X, Vec3::Y),
    (Vec3::NEG_Z, Vec3::X, Vec3::Y),
];

const FACE_CORNERS: [(Vec3, [f32; 2]); 4] = [
    (Vec3::new(-1.0, -1.0, 0.0), [0.0, 0.0]),
    (Vec3::new(1.0, -1.0, 0.0), [1.0, 0.0]),
    (Vec3::new(1.0, 1.0, 0.0), [1.0, 1.0]),
    (Vec3::new(-1.0, 1.0, 0.0), [0.0, 1.0]),
];
const FACE_TRIANGLE_CORNERS: [usize; 6] = [0, 1, 2, 0, 2, 3];
const NO_LIGHTMAP: [f32; 2] = [-1.0, -1.0];

pub const LIT: f32 = 0.0;
pub const GLOWING: f32 = 1.0;

pub fn box_rotation(level_box: &LevelBox) -> Quat {
    let angles = level_box.rotation_degrees;
    Quat::from_euler(
        EulerRot::XYZ,
        angles.x.to_radians(),
        angles.y.to_radians(),
        angles.z.to_radians(),
    )
}

pub fn build_level_vertices(boxes: &[LevelBox], lightmap: Option<&Lightmap>) -> Vec<Vertex> {
    boxes
        .iter()
        .enumerate()
        .flat_map(|(index, level_box)| {
            box_vertices(level_box, LIT, |face, st| {
                lightmap.map_or(NO_LIGHTMAP, |map| {
                    map.uv(index * CUBE_FACES.len() + face, st)
                })
            })
        })
        .collect()
}

pub fn build_box_vertices(boxes: &[LevelBox], emission: f32) -> Vec<Vertex> {
    boxes
        .iter()
        .flat_map(|level_box| box_vertices(level_box, emission, |_, _| NO_LIGHTMAP))
        .collect()
}

fn box_vertices(
    level_box: &LevelBox,
    emission: f32,
    lightmap_uv: impl Fn(usize, [f32; 2]) -> [f32; 2],
) -> Vec<Vertex> {
    let rotation = box_rotation(level_box);
    let half_size = level_box.size * 0.5;
    let [r, g, b] = level_box.color;
    let mut vertices = Vec::with_capacity(CUBE_FACES.len() * FACE_TRIANGLE_CORNERS.len());
    for (face, (normal, u, v)) in CUBE_FACES.into_iter().enumerate() {
        let world_normal = rotation * normal;
        for corner in FACE_TRIANGLE_CORNERS {
            let (offset, st) = FACE_CORNERS[corner];
            let local = normal + u * offset.x + v * offset.y;
            vertices.push(Vertex {
                position: (level_box.center + rotation * (local * half_size)).to_array(),
                normal: world_normal.to_array(),
                color: [r, g, b, emission],
                lightmap_uv: lightmap_uv(face, st),
            });
        }
    }
    vertices
}
