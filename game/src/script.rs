use glam::Vec3;
use parry3d::math::Pose;
use parry3d::query::{Ray, RayCast};
use parry3d::shape::Cuboid;
use serde::Deserialize;

use crate::collision::SKIN;
use crate::motion::player_body;
use crate::{ArenaPhase, Game, GameEvent, Player, Zone};

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum ScriptAction {
    Open(String),
    Close(String),
    Break(String),
    DisableHazard(String),
    Spawn(String),
    Say(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct DoorSpawn {
    pub name: String,
    pub zones: Vec<Zone>,
    pub color: [f32; 3],
}

#[derive(Debug, Clone, Deserialize)]
pub struct KeySpawn {
    pub name: String,
    pub zone: Zone,
    pub color: [f32; 3],
}

#[derive(Debug, Clone, Deserialize)]
pub struct BreakableSpawn {
    pub name: String,
    pub zones: Vec<Zone>,
    pub color: [f32; 3],
    #[serde(default)]
    pub by_slam: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SwitchSpawn {
    pub name: String,
    pub zone: Zone,
    pub color: [f32; 3],
    pub health: f32,
    pub actions: Vec<ScriptAction>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TriggerSpawn {
    pub zone: Zone,
    #[serde(default)]
    pub requires_key: Option<String>,
    pub actions: Vec<ScriptAction>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpawnGroup {
    pub name: String,
    pub positions: Vec<Vec3>,
}

#[derive(Debug, Clone)]
pub struct Door {
    pub name: String,
    pub zones: Vec<Zone>,
    pub color: [f32; 3],
    pub open: bool,
}

impl Door {
    pub(crate) fn from_spawns(spawns: &[DoorSpawn]) -> Vec<Door> {
        spawns
            .iter()
            .map(|spawn| Door {
                name: spawn.name.clone(),
                zones: spawn.zones.clone(),
                color: spawn.color,
                open: false,
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct Key {
    pub name: String,
    pub zone: Zone,
    pub color: [f32; 3],
    pub collected: bool,
}

impl Key {
    pub(crate) fn from_spawns(spawns: &[KeySpawn]) -> Vec<Key> {
        spawns
            .iter()
            .map(|spawn| Key {
                name: spawn.name.clone(),
                zone: spawn.zone.clone(),
                color: spawn.color,
                collected: false,
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct Breakable {
    pub name: String,
    pub zones: Vec<Zone>,
    pub color: [f32; 3],
    pub by_slam: bool,
    pub broken: bool,
}

impl Breakable {
    pub(crate) fn from_spawns(spawns: &[BreakableSpawn]) -> Vec<Breakable> {
        spawns
            .iter()
            .map(|spawn| Breakable {
                name: spawn.name.clone(),
                zones: spawn.zones.clone(),
                color: spawn.color,
                by_slam: spawn.by_slam,
                broken: false,
            })
            .collect()
    }

    fn center(&self) -> Vec3 {
        self.zones.iter().map(|zone| zone.center).sum::<Vec3>() / self.zones.len() as f32
    }
}

#[derive(Debug, Clone)]
pub struct Switch {
    pub name: String,
    pub zone: Zone,
    pub color: [f32; 3],
    pub health: f32,
    pub max_health: f32,
    pub actions: Vec<ScriptAction>,
    pub activated: bool,
}

impl Switch {
    pub(crate) fn from_spawns(spawns: &[SwitchSpawn]) -> Vec<Switch> {
        spawns
            .iter()
            .map(|spawn| Switch {
                name: spawn.name.clone(),
                zone: spawn.zone.clone(),
                color: spawn.color,
                health: spawn.health,
                max_health: spawn.health,
                actions: spawn.actions.clone(),
                activated: false,
            })
            .collect()
    }

    pub fn ray_distance(&self, ray: &Ray, max_distance: f32) -> Option<f32> {
        if self.health <= 0.0 {
            return None;
        }
        Cuboid::new(self.zone.size * 0.5).cast_ray(
            &Pose::from_translation(self.zone.center),
            ray,
            max_distance,
            true,
        )
    }
}

#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub index: usize,
    pub position: Vec3,
    pub yaw: f32,
}

impl Game {
    pub fn has_key(&self, name: &str) -> bool {
        self.keys
            .iter()
            .any(|key| key.collected && key.name == name)
    }

    pub fn script_blockers(&self) -> impl Iterator<Item = &Zone> {
        self.doors
            .iter()
            .filter(|door| !door.open)
            .flat_map(|door| door.zones.iter())
            .chain(
                self.breakables
                    .iter()
                    .filter(|breakable| !breakable.broken)
                    .flat_map(|breakable| breakable.zones.iter()),
            )
            .chain(self.switches.iter().map(|switch| &switch.zone))
    }

    pub(crate) fn update_script(&mut self, slammed: bool) {
        self.collect_keys();
        self.activate_switches();
        if slammed {
            self.break_by_slam();
        }
        self.update_checkpoints();
        self.fire_triggers();
    }

    pub(crate) fn respawn(&mut self) {
        match self.checkpoint.clone() {
            Some(checkpoint) => self.respawn_at(&checkpoint),
            None => self.restart(),
        }
    }

    fn collect_keys(&mut self) {
        if self.keys.is_empty() {
            return;
        }
        let body = player_body(&self.config.movement, self.player.is_low);
        let feet = self.player.position;
        let height = body.half_height * 2.0;
        let reach = self.config.movement.radius * 2.0;
        let mut collected = Vec::new();
        for key in &mut self.keys {
            if key.collected {
                continue;
            }
            let zone = Zone {
                center: key.zone.center,
                size: key.zone.size + Vec3::new(reach, 0.0, reach),
            };
            if zone.reaches(feet, height) {
                key.collected = true;
                collected.push(key.name.clone());
            }
        }
        for name in collected {
            self.events.push(GameEvent::KeyCollected { name });
        }
    }

    fn activate_switches(&mut self) {
        for index in 0..self.switches.len() {
            if self.switches[index].activated || self.switches[index].health > 0.0 {
                continue;
            }
            self.switches[index].activated = true;
            let name = self.switches[index].name.clone();
            let actions = self.switches[index].actions.clone();
            self.events.push(GameEvent::SwitchActivated { name });
            self.run_actions(&actions);
        }
    }

    fn break_by_slam(&mut self) {
        let feet = self.player.position;
        let mut broken = Vec::new();
        for breakable in &mut self.breakables {
            if breakable.broken || !breakable.by_slam {
                continue;
            }
            if breakable.zones.iter().any(|zone| zone.stands_on_top(feet)) {
                breakable.broken = true;
                broken.push((breakable.name.clone(), breakable.center()));
            }
        }
        if broken.is_empty() {
            return;
        }
        for (name, position) in broken {
            self.events.push(GameEvent::Broken { name, position });
        }
        self.rebuild_collision();
    }

    fn update_checkpoints(&mut self) {
        if self.level.checkpoints.is_empty() {
            return;
        }
        let body = player_body(&self.config.movement, self.player.is_low);
        let feet = self.player.position;
        let height = body.half_height * 2.0;
        let yaw = self.player.yaw;
        for index in 0..self.level.checkpoints.len() {
            if self
                .checkpoint
                .as_ref()
                .is_some_and(|checkpoint| checkpoint.index >= index)
            {
                continue;
            }
            let zone = &self.level.checkpoints[index];
            if !zone.reaches(feet, height) {
                continue;
            }
            self.checkpoint = Some(Checkpoint {
                index,
                position: Vec3::new(
                    zone.center.x,
                    zone.center.y - zone.size.y * 0.5,
                    zone.center.z,
                ),
                yaw,
            });
            self.events.push(GameEvent::CheckpointReached { index });
        }
    }

    fn fire_triggers(&mut self) {
        if self.level.triggers.is_empty() {
            return;
        }
        let body = player_body(&self.config.movement, self.player.is_low);
        let feet = self.player.position;
        let height = body.half_height * 2.0;
        for index in 0..self.level.triggers.len() {
            if self.triggers[index] {
                continue;
            }
            let trigger = &self.level.triggers[index];
            if !trigger.zone.reaches(feet, height) {
                continue;
            }
            if let Some(key) = &trigger.requires_key
                && !self.has_key(key)
            {
                continue;
            }
            self.triggers[index] = true;
            let actions = trigger.actions.clone();
            self.run_actions(&actions);
        }
    }

    fn run_actions(&mut self, actions: &[ScriptAction]) {
        let mut collision_changed = false;
        for action in actions {
            match action {
                ScriptAction::Open(name) => collision_changed |= self.set_door(name, true),
                ScriptAction::Close(name) => collision_changed |= self.set_door(name, false),
                ScriptAction::Break(name) => collision_changed |= self.break_group(name),
                ScriptAction::DisableHazard(name) => self.disable_hazard(name),
                ScriptAction::Spawn(name) => self.spawn_group(name),
                ScriptAction::Say(key) => self.events.push(GameEvent::Say { key: key.clone() }),
            }
        }
        if collision_changed {
            self.rebuild_collision();
        }
    }

    fn set_door(&mut self, name: &str, open: bool) -> bool {
        let mut changed = false;
        for door in &mut self.doors {
            if door.name != name || door.open == open {
                continue;
            }
            door.open = open;
            changed = true;
            let event = if open {
                GameEvent::DoorOpened {
                    name: door.name.clone(),
                }
            } else {
                GameEvent::DoorClosed {
                    name: door.name.clone(),
                }
            };
            self.events.push(event);
        }
        changed
    }

    fn break_group(&mut self, name: &str) -> bool {
        let mut broken = None;
        for breakable in &mut self.breakables {
            if breakable.broken || breakable.name != name {
                continue;
            }
            breakable.broken = true;
            broken = Some(breakable.center());
        }
        let Some(position) = broken else {
            return false;
        };
        self.events.push(GameEvent::Broken {
            name: name.to_owned(),
            position,
        });
        true
    }

    fn spawn_group(&mut self, name: &str) {
        let positions = self
            .level
            .spawns
            .iter()
            .find(|group| group.name == name)
            .map(|group| group.positions.clone());
        let Some(positions) = positions else {
            return;
        };
        for position in positions {
            self.spawn_enemy(position);
        }
    }

    fn respawn_at(&mut self, checkpoint: &Checkpoint) {
        self.reset_active_arenas();
        self.player = Player::spawned_at(
            checkpoint.position + Vec3::Y * SKIN,
            checkpoint.yaw,
            self.config.movement.stamina_max,
            self.config.health.max_health,
        );
        self.parts.clear();
        self.weapon.cooldowns = [0.0; 3];
        self.weapon.charge = 0.0;
        self.weapon.beam = None;
        self.projectiles.clear();
        self.restart_countdown_seconds = None;
        self.rebuild_collision();
    }

    fn reset_active_arenas(&mut self) {
        let mut removed = Vec::new();
        for arena in &mut self.arenas {
            if arena.phase != ArenaPhase::Active {
                continue;
            }
            arena.phase = ArenaPhase::Waiting;
            arena.wave = 0;
            removed.append(&mut arena.wave_enemy_ids);
            removed.append(&mut arena.machine_enemy_ids);
            arena.machine_timer_seconds = 0.0;
            if let Some(id) = arena.machine_target_id.take() {
                self.targets.retain(|target| target.id != id);
            }
        }
        self.enemies.retain(|enemy| !removed.contains(&enemy.id));
    }
}
