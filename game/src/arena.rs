use glam::Vec3;
use serde::Deserialize;

use crate::collision::CollisionWorld;
use crate::{Enemy, Game, GameEvent, STEP_SECONDS, Target, TargetKind};

const MACHINE_TARGET_ID_BASE: u32 = 10_000;
const STAND_ON_TOP_TOLERANCE: f32 = 0.05;

#[derive(Debug, Clone, Deserialize)]
pub struct Zone {
    pub center: Vec3,
    pub size: Vec3,
}

impl Zone {
    pub fn contains(&self, point: Vec3) -> bool {
        ((point - self.center).abs() - self.size * 0.5).max_element() <= 0.0
    }

    pub fn reaches(&self, feet: Vec3, height: f32) -> bool {
        let y = self.center.y.clamp(feet.y, feet.y + height);
        self.contains(Vec3::new(feet.x, y, feet.z))
    }

    pub fn stands_on_top(&self, feet: Vec3) -> bool {
        let top = self.center.y + self.size.y * 0.5;
        (feet.y - top).abs() <= STAND_ON_TOP_TOLERANCE
            && self.contains(Vec3::new(feet.x, top, feet.z))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArenaSpawn {
    pub trigger: Zone,
    pub doors: Vec<Zone>,
    #[serde(default)]
    pub waves: Vec<Vec<Vec3>>,
    #[serde(default)]
    pub machine: Option<MachineSpawn>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MachineSpawn {
    pub center: Vec3,
    pub size: Vec3,
    pub health: f32,
    pub spawn_point: Vec3,
    pub spawn_interval_seconds: f32,
    pub max_alive: usize,
    #[serde(default)]
    pub say: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArenaPhase {
    Waiting,
    Active,
    Cleared,
}

#[derive(Debug, Clone)]
pub struct Arena {
    pub phase: ArenaPhase,
    pub wave: usize,
    pub wave_enemy_ids: Vec<u32>,
    pub machine_enemy_ids: Vec<u32>,
    pub machine_target_id: Option<u32>,
    pub machine_timer_seconds: f32,
    pub machine_announced: bool,
}

impl Arena {
    pub fn waiting(count: usize) -> Vec<Arena> {
        (0..count)
            .map(|_| Arena {
                phase: ArenaPhase::Waiting,
                wave: 0,
                wave_enemy_ids: Vec::new(),
                machine_enemy_ids: Vec::new(),
                machine_target_id: None,
                machine_timer_seconds: 0.0,
                machine_announced: false,
            })
            .collect()
    }
}

impl Game {
    pub fn closed_doors(&self) -> impl Iterator<Item = &Zone> {
        self.arenas
            .iter()
            .zip(&self.level.arenas)
            .filter(|(arena, _)| arena.phase == ArenaPhase::Active)
            .flat_map(|(_, spawn)| &spawn.doors)
    }

    pub(crate) fn rebuild_collision(&mut self) {
        self.collision = CollisionWorld::new(
            &self.level,
            self.closed_doors().chain(self.script_blockers()),
        );
    }

    pub(crate) fn update_arenas(&mut self) {
        let mut doors_changed = false;
        for index in 0..self.arenas.len() {
            let spawn = self.level.arenas[index].clone();
            match self.arenas[index].phase {
                ArenaPhase::Waiting if spawn.trigger.contains(self.player.position) => {
                    self.start_arena(index, &spawn);
                    doors_changed = true;
                }
                ArenaPhase::Active => doors_changed |= self.progress_arena(index, &spawn),
                ArenaPhase::Waiting | ArenaPhase::Cleared => {}
            }
        }
        if doors_changed {
            self.rebuild_collision();
        }
    }

    fn start_arena(&mut self, index: usize, spawn: &ArenaSpawn) {
        self.arenas[index].phase = ArenaPhase::Active;
        self.events.push(GameEvent::ArenaStarted { arena: index });
        self.start_wave(index, spawn, 0);
        if let Some(machine) = &spawn.machine {
            let id = MACHINE_TARGET_ID_BASE + index as u32;
            self.targets.push(Target {
                id,
                center: machine.center,
                size: machine.size,
                max_health: machine.health,
                health: machine.health,
                kind: TargetKind::Machine,
            });
            let arena = &mut self.arenas[index];
            arena.machine_target_id = Some(id);
            arena.machine_timer_seconds = machine.spawn_interval_seconds;
        }
    }

    fn start_wave(&mut self, index: usize, spawn: &ArenaSpawn, wave: usize) {
        let Some(positions) = spawn.waves.get(wave) else {
            return;
        };
        let ids = positions
            .iter()
            .map(|position| self.spawn_enemy(*position))
            .collect();
        let arena = &mut self.arenas[index];
        arena.wave = wave;
        arena.wave_enemy_ids = ids;
        self.events
            .push(GameEvent::WaveStarted { arena: index, wave });
    }

    fn progress_arena(&mut self, index: usize, spawn: &ArenaSpawn) -> bool {
        let enemies = &self.enemies;
        let is_alive = |id: &u32| enemies.iter().any(|enemy| enemy.id == *id);
        let arena = &mut self.arenas[index];
        arena.wave_enemy_ids.retain(is_alive);
        arena.machine_enemy_ids.retain(is_alive);
        let machine_alive = arena
            .machine_target_id
            .is_some_and(|id| self.targets.iter().any(|target| target.id == id));

        if let (true, Some(machine)) = (machine_alive, &spawn.machine) {
            let arena = &mut self.arenas[index];
            arena.machine_timer_seconds -= STEP_SECONDS;
            if arena.machine_timer_seconds <= 0.0 {
                arena.machine_timer_seconds = machine.spawn_interval_seconds;
                if arena.machine_enemy_ids.len() < machine.max_alive {
                    let id = self.spawn_enemy(machine.spawn_point);
                    self.arenas[index].machine_enemy_ids.push(id);
                }
            }
        }

        if !self.arenas[index].wave_enemy_ids.is_empty() {
            return false;
        }
        let last_wave = self.arenas[index].wave + 1 >= spawn.waves.len();
        if last_wave && machine_alive && !self.arenas[index].machine_announced {
            if let Some(say) = spawn
                .machine
                .as_ref()
                .and_then(|machine| machine.say.clone())
            {
                self.events.push(GameEvent::Say { key: say });
            }
            self.arenas[index].machine_announced = true;
        }
        let next_wave = self.arenas[index].wave + 1;
        if next_wave < spawn.waves.len() {
            self.start_wave(index, spawn, next_wave);
            return false;
        }
        if machine_alive || !self.arenas[index].machine_enemy_ids.is_empty() {
            return false;
        }
        self.arenas[index].phase = ArenaPhase::Cleared;
        self.events.push(GameEvent::ArenaCleared { arena: index });
        true
    }

    pub(crate) fn spawn_enemy(&mut self, position: Vec3) -> u32 {
        let id = self.next_enemy_id;
        self.next_enemy_id += 1;
        self.enemies
            .push(Enemy::new(id, position, &self.config.grunt));
        id
    }
}
