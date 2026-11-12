use glam::Vec3;
use parry3d::query::Ray;

use crate::motion::{BodyState, carry_body};
use crate::weapon::{ShotHit, trace_shot};
use crate::{Actions, FireConfig, FireMode, Game, GameEvent, Projectile, STEP_SECONDS, WeaponKind};

impl Game {
    pub fn charge_fraction(&self) -> f32 {
        (self.weapon.charge / self.config.weapon.charge_seconds.max(STEP_SECONDS)).clamp(0.0, 1.0)
    }

    pub fn ready_fraction(&self) -> f32 {
        let cooldown = self
            .config
            .weapon
            .fire(self.weapon.selected, FireMode::Secondary)
            .cooldown_seconds;
        (1.0 - self.weapon.cooldowns[self.weapon.selected.index()] / cooldown.max(STEP_SECONDS))
            .clamp(0.0, 1.0)
    }

    pub(crate) fn update_weapon(&mut self, actions: &Actions) {
        for cooldown in &mut self.weapon.cooldowns {
            *cooldown = (*cooldown - STEP_SECONDS).max(0.0);
        }
        let was_beam = self.weapon.beam.is_some();
        self.weapon.beam = None;
        if let Some(selected) = actions.select_weapon
            && selected != self.weapon.selected
        {
            self.weapon.selected = selected;
            self.weapon.charge = 0.0;
        }
        self.update_projectiles();
        if actions.cancel_attacks {
            self.weapon.charge = 0.0;
            return;
        }
        let weapon = self.weapon.selected;
        let mode = if actions.alt_held || actions.alt_released {
            FireMode::Secondary
        } else {
            FireMode::Primary
        };
        let config = self.config.weapon.fire(weapon, mode);
        let origin = self.eye_position();
        let direction = crate::look_direction(self.player.yaw, self.player.pitch);
        if self.weapon.cooldowns[weapon.index()] > 0.0 {
            if weapon == WeaponKind::PlasmaCutter
                && mode == FireMode::Primary
                && actions.fire_held
                && was_beam
            {
                let hit = trace_shot(
                    &Ray::new(origin, direction),
                    config.range,
                    &self.collision,
                    &self.targets,
                    &self.switches,
                    &self.enemies,
                    &self.config.grunt,
                );
                self.weapon.beam = Some((origin, origin + direction * hit.distance(config.range)));
            }
            return;
        }
        if weapon == WeaponKind::PlasmaCutter && mode == FireMode::Secondary {
            if actions.alt_held {
                self.weapon.charge =
                    (self.weapon.charge + STEP_SECONDS).min(self.config.weapon.charge_seconds);
                return;
            }
            if actions.alt_released {
                let charge = self.charge_fraction();
                self.weapon.charge = 0.0;
                if charge >= self.config.weapon.min_charge_fraction {
                    let mut charged = config;
                    charged.damage *= charge;
                    let end = self.piercing_shot(origin, direction, charged);
                    self.shot_event(origin, end, mode);
                    self.weapon.cooldowns[weapon.index()] = config.cooldown_seconds;
                }
            }
            return;
        }
        self.weapon.charge = 0.0;
        if !actions.fire_held && !actions.alt_held {
            return;
        }
        let mut end = origin + direction * config.range;
        match (weapon, mode) {
            (WeaponKind::RivetGun, FireMode::Secondary) => {
                self.projectiles.push(Projectile {
                    position: origin,
                    previous_position: origin,
                    velocity: direction * self.config.weapon.projectile_speed,
                    lifetime: self.config.weapon.projectile_lifetime,
                    damage: config.damage,
                    impulse: config.impulse,
                });
            }
            (WeaponKind::Sandblaster, FireMode::Primary) => {
                let right = direction.cross(Vec3::Y).normalize_or_zero();
                let up = right.cross(direction).normalize_or_zero();
                let count = self.config.weapon.pellets.clamp(1, 32);
                let spread = self.config.weapon.spread_degrees.to_radians().tan();
                for index in 0..count {
                    let radius = (index as f32 / count as f32).sqrt() * spread;
                    let angle = index as f32 * 2.399_963_1;
                    let ray_direction =
                        (direction + right * angle.cos() * radius + up * angle.sin() * radius)
                            .normalize();
                    let hit = self.fire_ray(origin, ray_direction, config);
                    if index == 0 {
                        end = hit;
                    }
                }
                self.player.velocity -= direction * self.config.weapon.blast_recoil * 0.15;
            }
            (WeaponKind::Sandblaster, FireMode::Secondary) => {
                self.air_blast(origin, direction, config);
                self.player.velocity -= direction * self.config.weapon.blast_recoil;
            }
            (WeaponKind::PlasmaCutter, FireMode::Primary) => {
                end = self.fire_ray(origin, direction, config);
                self.weapon.beam = Some((origin, end));
            }
            _ => end = self.fire_ray(origin, direction, config),
        }
        self.weapon.cooldowns[weapon.index()] = config.cooldown_seconds;
        if weapon != WeaponKind::PlasmaCutter || !was_beam {
            self.shot_event(origin, end, mode);
        }
    }

    fn shot_event(&mut self, origin: Vec3, end: Vec3, mode: FireMode) {
        self.events.push(GameEvent::ShotFired {
            origin,
            end,
            weapon: self.weapon.selected,
            mode,
        });
    }

    fn fire_ray(&mut self, origin: Vec3, direction: Vec3, config: FireConfig) -> Vec3 {
        let hit = trace_shot(
            &Ray::new(origin, direction),
            config.range,
            &self.collision,
            &self.targets,
            &self.switches,
            &self.enemies,
            &self.config.grunt,
        );
        let end = origin + direction * hit.distance(config.range);
        self.apply_hit(hit, end, direction, config, false);
        end
    }

    fn apply_hit(
        &mut self,
        hit: ShotHit,
        position: Vec3,
        direction: Vec3,
        config: FireConfig,
        pin: bool,
    ) {
        match hit {
            ShotHit::Nothing => {}
            ShotHit::Wall { normal, .. } => {
                self.events.push(GameEvent::ShotImpact { position, normal })
            }
            ShotHit::Target { index, .. } => {
                self.targets[index].health -= config.damage;
                let id = self.targets[index].id;
                self.events.push(GameEvent::TargetHit { id });
                if self.targets[index].health <= 0.0 {
                    self.targets.remove(index);
                    self.events
                        .push(GameEvent::TargetDestroyed { id, position });
                }
            }
            ShotHit::Switch { index, .. } => self.switches[index].health -= config.damage,
            ShotHit::Enemy { index, .. } => {
                let id = self.enemies[index].id;
                self.damage_enemy(index, position, direction, config);
                if pin && let Some(enemy) = self.enemies.iter_mut().find(|enemy| enemy.id == id) {
                    let center = enemy.position + Vec3::Y * self.config.grunt.height * 0.5;
                    if self
                        .collision
                        .ray(&Ray::new(center, direction), self.config.weapon.pin_range)
                        .is_some_and(|(_, normal)| normal.y.abs() < 0.5)
                    {
                        let body = self.config.grunt.body();
                        carry_body(
                            &body,
                            BodyState {
                                position: &mut enemy.position,
                                velocity: &mut enemy.velocity,
                                grounded: &mut enemy.grounded,
                            },
                            &self.collision,
                            direction * self.config.weapon.pin_range,
                        );
                        enemy.previous_position = enemy.position;
                        enemy.pinned_seconds = self.config.weapon.pin_seconds;
                    }
                }
            }
        }
    }

    fn damage_enemy(&mut self, index: usize, position: Vec3, direction: Vec3, config: FireConfig) {
        let enemy = &mut self.enemies[index];
        enemy.health -= config.damage;
        enemy.knockback += direction * config.impulse + Vec3::Y * config.impulse * 0.05;
        self.events.push(GameEvent::EnemyHit {
            id: enemy.id,
            position,
            normal: -direction,
        });
        if enemy.health <= 0.0 {
            self.finish_enemy(index, true);
        }
    }

    pub(crate) fn finish_enemy(&mut self, index: usize, drop_parts: bool) {
        let enemy = self.enemies.remove(index);
        let position = enemy.position + Vec3::Y * self.config.grunt.height * 0.5;
        if drop_parts {
            crate::parts::spawn_parts(position, enemy.id, &self.config.health, &mut self.parts);
        }
        self.kills += 1;
        self.events.push(GameEvent::EnemyKilled {
            id: enemy.id,
            position,
        });
    }

    fn piercing_shot(&mut self, origin: Vec3, direction: Vec3, config: FireConfig) -> Vec3 {
        let ray = Ray::new(origin, direction);
        let wall = self.collision.ray(&ray, config.range);
        let range = wall.map_or(config.range, |(distance, _)| distance);
        let victims: Vec<_> = self
            .enemies
            .iter()
            .filter_map(|enemy| {
                enemy
                    .ray_distance(&ray, range, &self.config.grunt)
                    .map(|distance| (enemy.id, distance))
            })
            .collect();
        for (id, distance) in victims {
            if let Some(index) = self.enemies.iter().position(|enemy| enemy.id == id) {
                self.damage_enemy(index, origin + direction * distance, direction, config);
            }
        }
        let hit = trace_shot(
            &ray,
            range,
            &self.collision,
            &self.targets,
            &self.switches,
            &[],
            &self.config.grunt,
        );
        let hit_distance = hit.distance(range);
        self.apply_hit(
            hit,
            origin + direction * hit_distance,
            direction,
            config,
            false,
        );
        origin + direction * range
    }

    fn air_blast(&mut self, origin: Vec3, direction: Vec3, config: FireConfig) {
        let cone = self.config.weapon.blast_cone_degrees.to_radians().cos();
        let ids: Vec<_> = self
            .enemies
            .iter()
            .filter(|enemy| {
                let delta = enemy.position + Vec3::Y - origin;
                delta.length() <= config.range
                    && delta.normalize_or_zero().dot(direction) >= cone
                    && self
                        .collision
                        .line_of_sight(origin, enemy.position + Vec3::Y)
            })
            .map(|enemy| enemy.id)
            .collect();
        for id in ids {
            if let Some(index) = self.enemies.iter().position(|enemy| enemy.id == id) {
                let position = self.enemies[index].position + Vec3::Y;
                self.damage_enemy(index, position, direction, config);
            }
        }
        for projectile in &mut self.projectiles {
            let delta = projectile.position - origin;
            if delta.length() < config.range
                && delta.normalize_or_zero().dot(direction) >= cone
                && self.collision.line_of_sight(origin, projectile.position)
            {
                projectile.velocity = direction * self.config.weapon.projectile_speed * 1.5;
                projectile.damage *= 1.5;
            }
        }
    }

    fn update_projectiles(&mut self) {
        let projectiles = std::mem::take(&mut self.projectiles);
        for mut projectile in projectiles {
            projectile.previous_position = projectile.position;
            let motion = projectile.velocity * STEP_SECONDS;
            let distance = motion.length();
            if distance < 1e-6 {
                continue;
            }
            let direction = motion / distance;
            let hit = trace_shot(
                &Ray::new(projectile.position, direction),
                distance,
                &self.collision,
                &self.targets,
                &self.switches,
                &self.enemies,
                &self.config.grunt,
            );
            if !matches!(hit, ShotHit::Nothing) {
                let position = projectile.position + direction * hit.distance(distance);
                self.apply_hit(
                    hit,
                    position,
                    direction,
                    FireConfig {
                        damage: projectile.damage,
                        impulse: projectile.impulse,
                        range: distance,
                        cooldown_seconds: 0.0,
                    },
                    true,
                );
            } else {
                projectile.position += motion;
                projectile.lifetime -= STEP_SECONDS;
                if projectile.lifetime > 0.0 {
                    self.projectiles.push(projectile);
                }
            }
        }
    }
}
