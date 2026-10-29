use game::LevelBox;
use glam::{Quat, Vec3};

use super::level_mesh::{CUBE_FACES, box_rotation};
use super::lights::PointLight;

const PADDING: u32 = 1;
const MAX_FACE_TEXELS: u32 = 128;
const ATLAS_WIDTH: u32 = 2048;
const RAY_OFFSET: f32 = 0.02;
const SHADOW_RAY_SHORTEN: f32 = 0.05;
const GAMMA: f32 = 2.2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BakeSettings {
    pub texel_size: f32,
    pub samples: u32,
    pub bounces: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub struct Lightmap {
    pub width: u32,
    pub height: u32,
    pub indirect: Vec<Vec3>,
    pub faces: Vec<FaceRect>,
}

impl Lightmap {
    pub fn uv(&self, face: usize, [s, t]: [f32; 2]) -> [f32; 2] {
        let rect = self.faces[face];
        [
            ((rect.x + PADDING) as f32 + s * rect.width as f32) / self.width as f32,
            ((rect.y + PADDING) as f32 + t * rect.height as f32) / self.height as f32,
        ]
    }

    pub fn upload(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::TextureView {
        let texels: Vec<u16> = self
            .indirect
            .iter()
            .flat_map(|value| [value.x, value.y, value.z, 1.0].map(f16_bits))
            .collect();
        upload_texels(device, queue, self.width, self.height, &texels)
    }
}

pub fn empty_texture(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::TextureView {
    upload_texels(device, queue, 1, 1, &[0; 4])
}

fn upload_texels(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    texels: &[u16],
) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("lightmap"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        texture.as_image_copy(),
        bytemuck::cast_slice(texels),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 8),
            rows_per_image: None,
        },
        size,
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn f16_bits(value: f32) -> u16 {
    let bits = value.max(0.0).to_bits();
    let exponent = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    if exponent <= 0 {
        return 0;
    }
    if exponent >= 31 {
        return 0x7bff;
    }
    (((exponent as u32) << 10) | ((bits & 0x7f_ffff) >> 13)) as u16
}

struct Face {
    center: Vec3,
    normal: Vec3,
    u_axis: Vec3,
    v_axis: Vec3,
    half_u: f32,
    half_v: f32,
    albedo: Vec3,
}

impl Face {
    fn point(&self, s: f32, t: f32) -> Vec3 {
        self.center
            + self.u_axis * self.half_u * (2.0 * s - 1.0)
            + self.v_axis * self.half_v * (2.0 * t - 1.0)
    }
}

struct OrientedBox {
    center: Vec3,
    inverse_rotation: Quat,
    half_size: Vec3,
}

struct Hit {
    distance: f32,
    face: usize,
    s: f32,
    t: f32,
}

struct Scene<'a> {
    faces: &'a [Face],
    obbs: &'a [OrientedBox],
    rects: &'a [FaceRect],
    width: u32,
}

impl Scene<'_> {
    fn texel_index(&self, face: usize, s: f32, t: f32) -> usize {
        let rect = self.rects[face];
        let x = ((s * rect.width as f32) as u32).min(rect.width - 1) + rect.x + PADDING;
        let y = ((t * rect.height as f32) as u32).min(rect.height - 1) + rect.y + PADDING;
        (y * self.width + x) as usize
    }

    fn cast(&self, origin: Vec3, direction: Vec3, max_distance: f32) -> Option<Hit> {
        self.obbs
            .iter()
            .enumerate()
            .filter_map(|(index, obb)| intersect(obb, origin, direction, max_distance, index))
            .min_by(|a, b| a.distance.total_cmp(&b.distance))
    }
}

pub fn bake(boxes: &[LevelBox], lights: &[PointLight], settings: BakeSettings) -> Lightmap {
    bake_with_direct(boxes, lights, settings).0
}

fn bake_with_direct(
    boxes: &[LevelBox],
    lights: &[PointLight],
    settings: BakeSettings,
) -> (Lightmap, Vec<Vec3>) {
    let faces = build_faces(boxes);
    let obbs: Vec<OrientedBox> = boxes
        .iter()
        .map(|level_box| OrientedBox {
            center: level_box.center,
            inverse_rotation: box_rotation(level_box).inverse(),
            half_size: level_box.size * 0.5,
        })
        .collect();
    let (rects, width, height) = pack(&faces, settings.texel_size);
    let texel_count = (width * height) as usize;
    let lights: Vec<PointLight> = lights
        .iter()
        .map(|light| PointLight {
            color: light.color.map(|channel| channel.powf(GAMMA)),
            ..*light
        })
        .collect();
    let scene = Scene {
        faces: &faces,
        obbs: &obbs,
        rects: &rects,
        width,
    };

    let mut direct = vec![Vec3::ZERO; texel_count];
    for_each_texel(&scene, &mut direct, |face, point| {
        direct_light(&scene, &lights, point, face.normal)
    });
    let mut indirect = vec![Vec3::ZERO; texel_count];
    for _ in 0..settings.bounces {
        let previous = indirect.clone();
        for_each_texel(&scene, &mut indirect, |face, point| {
            gather(&scene, &direct, &previous, face, point, settings.samples)
        });
    }

    let lightmap = Lightmap {
        width,
        height,
        indirect,
        faces: rects,
    };
    (lightmap, direct)
}

fn for_each_texel(scene: &Scene, target: &mut [Vec3], shade: impl Fn(&Face, Vec3) -> Vec3 + Sync) {
    let jobs: Vec<(usize, u32, u32)> = scene
        .rects
        .iter()
        .enumerate()
        .flat_map(|(face, rect)| {
            (0..rect.height).flat_map(move |y| (0..rect.width).map(move |x| (face, x, y)))
        })
        .collect();
    let threads = std::thread::available_parallelism().map_or(4, usize::from);
    let chunk_size = jobs.len().div_ceil(threads * 8).max(1);
    let shaded: Vec<Vec<(usize, Vec3)>> = std::thread::scope(|scope| {
        let handles: Vec<_> = jobs
            .chunks(chunk_size)
            .map(|chunk| {
                let shade = &shade;
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|&(face_index, x, y)| {
                            let rect = scene.rects[face_index];
                            let s = (x as f32 + 0.5) / rect.width as f32;
                            let t = (y as f32 + 0.5) / rect.height as f32;
                            let face = &scene.faces[face_index];
                            let index = scene.texel_index(face_index, s, t);
                            (index, shade(face, face.point(s, t)))
                        })
                        .collect()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("bake thread panicked"))
            .collect()
    });
    for (index, value) in shaded.into_iter().flatten() {
        target[index] = value;
    }
    fill_padding(scene, target);
}

fn fill_padding(scene: &Scene, target: &mut [Vec3]) {
    for rect in scene.rects {
        let (width, height) = (rect.width as i64, rect.height as i64);
        let atlas_index = |x: i64, y: i64| {
            ((rect.y + PADDING) as i64 + y) as usize * scene.width as usize
                + ((rect.x + PADDING) as i64 + x) as usize
        };
        for y in -1..=height {
            for x in -1..=width {
                if (0..width).contains(&x) && (0..height).contains(&y) {
                    continue;
                }
                target[atlas_index(x, y)] =
                    target[atlas_index(x.clamp(0, width - 1), y.clamp(0, height - 1))];
            }
        }
    }
}

fn direct_light(scene: &Scene, lights: &[PointLight], point: Vec3, normal: Vec3) -> Vec3 {
    let origin = point + normal * RAY_OFFSET;
    lights
        .iter()
        .filter_map(|light| {
            let to_light = light.position - origin;
            let distance = to_light.length();
            if distance >= light.range || distance <= f32::EPSILON {
                return None;
            }
            let direction = to_light / distance;
            let facing = normal.dot(direction);
            if facing <= 0.0 {
                return None;
            }
            if scene
                .cast(origin, direction, distance - SHADOW_RAY_SHORTEN)
                .is_some()
            {
                return None;
            }
            let edge = (1.0 - (distance / light.range).powi(4)).clamp(0.0, 1.0);
            let falloff = edge * edge / (distance * distance + 1.0);
            Some(Vec3::from(light.color) * light.intensity * facing * falloff)
        })
        .sum()
}

fn gather(
    scene: &Scene,
    direct: &[Vec3],
    indirect: &[Vec3],
    face: &Face,
    point: Vec3,
    samples: u32,
) -> Vec3 {
    let origin = point + face.normal * RAY_OFFSET;
    let rotation = hash(point) * std::f32::consts::TAU;
    let samples = samples.max(1);
    let total: Vec3 = (0..samples)
        .map(|index| {
            let direction = cosine_direction(index, samples, rotation, face);
            match scene.cast(origin, direction, f32::MAX) {
                Some(hit) => {
                    let texel = scene.texel_index(hit.face, hit.s, hit.t);
                    scene.faces[hit.face].albedo * (direct[texel] + indirect[texel])
                }
                None => Vec3::ZERO,
            }
        })
        .sum();
    total / samples as f32
}

fn cosine_direction(index: u32, count: u32, rotation: f32, face: &Face) -> Vec3 {
    let u = (index as f32 + 0.5) / count as f32;
    let radius = u.sqrt();
    let angle = radical_inverse(index) * std::f32::consts::TAU + rotation;
    let z = (1.0 - u).max(0.0).sqrt();
    (face.u_axis * radius * angle.cos() + face.v_axis * radius * angle.sin() + face.normal * z)
        .normalize()
}

fn radical_inverse(bits: u32) -> f32 {
    bits.reverse_bits() as f32 / 4_294_967_296.0
}

fn hash(point: Vec3) -> f32 {
    let value = point.dot(Vec3::new(12.9898, 78.233, 37.719)).sin() * 43_758.547;
    value - value.floor()
}

fn build_faces(boxes: &[LevelBox]) -> Vec<Face> {
    boxes
        .iter()
        .flat_map(|level_box| {
            let rotation = box_rotation(level_box);
            let half = level_box.size * 0.5;
            let albedo = Vec3::from(level_box.color).powf(GAMMA);
            CUBE_FACES.map(|(normal, u, v)| Face {
                center: level_box.center + rotation * (normal * half),
                normal: rotation * normal,
                u_axis: rotation * u,
                v_axis: rotation * v,
                half_u: u.dot(half),
                half_v: v.dot(half),
                albedo,
            })
        })
        .collect()
}

fn pack(faces: &[Face], texel_size: f32) -> (Vec<FaceRect>, u32, u32) {
    let texels = |half: f32| ((2.0 * half / texel_size).ceil() as u32).clamp(1, MAX_FACE_TEXELS);
    let mut rects: Vec<FaceRect> = faces
        .iter()
        .map(|face| FaceRect {
            x: 0,
            y: 0,
            width: texels(face.half_u),
            height: texels(face.half_v),
        })
        .collect();
    let mut order: Vec<usize> = (0..rects.len()).collect();
    order.sort_by_key(|&index| std::cmp::Reverse(rects[index].height));
    let (mut x, mut y, mut row_height) = (0, 0, 0);
    for index in order {
        let rect = &mut rects[index];
        let padded_width = rect.width + PADDING * 2;
        if x + padded_width > ATLAS_WIDTH {
            x = 0;
            y += row_height;
            row_height = 0;
        }
        rect.x = x;
        rect.y = y;
        x += padded_width;
        row_height = row_height.max(rect.height + PADDING * 2);
    }
    (rects, ATLAS_WIDTH, (y + row_height).max(1))
}

fn intersect(
    obb: &OrientedBox,
    origin: Vec3,
    direction: Vec3,
    max_distance: f32,
    box_index: usize,
) -> Option<Hit> {
    let local_origin = obb.inverse_rotation * (origin - obb.center);
    let local_direction = obb.inverse_rotation * direction;
    let mut near = f32::MIN;
    let mut far = f32::MAX;
    for axis in 0..3 {
        let start = local_origin[axis];
        let step = local_direction[axis];
        let half = obb.half_size[axis];
        if step.abs() < 1e-8 {
            if start.abs() > half {
                return None;
            }
            continue;
        }
        let a = (-half - start) / step;
        let b = (half - start) / step;
        near = near.max(a.min(b));
        far = far.min(a.max(b));
    }
    if near > far || far < 0.0 || near > max_distance {
        return None;
    }
    let near = near.max(0.0);
    let local_hit = local_origin + local_direction * near;
    let face = local_face(local_hit, obb.half_size);
    let (_, u, v) = CUBE_FACES[face];
    let s = (u.dot(local_hit) / u.dot(obb.half_size) + 1.0) * 0.5;
    let t = (v.dot(local_hit) / v.dot(obb.half_size) + 1.0) * 0.5;
    Some(Hit {
        distance: near,
        face: box_index * CUBE_FACES.len() + face,
        s: s.clamp(0.0, 1.0),
        t: t.clamp(0.0, 1.0),
    })
}

fn local_face(local_hit: Vec3, half_size: Vec3) -> usize {
    let relative = local_hit / half_size;
    let absolute = relative.abs();
    if absolute.x >= absolute.y && absolute.x >= absolute.z {
        if relative.x > 0.0 { 0 } else { 1 }
    } else if absolute.y >= absolute.z {
        if relative.y > 0.0 { 2 } else { 3 }
    } else if relative.z > 0.0 {
        4
    } else {
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(center: Vec3, size: Vec3) -> LevelBox {
        LevelBox {
            center,
            size,
            rotation_degrees: Vec3::ZERO,
            color: [1.0; 3],
        }
    }

    fn lamp(position: Vec3) -> PointLight {
        PointLight {
            position,
            color: [1.0; 3],
            intensity: 10.0,
            range: 20.0,
        }
    }

    fn settings() -> BakeSettings {
        BakeSettings {
            texel_size: 1.0,
            samples: 16,
            bounces: 1,
        }
    }

    fn floor_value(map: &Lightmap, values: &[Vec3], x_fraction: f32) -> f32 {
        let rect = map.faces[2];
        let x = ((x_fraction * rect.width as f32) as u32).min(rect.width - 1) + rect.x + PADDING;
        let y = rect.height / 2 + rect.y + PADDING;
        values[(y * map.width + x) as usize].x
    }

    #[test]
    fn wall_blocks_direct_light() {
        let floor = solid(Vec3::new(0.0, -0.5, 0.0), Vec3::new(20.0, 1.0, 4.0));
        let wall = solid(Vec3::new(0.0, 3.0, 0.0), Vec3::new(0.2, 6.0, 4.0));
        let (map, direct) = bake_with_direct(
            &[floor, wall],
            &[lamp(Vec3::new(-5.0, 4.0, 0.0))],
            settings(),
        );
        assert!(floor_value(&map, &direct, 0.25) > 0.01);
        assert_eq!(floor_value(&map, &direct, 0.75), 0.0);
    }

    #[test]
    fn bounce_light_reaches_shadowed_floor() {
        let floor = solid(Vec3::new(0.0, -0.5, 0.0), Vec3::new(20.0, 1.0, 4.0));
        let ceiling = solid(Vec3::new(0.0, 6.5, 0.0), Vec3::new(20.0, 1.0, 4.0));
        let wall = solid(Vec3::new(0.0, 2.0, 0.0), Vec3::new(0.2, 4.0, 4.0));
        let (map, direct) = bake_with_direct(
            &[floor, ceiling, wall],
            &[lamp(Vec3::new(-5.0, 3.0, 0.0))],
            settings(),
        );
        assert_eq!(floor_value(&map, &direct, 0.75), 0.0);
        assert!(floor_value(&map, &map.indirect, 0.75) > 0.0);
    }

    #[test]
    fn faces_do_not_overlap_in_atlas() {
        let boxes = [
            solid(Vec3::ZERO, Vec3::new(3.0, 2.0, 1.0)),
            solid(Vec3::X * 5.0, Vec3::new(1.0, 4.0, 2.0)),
        ];
        let (rects, width, height) = pack(&build_faces(&boxes), 0.5);
        let padded = 2 * PADDING;
        for (index, a) in rects.iter().enumerate() {
            assert!(a.x + a.width + padded <= width && a.y + a.height + padded <= height);
            for b in &rects[index + 1..] {
                let separate = a.x + a.width + padded <= b.x
                    || b.x + b.width + padded <= a.x
                    || a.y + a.height + padded <= b.y
                    || b.y + b.height + padded <= a.y;
                assert!(separate);
            }
        }
    }

    #[test]
    fn half_floats_match_known_bits() {
        assert_eq!(f16_bits(1.0), 0x3c00);
        assert_eq!(f16_bits(0.5), 0x3800);
        assert_eq!(f16_bits(-3.0), 0);
        assert_eq!(f16_bits(1.0e6), 0x7bff);
    }
}
