use std::collections::HashSet;

use game::Actions;
use glam::Vec2;
use winit::keyboard::KeyCode;

use crate::settings::KeyBindings;

#[derive(Default)]
pub struct Input {
    held: HashSet<KeyCode>,
    mouse_delta: Vec2,
    jump_pressed: bool,
    dash_pressed: bool,
    slide_pressed: bool,
    respawn_pressed: bool,
    fire_held: bool,
}

impl Input {
    pub fn add_mouse_motion(&mut self, dx: f64, dy: f64) {
        self.mouse_delta += Vec2::new(dx as f32, dy as f32);
    }

    pub fn key_pressed(&mut self, key: KeyCode, keys: &KeyBindings) {
        self.held.insert(key);
        self.jump_pressed |= key == keys.jump;
        self.dash_pressed |= key == keys.dash;
        self.slide_pressed |= key == keys.slide;
        self.respawn_pressed |= key == keys.respawn;
    }

    pub fn key_released(&mut self, key: KeyCode) {
        self.held.remove(&key);
    }

    pub fn set_fire_held(&mut self, held: bool) {
        self.fire_held = held;
    }

    pub fn release_all(&mut self) {
        self.fire_held = false;
        self.held.clear();
    }

    pub fn pending_look(&self, sensitivity: f32) -> Vec2 {
        self.mouse_delta * sensitivity
    }

    pub fn take_actions(&mut self, sensitivity: f32, keys: &KeyBindings) -> Actions {
        let axis = |positive: KeyCode, negative: KeyCode| {
            f32::from(u8::from(self.held.contains(&positive)))
                - f32::from(u8::from(self.held.contains(&negative)))
        };
        let actions = Actions {
            look: self.pending_look(sensitivity),
            move_input: Vec2::new(
                axis(keys.right, keys.left),
                axis(keys.forward, keys.backward),
            ),
            jump_pressed: self.jump_pressed,
            dash_pressed: self.dash_pressed,
            slide_pressed: self.slide_pressed,
            slide_held: self.held.contains(&keys.slide),
            fire_held: self.fire_held,
            respawn: self.respawn_pressed,
        };
        self.mouse_delta = Vec2::ZERO;
        self.jump_pressed = false;
        self.dash_pressed = false;
        self.slide_pressed = false;
        self.respawn_pressed = false;
        actions
    }
}
