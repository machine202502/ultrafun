use game::GameEvent;

const LINE_SECONDS: f32 = 4.0;

#[derive(Default)]
pub struct Dispatcher {
    line: Option<(String, f32)>,
}

impl Dispatcher {
    pub fn handle(&mut self, event: &GameEvent) -> bool {
        let key = match event {
            GameEvent::LevelStarted => "dispatcher.reject_announced",
            GameEvent::ArenaStarted { .. } => "dispatcher.arena_started",
            GameEvent::ArenaCleared { .. } => "dispatcher.arena_cleared",
            GameEvent::PlayerDied => "dispatcher.player_died",
            GameEvent::LevelCompleted => "dispatcher.level_complete",
            GameEvent::CheckpointReached { .. } => "dispatcher.checkpoint",
            GameEvent::Say { key } => {
                self.line = Some((key.clone(), LINE_SECONDS));
                return true;
            }
            _ => return false,
        };
        self.line = Some((key.to_owned(), LINE_SECONDS));
        true
    }

    pub fn advance(&mut self, seconds: f32) {
        if let Some((_, remaining)) = &mut self.line {
            *remaining -= seconds;
            if *remaining <= 0.0 {
                self.line = None;
            }
        }
    }

    pub fn current_key(&self) -> Option<&str> {
        self.line.as_ref().map(|(key, _)| key.as_str())
    }
}
