use std::collections::HashMap;

use anyhow::{Context, Result};
use glam::{Mat4, Quat, Vec3};

pub const LINEAR_TO_DISPLAY_GAMMA: f32 = 1.0 / 2.2;

#[derive(Debug, Clone, Default)]
pub struct SceneObjects {
    pub cubes: HashMap<String, Vec<SceneCube>>,
    pub points: HashMap<String, Vec<ScenePoint>>,
    pub lights: Vec<SceneLight>,
}

#[derive(Debug, Clone)]
pub struct SceneCube {
    pub center: Vec3,
    pub size: Vec3,
    pub rotation: Quat,
    pub color: [f32; 3],
}

#[derive(Debug, Clone)]
pub struct ScenePoint {
    pub position: Vec3,
    pub rotation: Quat,
}

#[derive(Debug, Clone)]
pub struct SceneLight {
    pub position: Vec3,
    pub color: [f32; 3],
    pub candela: f32,
    pub range: Option<f32>,
}

pub fn read_scene(bytes: &[u8]) -> Result<SceneObjects> {
    let gltf = gltf::Gltf::from_slice(bytes)?;
    let scene = gltf
        .default_scene()
        .or_else(|| gltf.scenes().next())
        .context("file has no scene")?;
    let mut objects = SceneObjects::default();
    for node in scene.nodes() {
        visit(&node, Mat4::IDENTITY, &mut objects);
    }
    Ok(objects)
}

fn visit(node: &gltf::Node, parent: Mat4, objects: &mut SceneObjects) {
    let world = parent * Mat4::from_cols_array_2d(&node.transform().matrix());
    let name = base_name(node.name().unwrap_or_default()).to_owned();
    let (scale, rotation, translation) = world.to_scale_rotation_translation();
    match node.mesh() {
        Some(mesh) => {
            let (min, max) = mesh_bounds(&mesh);
            objects.cubes.entry(name).or_default().push(SceneCube {
                center: world.transform_point3((min + max) * 0.5),
                size: (max - min) * scale.abs(),
                rotation,
                color: mesh_color(&mesh),
            });
        }
        None => match node.light() {
            Some(light) if matches!(light.kind(), gltf::khr_lights_punctual::Kind::Point) => {
                objects.lights.push(SceneLight {
                    position: translation,
                    color: light.color(),
                    candela: light.intensity(),
                    range: light.range(),
                })
            }
            _ => objects.points.entry(name).or_default().push(ScenePoint {
                position: translation,
                rotation,
            }),
        },
    }
    for child in node.children() {
        visit(&child, world, objects);
    }
}

fn mesh_bounds(mesh: &gltf::Mesh) -> (Vec3, Vec3) {
    mesh.primitives().fold(
        (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)),
        |(min, max), primitive| {
            let bounds = primitive.bounding_box();
            (
                min.min(Vec3::from(bounds.min)),
                max.max(Vec3::from(bounds.max)),
            )
        },
    )
}

fn mesh_color(mesh: &gltf::Mesh) -> [f32; 3] {
    let [r, g, b, _] = mesh.primitives().next().map_or([1.0; 4], |primitive| {
        primitive
            .material()
            .pbr_metallic_roughness()
            .base_color_factor()
    });
    [r, g, b].map(|channel| channel.powf(LINEAR_TO_DISPLAY_GAMMA))
}

pub fn base_name(name: &str) -> &str {
    name.split('.').next().unwrap_or(name)
}
