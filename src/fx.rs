use game::{AttackState, Game, GameEvent, LevelBox, Target, TargetKind};
use glam::{EulerRot, Quat, Vec2, Vec3};

use crate::render::{LightingStyle, PointLight};

const IMPACT_FLASH_SECONDS: f32 = 0.09;
const IMPACT_FLASH_SIZE: f32 = 0.18;
const IMPACT_FLASH_COLOR: [f32; 3] = [1.0, 0.8, 0.35];
const TRACER_SECONDS: f32 = 0.08;
const TRACER_THICKNESS: f32 = 0.055;
const TRACER_COLOR: [f32; 3] = [1.0, 0.92, 0.6];
const SPARK_SECONDS: f32 = 0.22;
const SPARK_COUNT: usize = 8;
const SPARK_SPEED: f32 = 4.0;
const SPARK_SPREAD: f32 = 3.0;
const SPARK_GRAVITY: f32 = 22.0;
const SPARK_SIZE: f32 = 0.07;
const SPARK_COLOR: [f32; 3] = [1.0, 0.72, 0.2];
const BLOOD_COUNT: usize = 9;
const BLOOD_SECONDS: f32 = 1.8;
const BLOOD_SIZE: f32 = 0.07;
const BLOOD_COLOR: [f32; 3] = [0.42, 0.04, 0.03];
const GUN_BODY_COLOR: [f32; 3] = [0.17, 0.18, 0.2];
const GUN_GRIP_COLOR: [f32; 3] = [0.3, 0.17, 0.1];
const GUN_SIGHT_COLOR: [f32; 3] = [0.75, 0.9, 1.0];
const GUN_SLEEVE_COLOR: [f32; 3] = [0.22, 0.2, 0.17];
const GUN_FORWARD: f32 = 0.62;
const GUN_RIGHT: f32 = 0.22;
const GUN_DOWN: f32 = 0.19;
const GUN_MUZZLE_FORWARD: f32 = 0.315;
const VIEW_KICK_PITCH: f32 = 0.02;
const VIEW_KICK_YAW: f32 = 0.003;
const VIEW_KICK_DECAY: f32 = 16.0;
const GUN_KICK_BACK: f32 = 0.07;
const GUN_KICK_UP: f32 = 0.02;
const GUN_KICK_PITCH_DEGREES: f32 = 5.0;
const GUN_KICK_ROLL_DEGREES: f32 = 9.0;
const TARGET_FULL_HEALTH_COLOR: [f32; 3] = [0.9, 0.15, 0.1];
const TARGET_LOW_HEALTH_COLOR: [f32; 3] = [0.25, 0.05, 0.05];
const GRUNT_COLOR: [f32; 3] = [0.8, 0.72, 0.55];
const GRUNT_WINDUP_COLOR: [f32; 3] = [1.0, 0.5, 0.1];
const PART_SIZE: f32 = 0.2;
const DOOR_COLOR: [f32; 3] = [0.45, 0.08, 0.06];
const MACHINE_FULL_HEALTH_COLOR: [f32; 3] = [0.55, 0.35, 0.18];
const MACHINE_LOW_HEALTH_COLOR: [f32; 3] = [0.15, 0.1, 0.06];
const PART_COLOR: [f32; 3] = [0.3, 0.95, 0.4];
const KEY_SPIN_DEGREES: f32 = 90.0;
const KEY_BOB_HZ: f32 = 2.0;
const KEY_BOB_HEIGHT: f32 = 0.1;
const SWITCH_ACTIVATED_SHADE: f32 = 0.2;

const MUZZLE_FLASH: Flash = Flash {
    position: Vec3::ZERO,
    color: [1.0, 0.8, 0.5],
    intensity: 8.0,
    range: 9.0,
    seconds: 0.06,
    remaining_seconds: 0.06,
};
const KILL_FLASH: Flash = Flash {
    position: Vec3::ZERO,
    color: [1.0, 0.45, 0.15],
    intensity: 25.0,
    range: 10.0,
    seconds: 0.15,
    remaining_seconds: 0.15,
};
const PART_GLOW_INTENSITY: f32 = 1.0;
const PART_GLOW_RANGE: f32 = 3.0;

struct Tracer {
    start: Vec3,
    end: Vec3,
    remaining_seconds: f32,
}

struct Spark {
    position: Vec3,
    velocity: Vec3,
    remaining_seconds: f32,
    color: [f32; 3],
}

struct Blood {
    position: Vec3,
    velocity: Vec3,
    floor_y: f32,
    remaining_seconds: f32,
    landed: bool,
}

#[derive(Clone, Copy)]
struct Flash {
    position: Vec3,
    color: [f32; 3],
    intensity: f32,
    range: f32,
    seconds: f32,
    remaining_seconds: f32,
}

#[derive(Default)]
pub struct Effects {
    impacts: Vec<(Vec3, f32)>,
    tracers: Vec<Tracer>,
    sparks: Vec<Spark>,
    blood: Vec<Blood>,
    flashes: Vec<Flash>,
    recoil: f32,
    view_kick: Vec2,
    seed: u32,
}

impl Effects {
    pub fn handle(&mut self, event: &GameEvent, muzzle: Vec3, floor_y: Option<f32>) {
        match event {
            GameEvent::ShotImpact { position, normal } => {
                self.impacts.push((*position, IMPACT_FLASH_SECONDS));
                self.spawn_sparks(*position, *normal, SPARK_COLOR);
            }
            GameEvent::EnemyHit {
                position, normal, ..
            } => self.spawn_blood(*position, *normal, floor_y.unwrap_or(position.y - 3.0)),
            GameEvent::ShotFired { end, .. } => {
                self.tracers.push(Tracer {
                    start: muzzle,
                    end: *end,
                    remaining_seconds: TRACER_SECONDS,
                });
                self.flashes.push(Flash {
                    position: muzzle,
                    ..MUZZLE_FLASH
                });
                self.recoil = 1.0;
                self.seed = self
                    .seed
                    .wrapping_mul(1_664_525)
                    .wrapping_add(1_013_904_223);
                let jitter = (self.seed >> 9) as f32 / 8_388_608.0 - 1.0;
                self.view_kick += Vec2::new(jitter * VIEW_KICK_YAW, VIEW_KICK_PITCH);
            }
            GameEvent::EnemyKilled { position, .. } => self.flashes.push(Flash {
                position: *position,
                ..KILL_FLASH
            }),
            _ => {}
        }
    }

    fn spawn_blood(&mut self, position: Vec3, normal: Vec3, floor_y: f32) {
        for index in 0..BLOOD_COUNT {
            self.seed = self
                .seed
                .wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
            let noise = (self.seed >> 8) as f32 / 16_777_216.0;
            let angle = index as f32 / BLOOD_COUNT as f32 * std::f32::consts::TAU + noise * 2.0;
            let spread = Vec3::new(angle.cos(), 0.0, angle.sin());
            self.blood.push(Blood {
                position: position + normal * 0.08,
                velocity: normal * 1.6
                    + spread * (1.2 + noise * 2.0)
                    + Vec3::Y * (1.4 + noise * 1.6),
                floor_y,
                remaining_seconds: BLOOD_SECONDS,
                landed: false,
            });
        }
    }

    fn spawn_sparks(&mut self, position: Vec3, normal: Vec3, color: [f32; 3]) {
        let helper = if normal.x.abs() < 0.9 {
            Vec3::X
        } else {
            Vec3::Y
        };
        let side = normal.cross(helper).normalize_or_zero();
        let other = normal.cross(side);
        for index in 0..SPARK_COUNT {
            self.seed = self
                .seed
                .wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
            let noise = (self.seed >> 8) as f32 / 16_777_216.0;
            let angle = index as f32 / SPARK_COUNT as f32 * std::f32::consts::TAU + noise;
            let spread = side * angle.cos() + other * angle.sin();
            let velocity = spread * SPARK_SPREAD * (0.4 + noise * 0.6)
                + normal * SPARK_SPEED * (0.5 + noise * 0.5);
            self.sparks.push(Spark {
                position: position + normal * 0.06,
                velocity,
                remaining_seconds: SPARK_SECONDS * (0.55 + noise * 0.45),
                color,
            });
        }
    }

    pub fn advance(&mut self, seconds: f32) {
        for (_, remaining) in &mut self.impacts {
            *remaining -= seconds;
        }
        self.impacts.retain(|(_, remaining)| *remaining > 0.0);
        for tracer in &mut self.tracers {
            tracer.remaining_seconds -= seconds;
        }
        self.tracers.retain(|tracer| tracer.remaining_seconds > 0.0);
        for spark in &mut self.sparks {
            spark.velocity.y -= SPARK_GRAVITY * seconds;
            spark.position += spark.velocity * seconds;
            spark.remaining_seconds -= seconds;
        }
        self.sparks.retain(|spark| spark.remaining_seconds > 0.0);
        for drop in &mut self.blood {
            if !drop.landed {
                drop.velocity.y -= SPARK_GRAVITY * seconds;
                drop.position += drop.velocity * seconds;
                if drop.position.y <= drop.floor_y + 0.01 {
                    drop.position.y = drop.floor_y + 0.01;
                    drop.velocity = Vec3::ZERO;
                    drop.landed = true;
                }
            }
            drop.remaining_seconds -= seconds;
        }
        self.blood.retain(|drop| drop.remaining_seconds > 0.0);
        for flash in &mut self.flashes {
            flash.remaining_seconds -= seconds;
        }
        self.flashes.retain(|flash| flash.remaining_seconds > 0.0);
        self.recoil = (self.recoil - seconds * 14.0).max(0.0);
        self.view_kick *= (-seconds * VIEW_KICK_DECAY).exp();
        if self.view_kick.length_squared() < 1e-8 {
            self.view_kick = Vec2::ZERO;
        }
    }

    pub fn dynamic_lights(&self, game: &Game, alpha: f32) -> Vec<PointLight> {
        let flashes = self.flashes.iter().map(|flash| PointLight {
            position: flash.position,
            color: flash.color,
            intensity: flash.intensity * flash.remaining_seconds / flash.seconds,
            range: flash.range,
        });
        let part_glows = game.parts.iter().map(|part| PointLight {
            position: part.interpolated_position(alpha),
            color: PART_COLOR,
            intensity: PART_GLOW_INTENSITY,
            range: PART_GLOW_RANGE,
        });
        flashes.chain(part_glows).collect()
    }

    pub fn view_kick(&self) -> Vec2 {
        self.view_kick
    }

    pub fn recoil(&self) -> f32 {
        self.recoil
    }

    pub fn glowing_boxes(&self) -> Vec<LevelBox> {
        let sparks = self.sparks.iter().map(|spark| {
            let t = (spark.remaining_seconds / SPARK_SECONDS).clamp(0.0, 1.0);
            LevelBox {
                center: spark.position,
                size: Vec3::splat(SPARK_SIZE * (0.35 + 0.65 * t)),
                rotation_degrees: Vec3::ZERO,
                color: [0, 1, 2].map(|channel| spark.color[channel] * (0.25 + 0.75 * t)),
            }
        });
        let impacts = self.impacts.iter().map(|(position, remaining)| {
            let t = (remaining / IMPACT_FLASH_SECONDS).clamp(0.0, 1.0);
            LevelBox {
                center: *position,
                size: Vec3::splat(IMPACT_FLASH_SIZE * (0.4 + 0.6 * (1.0 - t))),
                rotation_degrees: Vec3::ZERO,
                color: [0, 1, 2].map(|channel| IMPACT_FLASH_COLOR[channel] * (0.3 + 0.7 * t)),
            }
        });
        sparks.chain(impacts).collect()
    }

    pub fn dynamic_boxes(&self, game: &Game, alpha: f32) -> Vec<LevelBox> {
        let grunt = &game.config.grunt;
        let enemy_boxes = game.enemies.iter().map(|enemy| LevelBox {
            center: enemy.interpolated_position(alpha) + Vec3::Y * grunt.height * 0.5,
            size: Vec3::new(grunt.radius * 2.0, grunt.height, grunt.radius * 2.0),
            rotation_degrees: Vec3::ZERO,
            color: match enemy.attack {
                AttackState::Windup { .. } => GRUNT_WINDUP_COLOR,
                AttackState::Ready | AttackState::Recover { .. } => GRUNT_COLOR,
            },
        });
        let part_boxes = game.parts.iter().map(|part| LevelBox {
            center: part.interpolated_position(alpha),
            size: Vec3::splat(PART_SIZE),
            rotation_degrees: Vec3::ZERO,
            color: PART_COLOR,
        });
        let target_boxes = target_boxes(&game.targets);
        let door_boxes = game.closed_doors().map(|door| LevelBox {
            center: door.center,
            size: door.size,
            rotation_degrees: Vec3::ZERO,
            color: DOOR_COLOR,
        });
        let crusher_boxes = crusher_boxes(game, alpha);
        let tracer_boxes = self.tracers.iter().filter_map(tracer_box);
        let blood_boxes = self.blood.iter().map(|drop| {
            let t = (drop.remaining_seconds / BLOOD_SECONDS).clamp(0.0, 1.0);
            let size = if drop.landed {
                Vec3::new(BLOOD_SIZE * 2.4, BLOOD_SIZE * 0.25, BLOOD_SIZE * 2.4)
            } else {
                Vec3::splat(BLOOD_SIZE)
            };
            LevelBox {
                center: drop.position,
                size,
                rotation_degrees: Vec3::ZERO,
                color: [0, 1, 2].map(|channel| BLOOD_COLOR[channel] * (0.55 + 0.45 * t)),
            }
        });
        target_boxes
            .chain(door_boxes)
            .chain(crusher_boxes)
            .chain(script_boxes(game))
            .chain(enemy_boxes)
            .chain(part_boxes)
            .chain(tracer_boxes)
            .chain(blood_boxes)
            .collect()
    }
}
pub fn glowing_boxes(game: &Game) -> impl Iterator<Item = LevelBox> + '_ {
    let seconds = game.elapsed_seconds();
    let hazards = game
        .hazards
        .iter()
        .filter(|hazard| hazard.enabled)
        .map(|hazard| LevelBox {
            center: hazard.zone.center,
            size: hazard.zone.size,
            rotation_degrees: Vec3::ZERO,
            color: hazard.color,
        });
    let keys = game
        .keys
        .iter()
        .filter(|key| !key.collected)
        .map(move |key| LevelBox {
            center: key.zone.center + Vec3::Y * (seconds * KEY_BOB_HZ).sin() * KEY_BOB_HEIGHT,
            size: key.zone.size,
            rotation_degrees: Vec3::new(0.0, (seconds * KEY_SPIN_DEGREES).rem_euclid(360.0), 0.0),
            color: key.color,
        });
    hazards.chain(keys)
}

fn script_boxes(game: &Game) -> impl Iterator<Item = LevelBox> + '_ {
    let doors = game
        .doors
        .iter()
        .filter(|door| !door.open)
        .flat_map(|door| {
            let color = door.color;
            door.zones.iter().map(move |zone| LevelBox {
                center: zone.center,
                size: zone.size,
                rotation_degrees: Vec3::ZERO,
                color,
            })
        });
    let breakables = game
        .breakables
        .iter()
        .filter(|breakable| !breakable.broken)
        .flat_map(|breakable| {
            let color = breakable.color;
            breakable.zones.iter().map(move |zone| LevelBox {
                center: zone.center,
                size: zone.size,
                rotation_degrees: Vec3::ZERO,
                color,
            })
        });
    let switches = game.switches.iter().map(|switch| LevelBox {
        center: switch.zone.center,
        size: switch.zone.size,
        rotation_degrees: Vec3::ZERO,
        color: if switch.activated {
            switch.color.map(|channel| channel * SWITCH_ACTIVATED_SHADE)
        } else {
            switch.color
        },
    });
    doors.chain(breakables).chain(switches)
}

fn crusher_boxes(game: &Game, alpha: f32) -> impl Iterator<Item = LevelBox> + '_ {
    let seconds = game.elapsed_seconds();
    game.level().crushers.iter().map(move |crusher| LevelBox {
        center: crusher
            .center_at(seconds - game::STEP_SECONDS)
            .lerp(crusher.center_at(seconds), alpha),
        size: crusher.size,
        rotation_degrees: Vec3::ZERO,
        color: crusher.color,
    })
}

fn target_boxes(targets: &[Target]) -> impl Iterator<Item = LevelBox> + '_ {
    targets.iter().map(|target| {
        let health = (target.health / target.max_health).clamp(0.0, 1.0);
        LevelBox {
            center: target.center,
            size: target.size,
            rotation_degrees: Vec3::ZERO,
            color: match target.kind {
                TargetKind::Practice => {
                    lerp_color(TARGET_LOW_HEALTH_COLOR, TARGET_FULL_HEALTH_COLOR, health)
                }
                TargetKind::Machine => {
                    lerp_color(MACHINE_LOW_HEALTH_COLOR, MACHINE_FULL_HEALTH_COLOR, health)
                }
            },
        }
    })
}

fn tracer_box(tracer: &Tracer) -> Option<LevelBox> {
    let span = tracer.end - tracer.start;
    let length = span.length();
    if length <= f32::EPSILON {
        return None;
    }
    let t = (tracer.remaining_seconds / TRACER_SECONDS).clamp(0.0, 1.0);
    let (x, y, z) = Quat::from_rotation_arc(Vec3::Z, span / length).to_euler(EulerRot::XYZ);
    Some(LevelBox {
        center: tracer.start + span * 0.5,
        size: Vec3::new(
            TRACER_THICKNESS * (0.3 + 0.7 * t),
            TRACER_THICKNESS * (0.3 + 0.7 * t),
            length,
        ),
        rotation_degrees: Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees()),
        color: [0, 1, 2].map(|channel| TRACER_COLOR[channel] * (0.12 + 0.88 * t)),
    })
}

pub fn viewmodel_boxes(
    eye: Vec3,
    forward: Vec3,
    speed: f32,
    seconds: f32,
    recoil: f32,
) -> Vec<LevelBox> {
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(forward).normalize_or_zero();
    let run_speed = 16.5;
    let sway = (seconds * 9.0).sin() * (speed / run_speed).min(1.0) * 0.012;
    let origin = eye
        + right * GUN_RIGHT
        + up * (-GUN_DOWN + sway + recoil * GUN_KICK_UP)
        + forward * (GUN_FORWARD - recoil * GUN_KICK_BACK);
    let basis = glam::Mat3::from_cols(right, up, -forward)
        * glam::Mat3::from_axis_angle(Vec3::Y, 0.05)
        * glam::Mat3::from_axis_angle(Vec3::X, -recoil * GUN_KICK_PITCH_DEGREES.to_radians())
        * glam::Mat3::from_axis_angle(Vec3::Z, 0.05 + recoil * GUN_KICK_ROLL_DEGREES.to_radians());
    let (x, y, z) = Quat::from_mat3(&basis).to_euler(EulerRot::XYZ);
    let rotation = Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees());
    let parts = [
        (
            Vec3::new(0.0, -0.015, 0.12),
            Vec3::new(0.05, 0.055, 0.27),
            GUN_BODY_COLOR,
        ),
        (
            Vec3::new(0.0, 0.035, 0.13),
            Vec3::new(0.05, 0.05, 0.3),
            GUN_BODY_COLOR,
        ),
        (
            Vec3::new(0.0, -0.135, 0.0),
            Vec3::new(0.05, 0.18, 0.085),
            GUN_GRIP_COLOR,
        ),
        (
            Vec3::new(0.0, -0.075, 0.085),
            Vec3::new(0.05, 0.02, 0.055),
            GUN_BODY_COLOR,
        ),
        (
            Vec3::new(0.0, 0.035, 0.295),
            Vec3::new(0.028, 0.028, 0.04),
            GUN_BODY_COLOR,
        ),
        (
            Vec3::new(0.0, 0.068, 0.02),
            Vec3::new(0.032, 0.016, 0.02),
            GUN_SIGHT_COLOR,
        ),
        (
            Vec3::new(0.0, 0.068, 0.2),
            Vec3::new(0.016, 0.016, 0.02),
            GUN_SIGHT_COLOR,
        ),
        (
            Vec3::new(0.02, -0.23, -0.05),
            Vec3::new(0.08, 0.1, 0.18),
            GUN_GRIP_COLOR,
        ),
        (
            Vec3::new(0.04, -0.27, -0.22),
            Vec3::new(0.1, 0.12, 0.28),
            GUN_SLEEVE_COLOR,
        ),
    ];
    parts
        .into_iter()
        .map(|(offset, size, color)| LevelBox {
            center: origin + right * offset.x + up * offset.y + forward * offset.z,
            size,
            rotation_degrees: rotation,
            color,
        })
        .collect()
}
fn lerp_color(from: [f32; 3], to: [f32; 3], amount: f32) -> [f32; 3] {
    [0, 1, 2].map(|channel| from[channel] + (to[channel] - from[channel]) * amount)
}

pub fn muzzle_position(eye: Vec3, forward: Vec3) -> Vec3 {
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(forward);
    eye + right * GUN_RIGHT + up * -GUN_DOWN + forward * (GUN_FORWARD + GUN_MUZZLE_FORWARD)
}

pub fn lamp_fixtures(lights: &[PointLight], style: &LightingStyle) -> Vec<LevelBox> {
    if !style.lamp_fixtures {
        return Vec::new();
    }
    lights
        .iter()
        .map(|light| LevelBox {
            center: light.position,
            size: Vec3::splat(style.fixture_size),
            rotation_degrees: Vec3::ZERO,
            color: light.color,
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracer_fades_and_thins_with_time() {
        let tracer = Tracer {
            start: Vec3::ZERO,
            end: Vec3::new(0.0, 0.0, -10.0),
            remaining_seconds: TRACER_SECONDS,
        };
        let fresh = tracer_box(&tracer).unwrap();
        let faded = tracer_box(&Tracer {
            remaining_seconds: TRACER_SECONDS * 0.2,
            ..tracer
        })
        .unwrap();
        assert!(faded.size.x < fresh.size.x);
        assert!(faded.color[0] < fresh.color[0]);
    }

    #[test]
    fn impact_spawns_sparks_that_fall_and_disappear() {
        let mut effects = Effects::default();
        effects.spawn_sparks(Vec3::new(0.0, 0.0, -3.0), Vec3::Y, SPARK_COLOR);
        assert_eq!(effects.sparks.len(), SPARK_COUNT);
        effects.advance(0.05);
        assert!(
            effects
                .sparks
                .iter()
                .any(|spark| spark.position != Vec3::new(0.0, 0.0, -3.0))
        );
        effects.advance(0.2);
        assert!(effects.sparks.iter().all(|spark| spark.velocity.y < 0.0));
        effects.advance(1.0);
        assert!(effects.sparks.is_empty());
    }

    #[test]
    fn firing_kicks_the_view_then_settles() {
        let mut effects = Effects::default();
        assert_eq!(effects.view_kick(), Vec2::ZERO);
        effects.handle(
            &GameEvent::ShotFired {
                origin: Vec3::ZERO,
                end: Vec3::new(0.0, 0.0, -5.0),
            },
            Vec3::ZERO,
            None,
        );
        assert!(effects.view_kick().y > 0.0);
        assert!(effects.recoil > 0.0);
        effects.advance(0.5);
        assert!(effects.view_kick().length() < 1e-4);
        assert_eq!(effects.recoil, 0.0);
    }

    #[test]
    fn enemy_hits_spray_blood_that_lands_on_the_floor() {
        let mut effects = Effects::default();
        effects.handle(
            &GameEvent::EnemyHit {
                id: 7,
                position: Vec3::new(0.0, 1.5, -3.0),
                normal: Vec3::Z,
            },
            Vec3::ZERO,
            Some(0.0),
        );
        assert_eq!(effects.blood.len(), BLOOD_COUNT);
        assert!(effects.blood.iter().all(|drop| !drop.landed));
        effects.advance(1.0);
        assert!(effects.blood.iter().all(|drop| drop.landed));
        assert!(
            effects
                .blood
                .iter()
                .all(|drop| (drop.position.y - 0.01).abs() < 0.05)
        );
    }

    #[test]
    fn muzzle_sits_in_front_of_the_eye() {
        let tracer = Tracer {
            start: Vec3::ZERO,
            end: Vec3::new(0.0, 0.0, -5.0),
            remaining_seconds: TRACER_SECONDS,
        };
        assert!(tracer_box(&tracer).is_some());
    }
}
