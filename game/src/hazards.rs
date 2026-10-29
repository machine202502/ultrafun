use glam::{Vec2, Vec3};
use serde::Deserialize;

use crate::collision::SKIN;
use crate::motion::{self, BodyState, player_body};
use crate::player::{MoveState, flat_direction};
use crate::{Game, GameEvent, STEP_SECONDS, Zone};

pub const DEFAULT_HAZARD_DAMAGE_PER_SECOND: f32 = 1000.0;

const CRUSHER_DOWN_END: f32 = 0.15;
const CRUSHER_BOTTOM_END: f32 = 0.25;
const CRUSHER_UP_END: f32 = 0.60;

#[derive(Debug, Clone, Deserialize)]
pub struct HazardSpawn {
    pub name: String,
    pub zone: Zone,
    pub color: [f32; 3],
    #[serde(default = "default_damage_per_second")]
    pub damage_per_second: f32,
}

fn default_damage_per_second() -> f32 {
    DEFAULT_HAZARD_DAMAGE_PER_SECOND
}

#[derive(Debug, Clone)]
pub struct Hazard {
    pub name: String,
    pub zone: Zone,
    pub color: [f32; 3],
    pub damage_per_second: f32,
    pub enabled: bool,
}

impl Hazard {
    pub fn from_spawns(spawns: &[HazardSpawn]) -> Vec<Hazard> {
        spawns
            .iter()
            .map(|spawn| Hazard {
                name: spawn.name.clone(),
                zone: spawn.zone.clone(),
                color: spawn.color,
                damage_per_second: spawn.damage_per_second,
                enabled: true,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PadSpawn {
    pub zone: Zone,
    pub launch_speed: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BeltSpawn {
    pub zone: Zone,
    pub velocity: Vec3,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CrusherSpawn {
    pub center: Vec3,
    pub size: Vec3,
    pub color: [f32; 3],
    pub travel: f32,
    pub period_seconds: f32,
    pub offset_seconds: f32,
}

impl CrusherSpawn {
    pub fn center_at(&self, seconds: f32) -> Vec3 {
        self.center - Vec3::Y * self.travel * self.down_fraction(seconds)
    }

    fn is_lethal(&self, seconds: f32) -> bool {
        self.phase(seconds) < CRUSHER_BOTTOM_END
    }

    fn slams_at(&self, seconds: f32) -> bool {
        self.phase(seconds) >= CRUSHER_DOWN_END
            && self.phase(seconds - STEP_SECONDS) < CRUSHER_DOWN_END
    }

    fn zone_at(&self, seconds: f32) -> Zone {
        Zone {
            center: self.center_at(seconds),
            size: self.size,
        }
    }

    fn down_fraction(&self, seconds: f32) -> f32 {
        let phase = self.phase(seconds);
        if phase < CRUSHER_DOWN_END {
            phase / CRUSHER_DOWN_END
        } else if phase < CRUSHER_BOTTOM_END {
            1.0
        } else if phase < CRUSHER_UP_END {
            1.0 - (phase - CRUSHER_BOTTOM_END) / (CRUSHER_UP_END - CRUSHER_BOTTOM_END)
        } else {
            0.0
        }
    }

    fn phase(&self, seconds: f32) -> f32 {
        (seconds + self.offset_seconds).rem_euclid(self.period_seconds) / self.period_seconds
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct TubeSpawn {
    pub zone: Zone,
    pub exit: Vec3,
    pub exit_yaw_degrees: f32,
    pub exit_speed: f32,
}

impl Game {
    pub fn disable_hazard(&mut self, name: &str) {
        for hazard in &mut self.hazards {
            if hazard.name == name {
                hazard.enabled = false;
            }
        }
    }

    pub(crate) fn update_transport(&mut self) {
        self.launch_from_pads();
        self.carry_player_on_belts();
        self.travel_tubes();
    }

    pub(crate) fn carry_enemies_on_belts(&mut self) {
        if self.level.belts.is_empty() {
            return;
        }
        let body = self.config.grunt.body();
        for enemy in &mut self.enemies {
            let motion = belt_motion(&self.level.belts, enemy.position);
            if motion == Vec3::ZERO {
                continue;
            }
            let state = BodyState {
                position: &mut enemy.position,
                velocity: &mut enemy.velocity,
                grounded: &mut enemy.grounded,
            };
            motion::carry_body(&body, state, &self.collision, motion);
        }
    }

    pub(crate) fn update_traps(&mut self) {
        if self.player.health <= 0.0 {
            return;
        }
        self.apply_hazards();
        let seconds = self.elapsed_seconds();
        self.apply_crushers(seconds);
    }

    fn apply_hazards(&mut self) {
        if self.hazards.is_empty() {
            return;
        }
        let body = player_body(&self.config.movement, self.player.is_low);
        let center = body.center(self.player.position);
        let damage_per_second: f32 = self
            .hazards
            .iter()
            .filter(|hazard| hazard.enabled && hazard.zone.contains(center))
            .map(|hazard| hazard.damage_per_second)
            .sum();
        if damage_per_second > 0.0 {
            hurt_player(self, damage_per_second * STEP_SECONDS);
        }
        self.burn_enemies();
    }

    fn burn_enemies(&mut self) {
        let hazards = &self.hazards;
        let height = self.config.grunt.height;
        let mut killed = Vec::new();
        for (index, enemy) in self.enemies.iter_mut().enumerate() {
            let center = enemy.position + Vec3::Y * height * 0.5;
            let damage_per_second: f32 = hazards
                .iter()
                .filter(|hazard| hazard.enabled && hazard.zone.contains(center))
                .map(|hazard| hazard.damage_per_second)
                .sum();
            if damage_per_second <= 0.0 {
                continue;
            }
            enemy.health -= damage_per_second * STEP_SECONDS;
            if enemy.health <= 0.0 {
                killed.push((index, center, enemy.id));
            }
        }
        for (index, center, id) in killed.into_iter().rev() {
            self.enemies.remove(index);
            crate::parts::spawn_parts(center, id, &self.config.health, &mut self.parts);
            self.kills += 1;
            self.events.push(GameEvent::EnemyKilled {
                id,
                position: center,
            });
        }
    }

    fn apply_crushers(&mut self, seconds: f32) {
        if self.level.crushers.is_empty() {
            return;
        }
        let body = player_body(&self.config.movement, self.player.is_low);
        let feet = self.player.position;
        let height = body.half_height * 2.0;
        let crushed = self.level.crushers.iter().any(|crusher| {
            crusher.is_lethal(seconds) && crusher.zone_at(seconds).reaches(feet, height)
        });
        if crushed {
            let amount = self.player.health;
            hurt_player(self, amount);
        }
        let slams: Vec<Vec3> = self
            .level
            .crushers
            .iter()
            .filter(|crusher| crusher.slams_at(seconds))
            .map(|crusher| crusher.center_at(seconds))
            .collect();
        for position in slams {
            self.events.push(GameEvent::CrusherSlammed { position });
        }
    }

    fn launch_from_pads(&mut self) {
        if self.level.pads.is_empty() || self.player.velocity.y > 0.0 {
            return;
        }
        let body = player_body(&self.config.movement, self.player.is_low);
        let feet = self.player.position;
        let height = body.half_height * 2.0;
        let launch_speed = self
            .level
            .pads
            .iter()
            .filter(|pad| pad.zone.reaches(feet, height))
            .map(|pad| pad.launch_speed)
            .reduce(f32::max);
        let Some(launch_speed) = launch_speed else {
            return;
        };
        self.player.velocity.y = launch_speed;
        self.player.grounded = false;
        self.player.wall_jumps_used = 0;
        self.player.state = MoveState::Normal;
        self.events.push(GameEvent::Launched { position: feet });
    }

    fn carry_player_on_belts(&mut self) {
        if !self.player.grounded {
            return;
        }
        let motion = belt_motion(&self.level.belts, self.player.position);
        if motion == Vec3::ZERO {
            return;
        }
        motion::carry_player(
            &mut self.player,
            &self.config.movement,
            &self.collision,
            motion,
        );
    }

    fn travel_tubes(&mut self) {
        if self.level.tubes.is_empty() {
            return;
        }
        let body = player_body(&self.config.movement, self.player.is_low);
        let feet = self.player.position;
        let height = body.half_height * 2.0;
        let Some((exit, yaw_degrees, speed)) = self
            .level
            .tubes
            .iter()
            .find(|tube| tube.zone.reaches(feet, height))
            .map(|tube| (tube.exit, tube.exit_yaw_degrees, tube.exit_speed))
        else {
            return;
        };
        let from = feet;
        let yaw = yaw_degrees.to_radians();
        self.player.position = exit + Vec3::Y * SKIN;
        self.player.previous_position = self.player.position;
        self.player.velocity = flat_direction(yaw, Vec2::Y) * speed;
        self.player.yaw = yaw;
        self.player.grounded = false;
        self.player.state = MoveState::Normal;
        self.events.push(GameEvent::TubeTravelled {
            from,
            to: self.player.position,
        });
    }
}

fn belt_motion(belts: &[BeltSpawn], feet: Vec3) -> Vec3 {
    belts
        .iter()
        .filter(|belt| belt.zone.stands_on_top(feet))
        .fold(Vec3::ZERO, |motion, belt| {
            motion + belt.velocity * STEP_SECONDS
        })
}

fn hurt_player(game: &mut Game, amount: f32) {
    let amount = amount.min(game.player.health);
    if amount <= 0.0 {
        return;
    }
    game.player.health -= amount;
    game.events.push(GameEvent::PlayerDamaged { amount });
}
