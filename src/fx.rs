use game::{AttackState, Game, GameEvent, LevelBox, Target, TargetKind};
use glam::{EulerRot, Quat, Vec3};

use crate::render::{LightingStyle, PointLight};

const IMPACT_MARK_SECONDS: f32 = 2.0;
const IMPACT_MARK_SIZE: f32 = 0.15;
const IMPACT_MARK_COLOR: [f32; 3] = [1.0, 0.85, 0.3];
const TRACER_SECONDS: f32 = 0.1;
const TRACER_THICKNESS: f32 = 0.04;
const TRACER_COLOR: [f32; 3] = [1.0, 0.95, 0.7];
const TARGET_FULL_HEALTH_COLOR: [f32; 3] = [0.9, 0.15, 0.1];
const TARGET_LOW_HEALTH_COLOR: [f32; 3] = [0.25, 0.05, 0.05];
const GRUNT_COLOR: [f32; 3] = [0.8, 0.72, 0.55];
const GRUNT_WINDUP_COLOR: [f32; 3] = [1.0, 0.5, 0.1];
const PART_SIZE: f32 = 0.2;
const DOOR_COLOR: [f32; 3] = [0.45, 0.08, 0.06];
const MACHINE_FULL_HEALTH_COLOR: [f32; 3] = [0.55, 0.35, 0.18];
const MACHINE_LOW_HEALTH_COLOR: [f32; 3] = [0.15, 0.1, 0.06];
const PART_COLOR: [f32; 3] = [0.3, 0.95, 0.4];

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
const MUZZLE_OFFSET: Vec3 = Vec3::new(0.35, -0.3, 0.6);

struct Tracer {
    start: Vec3,
    end: Vec3,
    remaining_seconds: f32,
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
    impact_marks: Vec<(Vec3, f32)>,
    tracers: Vec<Tracer>,
    flashes: Vec<Flash>,
}

impl Effects {
    pub fn handle(&mut self, event: &GameEvent, muzzle: Vec3) {
        match event {
            GameEvent::ShotImpact { position, normal } => {
                let surface_offset = *normal * IMPACT_MARK_SIZE * 0.5;
                self.impact_marks
                    .push((*position + surface_offset, IMPACT_MARK_SECONDS));
            }
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
            }
            GameEvent::EnemyKilled { position, .. } => self.flashes.push(Flash {
                position: *position,
                ..KILL_FLASH
            }),
            _ => {}
        }
    }

    pub fn advance(&mut self, seconds: f32) {
        for (_, remaining) in &mut self.impact_marks {
            *remaining -= seconds;
        }
        self.impact_marks.retain(|(_, remaining)| *remaining > 0.0);
        for tracer in &mut self.tracers {
            tracer.remaining_seconds -= seconds;
        }
        self.tracers.retain(|tracer| tracer.remaining_seconds > 0.0);
        for flash in &mut self.flashes {
            flash.remaining_seconds -= seconds;
        }
        self.flashes.retain(|flash| flash.remaining_seconds > 0.0);
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
        let impact_boxes = self.impact_marks.iter().map(|(position, _)| LevelBox {
            center: *position,
            size: Vec3::splat(IMPACT_MARK_SIZE),
            rotation_degrees: Vec3::ZERO,
            color: IMPACT_MARK_COLOR,
        });
        let tracer_boxes = self.tracers.iter().filter_map(tracer_box);
        target_boxes
            .chain(door_boxes)
            .chain(enemy_boxes)
            .chain(part_boxes)
            .chain(impact_boxes)
            .chain(tracer_boxes)
            .collect()
    }
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
    let (x, y, z) = Quat::from_rotation_arc(Vec3::Z, span / length).to_euler(EulerRot::XYZ);
    Some(LevelBox {
        center: tracer.start + span * 0.5,
        size: Vec3::new(TRACER_THICKNESS, TRACER_THICKNESS, length),
        rotation_degrees: Vec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees()),
        color: TRACER_COLOR,
    })
}

fn lerp_color(from: [f32; 3], to: [f32; 3], amount: f32) -> [f32; 3] {
    [0, 1, 2].map(|channel| from[channel] + (to[channel] - from[channel]) * amount)
}

pub fn muzzle_position(game: &Game) -> Vec3 {
    let player = &game.player;
    let forward = game::look_direction(player.yaw, player.pitch);
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(forward);
    game.eye_position() + right * MUZZLE_OFFSET.x + up * MUZZLE_OFFSET.y + forward * MUZZLE_OFFSET.z
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
