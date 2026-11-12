use game::{AttackState, Game};
use glam::{Mat4, Vec3};

use crate::render::ModelInstance;

pub fn enemies(game: &Game, alpha: f32, effects: &crate::fx::Effects) -> Vec<ModelInstance> {
    game.enemies
        .iter()
        .map(|enemy| {
            let displacement = enemy.position - enemy.previous_position;
            let speed = displacement.length() / game::STEP_SECONDS;
            let to_player = game.player.position - enemy.position;
            let heading =
                if displacement.x * displacement.x + displacement.z * displacement.z > 1e-5 {
                    displacement
                } else {
                    to_player
                };
            let yaw = (-heading.x).atan2(-heading.z);
            let phase = game.elapsed_seconds() * 13.0 + enemy.id as f32 * 1.7;
            let stride =
                phase.sin() * (speed / game.config.grunt.run_speed.max(0.001)).min(1.0) * 0.65;
            let mut joints = vec![
                ("left_leg".into(), Mat4::from_rotation_x(stride)),
                ("right_leg".into(), Mat4::from_rotation_x(-stride)),
                (
                    "left_shin".into(),
                    Mat4::from_rotation_x(stride.max(0.0) * 0.8),
                ),
                (
                    "right_shin".into(),
                    Mat4::from_rotation_x((-stride).max(0.0) * 0.8),
                ),
            ];
            let attack = match enemy.attack {
                AttackState::Ready => 0.0,
                AttackState::Windup { remaining_seconds } => {
                    -1.0 + remaining_seconds / game.config.grunt.windup_seconds
                }
                AttackState::Recover { remaining_seconds } => {
                    (remaining_seconds / game.config.grunt.recover_seconds).powi(3) * 1.3
                }
            };
            joints.push((
                "right_arm".into(),
                Mat4::from_rotation_x(-stride * 0.6 - attack),
            ));
            joints.push((
                "left_arm".into(),
                Mat4::from_rotation_x(stride * 0.6 - attack * 0.6),
            ));
            joints.push((
                "right_forearm".into(),
                Mat4::from_rotation_x(-0.35 - attack.abs() * 0.6),
            ));
            joints.push(("left_forearm".into(), Mat4::from_rotation_x(-0.35)));
            joints.push((
                "torso".into(),
                Mat4::from_rotation_x(attack * 0.12 - effects.hit_reaction(enemy.id) * 0.25),
            ));
            ModelInstance {
                model: 0,
                transform: Mat4::from_translation(enemy.interpolated_position(alpha))
                    * Mat4::from_rotation_y(yaw)
                    * Mat4::from_scale(Vec3::splat(game.config.grunt.height / 2.05)),
                joints,
            }
        })
        .collect()
}

pub fn weapon_transform(speed: f32, seconds: f32, recoil: f32) -> Mat4 {
    let running = (speed / 16.5).min(1.0);
    let sway = Vec3::new(
        (seconds * 8.0).sin() * 0.009,
        (seconds * 16.0).cos() * 0.006,
        0.0,
    ) * running;
    Mat4::from_translation(
        Vec3::new(0.23, -0.24, -0.54) + sway + Vec3::new(0.0, recoil * 0.025, recoil * 0.08),
    ) * Mat4::from_rotation_x(recoil * 0.12)
        * Mat4::from_rotation_y(-0.14)
        * Mat4::from_rotation_z(recoil * 0.08)
}

pub fn viewmodel(game: &Game, speed: f32, recoil: f32) -> Vec<ModelInstance> {
    vec![ModelInstance {
        model: 1 + game.weapon.selected.index(),
        transform: weapon_transform(speed, game.elapsed_seconds(), recoil),
        joints: vec![
            (
                "bolt".into(),
                Mat4::from_translation(Vec3::Z * recoil * 0.06),
            ),
            (
                "core".into(),
                Mat4::from_rotation_z(game.charge_fraction() * game.elapsed_seconds() * 7.0),
            ),
        ],
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recoil_moves_the_entire_weapon_and_muzzle_together() {
        let muzzle = Vec3::new(0.0, 0.06, -0.48);
        let idle = weapon_transform(0.0, 0.0, 0.0);
        let kicked = weapon_transform(0.0, 0.0, 1.0);
        assert!(
            kicked
                .transform_point3(muzzle)
                .distance(idle.transform_point3(muzzle))
                > 0.02
        );
        assert!((kicked.transform_vector3(Vec3::Y).length() - 1.0).abs() < 1e-5);
    }
}
