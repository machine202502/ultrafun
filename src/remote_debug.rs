use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};

use game::Actions;
use glam::Vec2;
use serde::{Deserialize, Serialize};

pub const REMOTE_DEBUG_PORT: u16 = 4852;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum RemoteAction {
    Jump,
    Dash,
    Slide,
    Fire,
    AltFire,
    Weapon1,
    Weapon2,
    Weapon3,
    Respawn,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum RemoteCommand {
    State,
    Screenshot { path: Option<String> },
    LookDelta { dx: f32, dy: f32 },
    Pointer { x: f32, y: f32 },
    Press { action: RemoteAction },
    Release { action: RemoteAction },
    MoveInput { x: f32, y: f32 },
    Reload,
    SetLevel { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteAppCommand {
    State,
    Reload,
    SetLevel { name: String },
}

#[derive(Debug, Serialize)]
pub struct RemoteState {
    pub level: String,
    pub position: [f32; 3],
    pub eye: [f32; 3],
    pub look_direction: [f32; 3],
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub health: f32,
    pub elapsed_seconds: f32,
    pub cheats: bool,
    pub selected_weapon: String,
    pub charge: f32,
    pub beam: bool,
    pub projectiles: usize,
    pub enemies: usize,
}

#[derive(Default)]
struct RemoteInput {
    look_delta: Vec2,
    move_input: Vec2,
    jump_pressed: bool,
    dash_pressed: bool,
    slide_pressed: bool,
    slide_held: bool,
    fire_held: bool,
    alt_held: bool,
    alt_released: bool,
    select_weapon: Option<game::WeaponKind>,
    respawn: bool,
}

pub struct RemoteDebug {
    listener: TcpListener,
    client: Option<TcpStream>,
    received: Vec<u8>,
    outgoing: Vec<u8>,
    input: RemoteInput,
    pointer: Option<(f32, f32)>,
    screenshot_path: Option<String>,
}

impl RemoteDebug {
    pub fn start(port: u16) -> std::io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            client: None,
            received: Vec::new(),
            outgoing: Vec::new(),
            input: RemoteInput::default(),
            pointer: None,
            screenshot_path: None,
        })
    }

    pub fn port(&self) -> u16 {
        self.listener
            .local_addr()
            .map_or(0, |address| address.port())
    }

    pub fn pointer(&self) -> Option<(f32, f32)> {
        self.pointer
    }

    pub fn take_screenshot_path(&mut self) -> Option<String> {
        self.screenshot_path.take()
    }

    pub fn send(&mut self, text: &str) {
        self.outgoing.extend_from_slice(text.as_bytes());
        self.outgoing.push(b'\n');
    }

    pub fn poll(&mut self) -> Vec<RemoteAppCommand> {
        self.accept_client();
        let mut commands = Vec::new();
        let bytes = self.read_bytes();
        let mut line_start = 0;
        for (index, byte) in bytes.iter().enumerate() {
            if *byte != b'\n' {
                continue;
            }
            if index > line_start {
                let line = String::from_utf8_lossy(&bytes[line_start..index]).into_owned();
                match self.handle_line(&line) {
                    Ok(command) => commands.extend(command),
                    Err(error) => self.send(&format!("bad command: {error}")),
                }
            }
            line_start = index + 1;
        }
        self.flush();
        commands
    }

    fn handle_line(
        &mut self,
        line: &str,
    ) -> Result<Option<RemoteAppCommand>, ron::error::SpannedError> {
        match ron::from_str::<RemoteCommand>(line)? {
            RemoteCommand::State => Ok(Some(RemoteAppCommand::State)),
            RemoteCommand::Reload => Ok(Some(RemoteAppCommand::Reload)),
            RemoteCommand::SetLevel { name } => Ok(Some(RemoteAppCommand::SetLevel { name })),
            RemoteCommand::Screenshot { path } => {
                self.screenshot_path = Some(path.unwrap_or_else(default_screenshot_path));
                Ok(None)
            }
            RemoteCommand::LookDelta { dx, dy } => {
                self.input.look_delta += Vec2::new(dx, dy);
                Ok(None)
            }
            RemoteCommand::Pointer { x, y } => {
                self.pointer = Some((x, y));
                Ok(None)
            }
            RemoteCommand::MoveInput { x, y } => {
                self.input.move_input = Vec2::new(x, y);
                Ok(None)
            }
            RemoteCommand::Press { action } => {
                self.set_action(action, true);
                Ok(None)
            }
            RemoteCommand::Release { action } => {
                self.set_action(action, false);
                Ok(None)
            }
        }
    }

    fn set_action(&mut self, action: RemoteAction, held: bool) {
        match action {
            RemoteAction::Jump => self.input.jump_pressed = held,
            RemoteAction::Dash => self.input.dash_pressed = held,
            RemoteAction::Slide => {
                self.input.slide_pressed = held;
                self.input.slide_held = held;
            }
            RemoteAction::Fire => self.input.fire_held = held,
            RemoteAction::AltFire => {
                self.input.alt_released |= self.input.alt_held && !held;
                self.input.alt_held = held;
            }
            RemoteAction::Weapon1 if held => {
                self.input.select_weapon = Some(game::WeaponKind::RivetGun)
            }
            RemoteAction::Weapon2 if held => {
                self.input.select_weapon = Some(game::WeaponKind::Sandblaster)
            }
            RemoteAction::Weapon3 if held => {
                self.input.select_weapon = Some(game::WeaponKind::PlasmaCutter)
            }
            RemoteAction::Weapon1 | RemoteAction::Weapon2 | RemoteAction::Weapon3 => {}
            RemoteAction::Respawn => self.input.respawn = held,
        }
    }

    pub fn merge_actions(&mut self, actions: &mut Actions) {
        let input = &self.input;
        actions.look += input.look_delta;
        if actions.move_input == Vec2::ZERO {
            actions.move_input = input.move_input;
        }
        actions.jump_pressed |= input.jump_pressed;
        actions.dash_pressed |= input.dash_pressed;
        actions.slide_pressed |= input.slide_pressed;
        actions.slide_held |= input.slide_held;
        actions.fire_held |= input.fire_held;
        actions.alt_held |= input.alt_held;
        actions.alt_released |= input.alt_released;
        if actions.select_weapon.is_none() {
            actions.select_weapon = input.select_weapon;
        }
        actions.respawn |= input.respawn;
        self.input.alt_released = false;
        self.input.select_weapon = None;
        self.input.look_delta = Vec2::ZERO;
        self.input.jump_pressed = false;
        self.input.dash_pressed = false;
        self.input.slide_pressed = false;
        self.input.respawn = false;
    }

    fn accept_client(&mut self) {
        while let Ok((stream, _)) = self.listener.accept() {
            let _ = stream.set_nonblocking(true);
            if self.client.is_none() {
                self.client = Some(stream);
                self.received.clear();
                log::info!("remote debug client connected on port {}", self.port());
            }
        }
    }

    fn read_bytes(&mut self) -> Vec<u8> {
        let mut buffer = [0u8; 4096];
        let Some(client) = &mut self.client else {
            return Vec::new();
        };
        match client.read(&mut buffer) {
            Ok(0) => {
                self.disconnect();
                Vec::new()
            }
            Ok(count) => {
                self.received.extend_from_slice(&buffer[..count]);
                std::mem::take(&mut self.received)
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => Vec::new(),
            Err(_) => {
                self.disconnect();
                Vec::new()
            }
        }
    }

    fn disconnect(&mut self) {
        self.client = None;
        self.received.clear();
        self.input = RemoteInput::default();
        self.pointer = None;
        log::info!("remote debug client disconnected");
    }

    fn flush(&mut self) {
        let Some(client) = &mut self.client else {
            self.outgoing.clear();
            return;
        };
        match client.write_all(&self.outgoing) {
            Ok(()) => self.outgoing.clear(),
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Err(_) => self.disconnect(),
        }
    }
}

fn default_screenshot_path() -> String {
    "target/remote_debug.png".to_owned()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn parses_state_command() {
        assert_eq!(
            ron::from_str::<RemoteCommand>("State").unwrap(),
            RemoteCommand::State
        );
    }

    #[test]
    fn parses_screenshot_with_path() {
        let command: RemoteCommand =
            ron::from_str(r#"Screenshot(path: Some("shot.png"))"#).unwrap();
        assert_eq!(
            command,
            RemoteCommand::Screenshot {
                path: Some("shot.png".to_owned())
            }
        );
    }

    #[test]
    fn parses_set_level() {
        let command: RemoteCommand = ron::from_str(r#"SetLevel(name: "foundry")"#).unwrap();
        assert_eq!(
            command,
            RemoteCommand::SetLevel {
                name: "foundry".to_owned()
            }
        );
    }

    #[test]
    fn remote_input_merges_into_actions() {
        let mut remote = RemoteDebug::start(0).unwrap();
        remote.handle_line("LookDelta(dx: 0.1, dy: -0.2)").unwrap();
        remote.handle_line("Press(action: Jump)").unwrap();
        remote.handle_line("MoveInput(x: 0.0, y: 1.0)").unwrap();
        let mut actions = Actions::default();
        remote.merge_actions(&mut actions);
        assert_eq!(actions.look, Vec2::new(0.1, -0.2));
        assert!(actions.jump_pressed);
        assert_eq!(actions.move_input, Vec2::new(0.0, 1.0));
    }

    #[test]
    fn player_move_input_wins_over_remote() {
        let mut remote = RemoteDebug::start(0).unwrap();
        remote.handle_line("MoveInput(x: 1.0, y: 0.0)").unwrap();
        let mut actions = Actions {
            move_input: Vec2::new(0.0, 1.0),
            ..Default::default()
        };
        remote.merge_actions(&mut actions);
        assert_eq!(actions.move_input, Vec2::new(0.0, 1.0));
    }

    #[test]
    fn loopback_delivers_commands_and_responses() {
        let mut remote = RemoteDebug::start(0).unwrap();
        let mut client = TcpStream::connect(("127.0.0.1", remote.port())).unwrap();
        client.write_all(b"State\n").unwrap();
        let commands = remote.poll();
        assert_eq!(commands, vec![RemoteAppCommand::State]);
        remote.send("ack");
        remote.poll();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut buffer = [0u8; 64];
        let count = client.read(&mut buffer).unwrap_or(0);
        assert!(count >= 3, "no response arrived");
        assert_eq!(&buffer[..3], b"ack");
    }
}
