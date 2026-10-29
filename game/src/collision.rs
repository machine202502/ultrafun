use glam::{EulerRot, Quat, Vec3};
use parry3d::bounding_volume::Aabb;
use parry3d::math::Pose;
use parry3d::query::{self, Ray, RayCast, ShapeCastHit, ShapeCastOptions};
use parry3d::shape::{Capsule, Cuboid, Shape};

use crate::{Level, Zone};

pub const SKIN: f32 = 0.01;
const SURFACE_PROBE_INSET: f32 = 0.02;
const SURFACE_PROBE_HEIGHT: f32 = 0.05;

struct WorldBox {
    pose: Pose,
    cuboid: Cuboid,
    bounds: Aabb,
}

pub struct CollisionWorld {
    boxes: Vec<WorldBox>,
}

pub struct Hit {
    pub fraction: f32,
    pub normal: Vec3,
    pub surface_normal: Vec3,
}

pub struct Touch {
    pub normal: Vec3,
    pub distance: f32,
}

impl CollisionWorld {
    pub fn new<'a>(level: &Level, blockers: impl Iterator<Item = &'a Zone>) -> Self {
        let level_boxes = level.boxes.iter().map(|level_box| {
            let angles = level_box.rotation_degrees;
            let rotation = Quat::from_euler(
                EulerRot::XYZ,
                angles.x.to_radians(),
                angles.y.to_radians(),
                angles.z.to_radians(),
            );
            (
                Pose::from_parts(level_box.center, rotation),
                Cuboid::new(level_box.size * 0.5),
            )
        });
        let blocker_boxes = blockers.map(|zone| {
            (
                Pose::from_translation(zone.center),
                Cuboid::new(zone.size * 0.5),
            )
        });
        Self {
            boxes: level_boxes
                .chain(blocker_boxes)
                .map(|(pose, cuboid)| WorldBox {
                    bounds: cuboid.compute_aabb(&pose),
                    pose,
                    cuboid,
                })
                .collect(),
        }
    }

    pub fn cast(&self, body: &Capsule, center: Vec3, motion: Vec3) -> Option<Hit> {
        let start = Pose::from_translation(center);
        let bounds = body.compute_aabb(&start);
        let swept = expand(&bounds, motion.abs());
        let options = ShapeCastOptions {
            max_time_of_impact: 1.0,
            stop_at_penetration: false,
            ..ShapeCastOptions::default()
        };
        self.boxes
            .iter()
            .filter(|world_box| intersects(&world_box.bounds, &swept))
            .filter_map(|world_box| {
                query::cast_shapes(
                    &start,
                    motion,
                    body,
                    &world_box.pose,
                    Vec3::ZERO,
                    &world_box.cuboid,
                    options,
                )
                .ok()
                .flatten()
                .map(|hit| (world_box, hit))
            })
            .min_by(|(_, a), (_, b)| a.time_of_impact.total_cmp(&b.time_of_impact))
            .map(|(world_box, hit)| Hit {
                fraction: hit.time_of_impact,
                normal: -hit.normal1,
                surface_normal: surface_normal_under(&world_box.pose, &world_box.cuboid, &hit),
            })
    }

    pub fn touches(&self, body: &Capsule, center: Vec3, max_distance: f32) -> Vec<Touch> {
        let start = Pose::from_translation(center);
        let bounds = expand(&body.compute_aabb(&start), Vec3::splat(max_distance));
        self.boxes
            .iter()
            .filter(|world_box| intersects(&world_box.bounds, &bounds))
            .filter_map(|world_box| {
                query::contact(
                    &start,
                    body,
                    &world_box.pose,
                    &world_box.cuboid,
                    max_distance,
                )
                .ok()
                .flatten()
            })
            .map(|contact| Touch {
                normal: -contact.normal1,
                distance: contact.dist,
            })
            .collect()
    }

    pub fn ray(&self, ray: &Ray, max_distance: f32) -> Option<(f32, Vec3)> {
        self.boxes
            .iter()
            .filter(|world_box| ray_reaches(&world_box.bounds, ray, max_distance))
            .filter_map(|world_box| {
                world_box
                    .cuboid
                    .cast_ray_and_get_normal(&world_box.pose, ray, max_distance, true)
            })
            .min_by(|a, b| a.time_of_impact.total_cmp(&b.time_of_impact))
            .map(|intersection| (intersection.time_of_impact, intersection.normal))
    }

    pub fn line_of_sight(&self, from: Vec3, to: Vec3) -> bool {
        let delta = to - from;
        let distance = delta.length();
        if distance < 1e-4 {
            return true;
        }
        let ray = Ray::new(from, delta / distance);
        self.ray(&ray, distance)
            .is_none_or(|(hit, _)| hit >= distance - 2.0 * SKIN)
    }

    pub fn overlaps(&self, body: &Capsule, center: Vec3) -> bool {
        let start = Pose::from_translation(center);
        let bounds = body.compute_aabb(&start);
        self.boxes
            .iter()
            .filter(|world_box| intersects(&world_box.bounds, &bounds))
            .any(|world_box| {
                query::intersection_test(&start, body, &world_box.pose, &world_box.cuboid)
                    .is_ok_and(|intersection| intersection.intersecting)
            })
    }
}

fn expand(bounds: &Aabb, amount: Vec3) -> Aabb {
    Aabb::new(bounds.mins - amount, bounds.maxs + amount)
}

fn intersects(a: &Aabb, b: &Aabb) -> bool {
    a.intersection(b).is_some()
}

fn ray_reaches(bounds: &Aabb, ray: &Ray, max_distance: f32) -> bool {
    let mut near = 0.0f32;
    let mut far = max_distance;
    for axis in 0..3 {
        let origin = ray.origin[axis];
        let direction = ray.dir[axis];
        let low = bounds.mins[axis];
        let high = bounds.maxs[axis];
        if direction.abs() < 1e-6 {
            if origin < low || origin > high {
                return false;
            }
            continue;
        }
        let first = (low - origin) / direction;
        let second = (high - origin) / direction;
        near = near.max(first.min(second));
        far = far.min(first.max(second));
        if near > far {
            return false;
        }
    }
    true
}

fn surface_normal_under(pose: &Pose, cuboid: &Cuboid, hit: &ShapeCastHit) -> Vec3 {
    let contact_normal = -hit.normal1;
    if contact_normal.y <= 0.0 {
        return contact_normal;
    }
    let toward_obstacle = Vec3::new(hit.normal1.x, 0.0, hit.normal1.z).normalize_or_zero();
    let contact_point = pose.transform_point(hit.witness2);
    let origin =
        contact_point + toward_obstacle * SURFACE_PROBE_INSET + Vec3::Y * SURFACE_PROBE_HEIGHT;
    let ray = Ray::new(origin, Vec3::NEG_Y);
    cuboid
        .cast_ray_and_get_normal(pose, &ray, SURFACE_PROBE_HEIGHT * 2.0, false)
        .map_or(contact_normal, |intersection| intersection.normal)
}
