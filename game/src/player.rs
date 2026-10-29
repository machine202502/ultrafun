use glam::{Vec2, Vec3};

use crate::MovementConfig;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MoveState {
    Normal,
    Dash {
        direction: Vec3,
        remaining_seconds: f32,
    },
    Slide {
        direction: Vec3,
        speed: f32,
    },
    Slam {
        start_height: f32,
    },
}

#[derive(Debug, Clone)]
pub struct Player {
    pub position: Vec3,
    pub previous_position: Vec3,
    pub velocity: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub state: MoveState,
    pub grounded: bool,
    pub is_low: bool,
    pub stamina: f32,
    pub health: f32,
    pub wall_jumps_used: u32,
    pub wall_cling_seconds: f32,
    pub coyote_seconds: f32,
    pub jump_buffer_seconds: f32,
    pub slam_bounce_seconds: f32,
    pub slam_bounce_height: f32,
}

impl Player {
    pub fn spawned_at(position: Vec3, yaw: f32, stamina: f32, health: f32) -> Self {
        Self {
            position,
            previous_position: position,
            velocity: Vec3::ZERO,
            yaw,
            pitch: 0.0,
            state: MoveState::Normal,
            grounded: false,
            is_low: false,
            stamina,
            health,
            wall_jumps_used: 0,
            wall_cling_seconds: 0.0,
            coyote_seconds: 0.0,
            jump_buffer_seconds: 0.0,
            slam_bounce_seconds: 0.0,
            slam_bounce_height: 0.0,
        }
    }

    pub fn looked(&self, look: Vec2, max_pitch: f32) -> (f32, f32) {
        let yaw = (self.yaw + look.x).rem_euclid(std::f32::consts::TAU);
        let pitch = (self.pitch - look.y).clamp(-max_pitch, max_pitch);
        (yaw, pitch)
    }

    pub fn look(&mut self, look: Vec2, max_pitch: f32) {
        (self.yaw, self.pitch) = self.looked(look, max_pitch);
    }

    pub fn interpolated_position(&self, alpha: f32) -> Vec3 {
        self.previous_position.lerp(self.position, alpha)
    }

    pub fn eye_height(&self, config: &MovementConfig) -> f32 {
        if self.is_low {
            config.slide_eye_height
        } else {
            config.eye_height
        }
    }

    pub fn horizontal_speed(&self) -> f32 {
        Vec2::new(self.velocity.x, self.velocity.z).length()
    }
}

pub fn look_direction(yaw: f32, pitch: f32) -> Vec3 {
    Vec3::new(
        yaw.sin() * pitch.cos(),
        pitch.sin(),
        -yaw.cos() * pitch.cos(),
    )
}

pub fn flat_direction(yaw: f32, input: Vec2) -> Vec3 {
    let forward = Vec3::new(yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(yaw.cos(), 0.0, yaw.sin());
    (right * input.x + forward * input.y).normalize_or_zero()
}
