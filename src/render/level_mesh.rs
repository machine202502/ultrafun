use game::LevelBox;
use glam::{EulerRot, Quat, Vec3};

use crate::level_loader::SceneDecor;

use super::atlas::{Atlas, Tile};
use super::lightmap::Lightmap;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub lightmap_uv: [f32; 2],
    pub atlas_uv: [f32; 2],
    pub atlas_tile: [f32; 4],
}

const POSITION_ATTRIBUTE: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];
const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
    0 => Float32x3, 1 => Float32x3, 2 => Float32x4, 3 => Float32x2, 4 => Float32x2, 5 => Float32x4
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

pub fn build_level_vertices(
    boxes: &[LevelBox],
    lightmap: Option<&Lightmap>,
    atlas: &Atlas,
) -> Vec<Vertex> {
    boxes
        .iter()
        .enumerate()
        .flat_map(|(index, level_box)| {
            let tile = atlas.tile(level_box.material.as_deref());
            box_vertices(level_box, tile, LIT, |face, st| {
                lightmap.map_or(NO_LIGHTMAP, |map| {
                    map.uv(index * CUBE_FACES.len() + face, st)
                })
            })
        })
        .collect()
}

pub fn build_box_vertices(boxes: &[LevelBox], emission: f32, atlas: &Atlas) -> Vec<Vertex> {
    boxes
        .iter()
        .flat_map(|level_box| {
            box_vertices(
                level_box,
                atlas.tile(level_box.material.as_deref()),
                emission,
                |_, _| NO_LIGHTMAP,
            )
        })
        .collect()
}

pub fn build_decor_vertices(decor: &[SceneDecor], atlas: &Atlas) -> Vec<Vertex> {
    decor
        .iter()
        .flat_map(|mesh| {
            let normal = glam::Mat3::from_mat4(mesh.transform).inverse().transpose();
            let primitive = &mesh.primitive;
            let tile = atlas.tile(primitive.material.as_deref());
            primitive.indices.iter().map(move |index| {
                let vertex = &primitive.vertices[*index as usize];
                Vertex {
                    position: mesh.transform.transform_point3(vertex.position).to_array(),
                    normal: (normal * vertex.normal).normalize_or_zero().to_array(),
                    color: [
                        primitive.color[0],
                        primitive.color[1],
                        primitive.color[2],
                        f32::from(primitive.emissive),
                    ],
                    lightmap_uv: NO_LIGHTMAP,
                    atlas_uv: vertex.uv.to_array(),
                    atlas_tile: tile.rectangle(),
                }
            })
        })
        .collect()
}

fn box_vertices(
    level_box: &LevelBox,
    tile: Tile,
    emission: f32,
    lightmap_uv: impl Fn(usize, [f32; 2]) -> [f32; 2],
) -> Vec<Vertex> {
    let rotation = box_rotation(level_box);
    let half_size = level_box.size * 0.5;
    let [r, g, b] = level_box.color;
    let mut vertices = Vec::with_capacity(CUBE_FACES.len() * FACE_TRIANGLE_CORNERS.len());
    for (face, (normal, u, v)) in CUBE_FACES.into_iter().enumerate() {
        let world_normal = rotation * normal;
        let (u_axis, v_axis) = face_uv_axes(world_normal);
        for corner in FACE_TRIANGLE_CORNERS {
            let (offset, st) = FACE_CORNERS[corner];
            let local = normal + u * offset.x + v * offset.y;
            let world_position = level_box.center + rotation * (local * half_size);
            vertices.push(Vertex {
                position: world_position.to_array(),
                normal: world_normal.to_array(),
                color: [r, g, b, emission],
                lightmap_uv: lightmap_uv(face, st),
                atlas_uv: [
                    world_position[u_axis] / tile.meters,
                    world_position[v_axis] / tile.meters,
                ],
                atlas_tile: tile.rectangle(),
            });
        }
    }
    vertices
}

fn face_uv_axes(world_normal: Vec3) -> (usize, usize) {
    let absolute = world_normal.abs();
    if absolute.x >= absolute.y && absolute.x >= absolute.z {
        (2, 1)
    } else if absolute.y >= absolute.z {
        (0, 2)
    } else {
        (0, 1)
    }
}
