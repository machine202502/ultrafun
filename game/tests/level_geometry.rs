use game::{LevelBox, coplanar_face_overlaps};
use glam::Vec3;

fn box_at(center: Vec3, size: Vec3) -> LevelBox {
    LevelBox {
        center,
        size,
        rotation_degrees: Vec3::ZERO,
        material: None,
        color: [1.0, 1.0, 1.0],
    }
}

#[test]
fn butt_jointed_walls_do_not_conflict() {
    let first = box_at(Vec3::new(0.0, 1.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
    let second = box_at(Vec3::new(2.0, 1.0, 0.0), Vec3::new(2.0, 2.0, 2.0));
    assert!(coplanar_face_overlaps(&[first, second]).is_empty());
}

#[test]
fn coplanar_faces_that_overlap_are_reported() {
    let first = box_at(Vec3::new(0.0, 1.0, 0.0), Vec3::new(2.0, 2.0, 4.0));
    let second = box_at(Vec3::new(1.0, 1.0, 0.0), Vec3::new(2.0, 2.0, 4.0));
    let overlaps = coplanar_face_overlaps(&[first, second]);
    assert_eq!(overlaps.len(), 2);
    assert!(
        overlaps
            .iter()
            .any(|overlap| overlap.axis == 1 && (overlap.plane - 2.0).abs() < 1e-6)
    );
    assert!(
        overlaps
            .iter()
            .any(|overlap| overlap.axis == 2 && (overlap.plane - 2.0).abs() < 1e-6)
    );
}

#[test]
fn wall_standing_on_a_floor_is_not_a_conflict() {
    let floor = box_at(Vec3::new(0.0, -0.5, 0.0), Vec3::new(8.0, 1.0, 8.0));
    let wall = box_at(Vec3::new(0.0, 1.5, 0.0), Vec3::new(1.0, 3.0, 1.0));
    assert!(coplanar_face_overlaps(&[floor, wall]).is_empty());
}

#[test]
fn nested_box_is_not_a_conflict() {
    let outer = box_at(Vec3::ZERO, Vec3::new(4.0, 4.0, 4.0));
    let inner = box_at(Vec3::new(1.0, 0.0, -1.0), Vec3::new(1.0, 1.0, 1.0));
    assert!(coplanar_face_overlaps(&[outer, inner]).is_empty());
}

#[test]
fn flush_nested_box_face_is_a_conflict() {
    let outer = box_at(Vec3::ZERO, Vec3::new(4.0, 4.0, 4.0));
    let flush = box_at(Vec3::new(0.0, 0.0, 1.0), Vec3::new(2.0, 2.0, 2.0));
    assert_eq!(coplanar_face_overlaps(&[outer, flush]).len(), 1);
}

#[test]
fn nearly_coplanar_faces_within_a_millimeter_are_reported() {
    let first = box_at(Vec3::new(0.0, 1.0, 0.0), Vec3::new(2.0, 2.0, 4.0));
    let second = box_at(Vec3::new(1.0, 1.0, 0.0005), Vec3::new(2.0, 2.0, 4.0));
    let overlaps = coplanar_face_overlaps(&[first, second]);
    assert!(
        overlaps
            .iter()
            .any(|overlap| overlap.axis == 2 && (overlap.plane - 2.0).abs() < 1e-6)
    );
}

#[test]
fn separated_boxes_do_not_conflict() {
    let first = box_at(Vec3::new(0.0, 1.0, 0.0), Vec3::ONE);
    let second = box_at(Vec3::new(5.0, 1.0, 0.0), Vec3::ONE);
    assert!(coplanar_face_overlaps(&[first, second]).is_empty());
}
