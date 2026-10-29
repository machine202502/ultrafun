use glam::{EulerRot, Quat, Vec3};
use parry3d::math::Pose;
use parry3d::query::{self, Ray, RayCast, ShapeCastHit, ShapeCastOptions};
use parry3d::shape::{Capsule, Cuboid};

use crate::{Level, Zone};

pub const SKIN: f32 = 0.01;
const SURFACE_PROBE_INSET: f32 = 0.02;
const SURFACE_PROBE_HEIGHT: f32 = 0.05;

pub struct CollisionWorld {
    boxes: Vec<(Pose, Cuboid)>,
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
            boxes: level_boxes.chain(blocker_boxes).collect(),
        }
    }

    pub fn cast(&self, body: &Capsule, center: Vec3, motion: Vec3) -> Option<Hit> {
        let start = Pose::from_translation(center);
        let options = ShapeCastOptions {
            max_time_of_impact: 1.0,
            stop_at_penetration: false,
            ..ShapeCastOptions::default()
        };
        self.boxes
            .iter()
            .filter_map(|(pose, cuboid)| {
                query::cast_shapes(&start, motion, body, pose, Vec3::ZERO, cuboid, options)
                    .ok()
                    .flatten()
                    .map(|hit| (pose, cuboid, hit))
            })
            .min_by(|(_, _, a), (_, _, b)| a.time_of_impact.total_cmp(&b.time_of_impact))
            .map(|(pose, cuboid, hit)| Hit {
                fraction: hit.time_of_impact,
                normal: -hit.normal1,
                surface_normal: surface_normal_under(pose, cuboid, &hit),
            })
    }

    pub fn touches(&self, body: &Capsule, center: Vec3, max_distance: f32) -> Vec<Touch> {
        let start = Pose::from_translation(center);
        self.boxes
            .iter()
            .filter_map(|(pose, cuboid)| {
                query::contact(&start, body, pose, cuboid, max_distance)
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
            .filter_map(|(pose, cuboid)| {
                cuboid.cast_ray_and_get_normal(pose, ray, max_distance, true)
            })
            .min_by(|a, b| a.time_of_impact.total_cmp(&b.time_of_impact))
            .map(|intersection| (intersection.time_of_impact, intersection.normal))
    }

    pub fn overlaps(&self, body: &Capsule, center: Vec3) -> bool {
        let start = Pose::from_translation(center);
        self.boxes.iter().any(|(pose, cuboid)| {
            query::intersection_test(&start, body, pose, cuboid)
                .is_ok_and(|intersection| intersection.intersecting)
        })
    }
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
