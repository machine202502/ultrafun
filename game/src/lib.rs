mod arena;
mod collision;
mod config;
mod enemy;
mod hazards;
mod level;
mod motion;
mod movement;
mod parts;
mod player;
mod script;
mod weapon;

pub use arena::{Arena, ArenaPhase, ArenaSpawn, MachineSpawn, Zone};
pub use config::{HealthConfig, MovementConfig};
pub use enemy::{AttackState, Enemy, GruntConfig};
pub use hazards::{
    BeltSpawn, CrusherSpawn, DEFAULT_HAZARD_DAMAGE_PER_SECOND, Hazard, HazardSpawn, PadSpawn,
    TubeSpawn,
};
pub use level::{EnemySpawn, Level, LevelBox, TargetSpawn};
pub use parts::RepairPart;
pub use player::{MoveState, Player, look_direction};
pub use script::{
    Breakable, BreakableSpawn, Checkpoint, Door, DoorSpawn, Key, KeySpawn, ScriptAction,
    SpawnGroup, Switch, SwitchSpawn, TriggerSpawn,
};
pub use weapon::{Target, TargetKind, WeaponConfig};

use collision::CollisionWorld;
use glam::{Vec2, Vec3};
use parry3d::query::Ray;
use weapon::ShotHit;

pub const STEP_SECONDS: f32 = 1.0 / 120.0;
pub const CHEAT_FLY_SPEED: f32 = 24.0;
const ENEMY_KNOCKBACK: f32 = 8.0;

#[derive(Debug, Clone, Copy, Default)]
pub struct Actions {
    pub look: Vec2,
    pub move_input: Vec2,
    pub jump_pressed: bool,
    pub dash_pressed: bool,
    pub slide_pressed: bool,
    pub slide_held: bool,
    pub fire_held: bool,
    pub respawn: bool,
    pub toggle_cheats: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameEvent {
    Slammed {
        position: Vec3,
        fall_height: f32,
    },
    ShotFired {
        origin: Vec3,
        end: Vec3,
    },
    ShotImpact {
        position: Vec3,
        normal: Vec3,
    },
    TargetHit {
        id: u32,
    },
    TargetDestroyed {
        id: u32,
        position: Vec3,
    },
    EnemyHit {
        id: u32,
        position: Vec3,
        normal: Vec3,
    },
    EnemyKilled {
        id: u32,
        position: Vec3,
    },
    PlayerDamaged {
        amount: f32,
    },
    PlayerDied,
    Jumped,
    Dashed,
    PartCollected {
        heal: f32,
    },
    ArenaStarted {
        arena: usize,
    },
    WaveStarted {
        arena: usize,
        wave: usize,
    },
    ArenaCleared {
        arena: usize,
    },
    Launched {
        position: Vec3,
    },
    CrusherSlammed {
        position: Vec3,
    },
    TubeTravelled {
        from: Vec3,
        to: Vec3,
    },
    DoorOpened {
        name: String,
    },
    DoorClosed {
        name: String,
    },
    KeyCollected {
        name: String,
    },
    Broken {
        name: String,
        position: Vec3,
    },
    SwitchActivated {
        name: String,
    },
    Say {
        key: String,
    },
    CheckpointReached {
        index: usize,
    },
    LevelStarted,
    LevelCompleted,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelResult {
    pub seconds: f32,
    pub kills: u32,
}

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub movement: MovementConfig,
    pub weapon: WeaponConfig,
    pub health: HealthConfig,
    pub grunt: GruntConfig,
}

pub struct Game {
    pub config: GameConfig,
    pub player: Player,
    pub targets: Vec<Target>,
    pub enemies: Vec<Enemy>,
    pub parts: Vec<RepairPart>,
    pub arenas: Vec<Arena>,
    pub hazards: Vec<Hazard>,
    pub doors: Vec<Door>,
    pub keys: Vec<Key>,
    pub breakables: Vec<Breakable>,
    pub switches: Vec<Switch>,
    pub triggers: Vec<bool>,
    pub checkpoint: Option<Checkpoint>,
    pub events: Vec<GameEvent>,
    pub restart_countdown_seconds: Option<f32>,
    pub result: Option<LevelResult>,
    pub cheats: bool,
    weapon_cooldown_seconds: f32,
    next_enemy_id: u32,
    elapsed_steps: u32,
    kills: u32,
    level: Level,
    collision: CollisionWorld,
    void_y: f32,
}

impl Game {
    pub fn new(level: Level, config: GameConfig) -> Self {
        let collision = CollisionWorld::new(&level, std::iter::empty());
        let lowest = level
            .boxes
            .iter()
            .map(|box_| box_.center.y - box_.size.y * 0.5)
            .fold(f32::MAX, f32::min);
        let void_y = if lowest == f32::MAX {
            -30.0
        } else {
            lowest - 20.0
        };
        let mut game = Self {
            player: level.spawn_player(&config.movement, config.health.max_health),
            targets: Vec::new(),
            enemies: Vec::new(),
            parts: Vec::new(),
            arenas: Vec::new(),
            hazards: Vec::new(),
            doors: Vec::new(),
            keys: Vec::new(),
            breakables: Vec::new(),
            switches: Vec::new(),
            triggers: Vec::new(),
            checkpoint: None,
            events: Vec::new(),
            restart_countdown_seconds: None,
            weapon_cooldown_seconds: 0.0,
            next_enemy_id: 0,
            result: None,
            cheats: false,
            elapsed_steps: 0,
            kills: 0,
            config,
            level,
            collision,
            void_y,
        };
        game.restart();
        game
    }

    pub fn level(&self) -> &Level {
        &self.level
    }

    pub fn set_level(&mut self, level: Level) {
        self.level = level;
        self.restart();
    }

    pub fn restart(&mut self) {
        self.player = self
            .level
            .spawn_player(&self.config.movement, self.config.health.max_health);
        self.targets = Target::from_spawns(&self.level.targets);
        self.enemies = Enemy::from_spawns(&self.level.enemies, &self.config.grunt);
        self.next_enemy_id = self.enemies.len() as u32;
        self.parts.clear();
        self.arenas = Arena::waiting(self.level.arenas.len());
        self.hazards = Hazard::from_spawns(&self.level.hazards);
        self.doors = Door::from_spawns(&self.level.doors);
        self.keys = Key::from_spawns(&self.level.keys);
        self.breakables = Breakable::from_spawns(&self.level.breakables);
        self.switches = Switch::from_spawns(&self.level.switches);
        self.triggers = vec![false; self.level.triggers.len()];
        self.checkpoint = None;
        self.restart_countdown_seconds = None;
        self.result = None;
        self.elapsed_steps = 0;
        self.kills = 0;
        self.events.push(GameEvent::LevelStarted);
        self.weapon_cooldown_seconds = 0.0;
        self.rebuild_collision();
    }

    pub fn update(&mut self, actions: &Actions) {
        if actions.toggle_cheats {
            self.cheats = !self.cheats;
            self.player.is_low = false;
            self.player.state = player::MoveState::Normal;
            self.player.velocity = Vec3::ZERO;
        }
        if actions.respawn {
            self.respawn();
            return;
        }
        if self.result.is_some() {
            return;
        }
        if let Some(remaining) = self.restart_countdown_seconds {
            let remaining = remaining - STEP_SECONDS;
            self.restart_countdown_seconds = Some(remaining);
            if remaining <= 0.0 {
                self.respawn();
            }
            return;
        }

        let first_event = self.events.len();
        if self.cheats {
            self.fly(actions);
        } else {
            movement::step_player(
                &mut self.player,
                actions,
                &self.config.movement,
                &self.collision,
                &mut self.events,
            );
        }
        self.update_transport();
        if !self.cheats {
            self.kill_fallen();
        }
        enemy::update_enemies(
            &mut self.enemies,
            &mut self.player,
            &self.config.grunt,
            &self.collision,
            &mut self.events,
            STEP_SECONDS,
        );
        self.carry_enemies_on_belts();
        self.update_weapon(actions);
        let collect_point =
            self.player.position + Vec3::Y * self.player.eye_height(&self.config.movement) * 0.5;
        parts::update_parts(
            &mut self.parts,
            &mut self.player,
            collect_point,
            &self.config.health,
            &self.collision,
            &mut self.events,
            STEP_SECONDS,
        );

        let slammed = self.events[first_event..]
            .iter()
            .any(|event| matches!(event, GameEvent::Slammed { .. }));
        self.update_script(slammed);
        self.update_traps();
        self.update_arenas();
        self.elapsed_steps += 1;
        self.check_exit();

        if self.player.health <= 0.0 {
            self.events.push(GameEvent::PlayerDied);
            self.restart_countdown_seconds = Some(self.config.health.restart_delay_seconds);
        }
    }

    pub fn eye_position(&self) -> Vec3 {
        self.player.position + Vec3::Y * self.player.eye_height(&self.config.movement)
    }

    fn kill_fallen(&mut self) {
        if self.player.position.y < self.void_y && self.player.health > 0.0 {
            let amount = self.player.health;
            self.player.health = 0.0;
            self.events.push(GameEvent::PlayerDamaged { amount });
        }
        let mut fallen = Vec::new();
        for (index, enemy) in self.enemies.iter_mut().enumerate() {
            if enemy.position.y < self.void_y {
                fallen.push(index);
            }
        }
        for index in fallen.into_iter().rev() {
            let enemy = self.enemies.remove(index);
            self.events.push(GameEvent::EnemyKilled {
                id: enemy.id,
                position: enemy.position,
            });
        }
    }

    fn fly(&mut self, actions: &Actions) {
        let dt = STEP_SECONDS;
        self.player.previous_position = self.player.position;
        self.player
            .look(actions.look, self.config.movement.max_pitch());
        let forward = look_direction(self.player.yaw, self.player.pitch);
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let velocity =
            (right * actions.move_input.x + forward * actions.move_input.y) * CHEAT_FLY_SPEED;
        self.player.position += velocity * dt;
        self.player.velocity = velocity;
        self.player.grounded = false;
        self.player.is_low = false;
        self.player.state = player::MoveState::Normal;
    }

    pub fn ground_height(&self, point: Vec3) -> f32 {
        let origin = point + Vec3::Y * 0.05;
        let ray = Ray::new(origin, Vec3::NEG_Y);
        self.collision
            .ray(&ray, 300.0)
            .map_or(point.y - 30.0, |(distance, _)| origin.y - distance)
    }

    pub fn elapsed_seconds(&self) -> f32 {
        self.elapsed_steps as f32 * STEP_SECONDS
    }

    fn update_weapon(&mut self, actions: &Actions) {
        self.weapon_cooldown_seconds = (self.weapon_cooldown_seconds - STEP_SECONDS).max(0.0);
        if !actions.fire_held || self.weapon_cooldown_seconds > 0.0 {
            return;
        }
        self.weapon_cooldown_seconds = self.config.weapon.cooldown_seconds;

        let origin = self.eye_position();
        let direction = look_direction(self.player.yaw, self.player.pitch);
        let range = self.config.weapon.range;
        let hit = weapon::trace_shot(
            &Ray::new(origin, direction),
            range,
            &self.collision,
            &self.targets,
            &self.switches,
            &self.enemies,
            &self.config.grunt,
        );
        let end = origin + direction * hit.distance(range);
        self.events.push(GameEvent::ShotFired { origin, end });

        let damage = self.config.weapon.damage;
        match hit {
            ShotHit::Nothing => {}
            ShotHit::Wall { normal, .. } => self.events.push(GameEvent::ShotImpact {
                position: end,
                normal,
            }),
            ShotHit::Target { index, .. } => {
                let target = &mut self.targets[index];
                target.health -= damage;
                self.events.push(GameEvent::TargetHit { id: target.id });
                if target.health <= 0.0 {
                    let id = target.id;
                    self.targets.remove(index);
                    self.events
                        .push(GameEvent::TargetDestroyed { id, position: end });
                }
            }
            ShotHit::Switch { index, .. } => {
                self.switches[index].health -= damage;
            }
            ShotHit::Enemy { index, .. } => {
                let enemy = &mut self.enemies[index];
                enemy.health -= damage;
                let push = (end - self.player.position).normalize_or_zero();
                let mut shove =
                    Vec3::new(push.x, 0.0, push.z).normalize_or_zero() * ENEMY_KNOCKBACK;
                shove += Vec3::Y * ENEMY_KNOCKBACK * 0.05;
                enemy.knockback += shove;
                self.events.push(GameEvent::EnemyHit {
                    id: enemy.id,
                    position: end,
                    normal: -push,
                });
                if enemy.health <= 0.0 {
                    let id = enemy.id;
                    let center = enemy.position + Vec3::Y * self.config.grunt.height * 0.5;
                    self.enemies.remove(index);
                    parts::spawn_parts(center, id, &self.config.health, &mut self.parts);
                    self.kills += 1;
                    self.events.push(GameEvent::EnemyKilled {
                        id,
                        position: center,
                    });
                }
            }
        }
    }
}

impl Game {
    fn check_exit(&mut self) {
        let Some(exit) = &self.level.exit else {
            return;
        };
        let arenas_cleared = self
            .arenas
            .iter()
            .all(|arena| arena.phase == ArenaPhase::Cleared);
        if arenas_cleared && exit.contains(self.player.position) {
            self.result = Some(LevelResult {
                seconds: self.elapsed_seconds(),
                kills: self.kills,
            });
            self.events.push(GameEvent::LevelCompleted);
        }
    }
}
