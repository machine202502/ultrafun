use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct MovementConfig {
    pub standing_height: f32,
    pub slide_height: f32,
    pub radius: f32,
    pub eye_height: f32,
    pub slide_eye_height: f32,
    pub step_height: f32,
    pub max_walkable_slope_degrees: f32,
    pub max_pitch_degrees: f32,

    pub run_speed: f32,
    pub ground_acceleration_seconds: f32,
    pub air_acceleration: f32,
    pub ground_excess_friction: f32,
    pub air_excess_friction: f32,

    pub gravity: f32,
    pub jump_speed: f32,
    pub max_fall_speed: f32,
    pub coyote_seconds: f32,
    pub jump_buffer_seconds: f32,

    pub stamina_max: f32,
    pub stamina_regen_per_second: f32,

    pub dash_cost: f32,
    pub dash_jump_cost: f32,
    pub dash_speed: f32,
    pub dash_seconds: f32,

    pub slide_speed: f32,
    pub slide_friction: f32,
    pub slide_max_speed: f32,

    pub wall_jumps_max: u32,
    pub wall_jump_away_speed: f32,
    pub wall_jump_up_speed: f32,
    pub slide_wall_jump_up_speed: f32,
    pub wall_max_tilt_degrees: f32,
    pub wall_contact_distance: f32,
    pub wall_cling_start_fall_speed: f32,
    pub wall_cling_fall_speed_growth: f32,

    pub slam_speed: f32,
    pub slam_bounce_seconds: f32,
    pub slam_bounce_max_height: f32,
}

impl MovementConfig {
    pub fn max_pitch(&self) -> f32 {
        self.max_pitch_degrees.to_radians()
    }

    pub fn jump_height(&self) -> f32 {
        self.jump_speed * self.jump_speed / (2.0 * self.gravity)
    }

    pub fn min_ground_normal_y(&self) -> f32 {
        self.max_walkable_slope_degrees.to_radians().cos()
    }

    pub fn max_wall_normal_y(&self) -> f32 {
        self.wall_max_tilt_degrees.to_radians().sin()
    }

    pub fn ground_acceleration(&self) -> f32 {
        self.run_speed / self.ground_acceleration_seconds
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct HealthConfig {
    pub max_health: f32,
    pub restart_delay_seconds: f32,
    pub parts_per_kill: u32,
    pub part_heal: f32,
    pub part_launch_speed: f32,
    pub part_gravity: f32,
    pub part_attract_radius: f32,
    pub part_attract_speed: f32,
    pub part_collect_radius: f32,
    pub part_lifetime_seconds: f32,
}
