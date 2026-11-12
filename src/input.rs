use std::collections::HashSet;

use game::{Actions, WeaponKind};
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
    cheats_pressed: bool,
    fire_held: bool,
    alt_held: bool,
    alt_released: bool,
    cancel_attacks: bool,
    selected: WeaponKind,
    select_weapon: Option<WeaponKind>,
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
        self.cheats_pressed |= key == keys.cheats;
        if let Some(index) = keys.weapons.iter().position(|binding| key == *binding) {
            self.selected = WeaponKind::ALL[index];
            self.select_weapon = Some(self.selected);
        }
    }

    pub fn cycle_weapon(&mut self, delta: i32) {
        self.selected =
            WeaponKind::ALL[(self.selected.index() as i32 + delta).rem_euclid(3) as usize];
        self.select_weapon = Some(self.selected);
    }

    pub fn key_released(&mut self, key: KeyCode) {
        self.held.remove(&key);
    }
    pub fn set_fire_held(&mut self, held: bool) {
        self.fire_held = held;
    }

    pub fn set_alt_held(&mut self, held: bool) {
        self.alt_released |= self.alt_held && !held;
        self.alt_held = held;
    }

    pub fn release_all(&mut self) {
        self.fire_held = false;
        self.alt_held = false;
        self.alt_released = false;
        self.cancel_attacks = true;
        self.mouse_delta = Vec2::ZERO;
        self.jump_pressed = false;
        self.dash_pressed = false;
        self.slide_pressed = false;
        self.respawn_pressed = false;
        self.cheats_pressed = false;
        self.select_weapon = None;
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
            jump_pressed: std::mem::take(&mut self.jump_pressed),
            dash_pressed: std::mem::take(&mut self.dash_pressed),
            slide_pressed: std::mem::take(&mut self.slide_pressed),
            slide_held: self.held.contains(&keys.slide),
            fire_held: self.fire_held,
            alt_held: self.alt_held,
            alt_released: std::mem::take(&mut self.alt_released),
            cancel_attacks: std::mem::take(&mut self.cancel_attacks),
            select_weapon: self.select_weapon.take(),
            respawn: std::mem::take(&mut self.respawn_pressed),
            toggle_cheats: std::mem::take(&mut self.cheats_pressed),
        };
        self.mouse_delta = Vec2::ZERO;
        actions
    }
}
