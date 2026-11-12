use std::path::Path;

use anyhow::{Context, Result, ensure};
use glam::{Mat4, Vec2, Vec3};

#[derive(Debug, Clone)]
pub struct ModelVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: Vec2,
}

#[derive(Debug, Clone)]
pub struct Primitive {
    pub vertices: Vec<ModelVertex>,
    pub indices: Vec<u32>,
    pub material: Option<String>,
    pub color: [f32; 3],
    pub emissive: bool,
}

#[derive(Debug, Clone)]
pub struct ModelNode {
    pub name: String,
    pub parent: Option<usize>,
    pub local: Mat4,
    pub primitives: Vec<Primitive>,
}

#[derive(Debug, Clone)]
pub struct Model {
    pub nodes: Vec<ModelNode>,
}

impl Model {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes =
            std::fs::read(path).with_context(|| format!("cannot read model {}", path.display()))?;
        Self::read(&bytes).with_context(|| format!("invalid model {}", path.display()))
    }

    pub fn read(bytes: &[u8]) -> Result<Self> {
        let (document, buffers, _) = gltf::import_slice(bytes)?;
        let scene = document
            .default_scene()
            .or_else(|| document.scenes().next())
            .context("model has no scene")?;
        let mut nodes = Vec::new();
        for node in scene.nodes() {
            read_node(node, None, &buffers, &mut nodes)?;
        }
        ensure!(!nodes.is_empty(), "model has no nodes");
        Ok(Self { nodes })
    }

    pub fn transforms(&self, root: Mat4, joints: &[(String, Mat4)]) -> Vec<Mat4> {
        let mut transforms = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let pose = joints
                .iter()
                .find(|(name, _)| name == &node.name)
                .map_or(Mat4::IDENTITY, |(_, pose)| *pose);
            let parent = node.parent.map_or(root, |parent| transforms[parent]);
            transforms.push(parent * node.local * pose);
        }
        transforms
    }
}

fn read_node(
    node: gltf::Node,
    parent: Option<usize>,
    buffers: &[gltf::buffer::Data],
    nodes: &mut Vec<ModelNode>,
) -> Result<()> {
    let local = Mat4::from_cols_array_2d(&node.transform().matrix());
    ensure!(local.is_finite(), "node transform is not finite");
    ensure!(local.determinant().abs() > 1e-8, "singular node transform");
    let primitives = node
        .mesh()
        .map(|mesh| {
            mesh.primitives()
                .map(|primitive| read_primitive(primitive, buffers))
                .collect::<Result<Vec<_>>>()
        })
        .transpose()?
        .unwrap_or_default();
    let index = nodes.len();
    nodes.push(ModelNode {
        name: node.name().unwrap_or_default().to_owned(),
        parent,
        local,
        primitives,
    });
    for child in node.children() {
        read_node(child, Some(index), buffers, nodes)?;
    }
    Ok(())
}

pub fn read_primitive(
    primitive: gltf::Primitive,
    buffers: &[gltf::buffer::Data],
) -> Result<Primitive> {
    ensure!(
        primitive.mode() == gltf::mesh::Mode::Triangles,
        "only triangle primitives are supported"
    );
    let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|data| &*data.0));
    let positions: Vec<Vec3> = reader
        .read_positions()
        .context("mesh has no positions")?
        .map(Vec3::from)
        .collect();
    let indices: Vec<u32> = reader.read_indices().map_or_else(
        || (0..positions.len() as u32).collect(),
        |indices| indices.into_u32().collect(),
    );
    ensure!(indices.len().is_multiple_of(3), "incomplete triangle");
    ensure!(
        indices
            .iter()
            .all(|index| (*index as usize) < positions.len()),
        "mesh index is out of bounds"
    );
    let normals = reader.read_normals().map_or_else(
        || generated_normals(&positions, &indices),
        |normals| normals.map(Vec3::from).collect(),
    );
    let uvs: Vec<Vec2> = reader.read_tex_coords(0).map_or_else(
        || vec![Vec2::ZERO; positions.len()],
        |uvs| uvs.into_f32().map(Vec2::from).collect(),
    );
    ensure!(
        normals.len() == positions.len() && uvs.len() == positions.len(),
        "vertex attribute counts differ"
    );
    let vertices: Vec<ModelVertex> = positions
        .into_iter()
        .zip(normals)
        .zip(uvs)
        .map(|((position, normal), uv)| ModelVertex {
            position,
            normal: normal.normalize_or_zero(),
            uv,
        })
        .collect();
    ensure!(
        vertices.iter().all(|vertex| vertex.position.is_finite()
            && vertex.normal.is_finite()
            && vertex.uv.is_finite()),
        "mesh contains non-finite vertex data"
    );
    let material = primitive.material();
    let [r, g, b, _] = material.pbr_metallic_roughness().base_color_factor();
    Ok(Primitive {
        vertices,
        indices,
        material: material.name().map(str::to_owned),
        color: [r, g, b].map(|channel| channel.powf(1.0 / 2.2)),
        emissive: material.emissive_factor().iter().any(|value| *value > 0.0),
    })
}

fn generated_normals(positions: &[Vec3], indices: &[u32]) -> Vec<Vec3> {
    let mut normals = vec![Vec3::ZERO; positions.len()];
    for triangle in indices.as_chunks::<3>().0 {
        let [a, b, c] = [
            triangle[0] as usize,
            triangle[1] as usize,
            triangle[2] as usize,
        ];
        let normal = (positions[b] - positions[a]).cross(positions[c] - positions[a]);
        for index in [a, b, c] {
            normals[index] += normal;
        }
    }
    normals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_pose_uses_the_joint_origin_and_parent_transform() {
        let model = Model {
            nodes: vec![
                ModelNode {
                    name: "root".into(),
                    parent: None,
                    local: Mat4::from_translation(Vec3::X * 2.0),
                    primitives: Vec::new(),
                },
                ModelNode {
                    name: "arm".into(),
                    parent: Some(0),
                    local: Mat4::from_translation(Vec3::Y),
                    primitives: Vec::new(),
                },
            ],
        };
        let transforms = model.transforms(
            Mat4::from_translation(Vec3::Z),
            &[(
                "arm".into(),
                Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2),
            )],
        );
        let tip = transforms[1].transform_point3(Vec3::X);
        assert!(tip.distance(Vec3::new(2.0, 2.0, 1.0)) < 1e-5);
    }

    #[test]
    fn shipped_combat_models_have_rigid_parts_uvs_and_weapon_muzzles() {
        for name in ["grunt", "rivet_gun", "sandblaster", "plasma_cutter"] {
            let model = Model::load(
                &Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("assets/models/{name}.glb")),
            )
            .unwrap();
            let geometry: Vec<_> = model
                .nodes
                .iter()
                .flat_map(|node| &node.primitives)
                .collect();
            assert!(!geometry.is_empty());
            assert!(geometry.iter().any(|primitive| {
                primitive
                    .vertices
                    .iter()
                    .any(|vertex| vertex.uv.length_squared() > 0.01)
            }));
            if name == "grunt" {
                assert!(
                    model
                        .nodes
                        .iter()
                        .any(|node| node.name == "left_shin" && node.parent.is_some())
                );
            } else {
                assert!(model.nodes.iter().any(|node| node.name == "muzzle"));
            }
        }
    }

    #[test]
    fn generated_normals_follow_triangle_winding() {
        let normals = generated_normals(&[Vec3::ZERO, Vec3::X, Vec3::Y], &[0, 1, 2]);
        assert!(normals.iter().all(|normal| normal.normalize() == Vec3::Z));
    }
}
