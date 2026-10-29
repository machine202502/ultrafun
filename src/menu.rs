use egui::{Color32, Rect, Vec2};

use crate::text::Texts;

const BUTTON_WIDTH: f32 = 320.0;
const BUTTON_HEIGHT: f32 = 40.0;
const BUTTON_GAP: f32 = 8.0;
const TITLE_OFFSET: f32 = 28.0;
const MENU_BACKGROUND: Color32 = Color32::from_rgb(12, 10, 9);
const PANEL_BACKGROUND: Color32 = Color32::from_black_alpha(214);
const TITLE_COLOR: Color32 = Color32::from_rgb(240, 230, 210);

pub enum MenuAction {
    Load(String),
    Exit,
}

pub struct Menu {
    pub visible: bool,
    pub selected: usize,
    title: String,
    entries: Vec<(String, String)>,
    settings: String,
    exit: String,
}

fn button(label: &str, selected: bool) -> egui::Button<'_> {
    let button = egui::Button::new(label);
    if selected {
        button.fill(egui::Color32::from_rgb(150, 60, 30))
    } else {
        button
    }
}

fn stacked_rects(screen: Rect, count: usize) -> Vec<Rect> {
    let total = count as f32 * (BUTTON_HEIGHT + BUTTON_GAP) - BUTTON_GAP;
    let first_top = screen.center().y - total * 0.5 + TITLE_OFFSET;
    (0..count)
        .map(|index| {
            let top = first_top + index as f32 * (BUTTON_HEIGHT + BUTTON_GAP);
            Rect::from_center_size(
                egui::pos2(screen.center().x, top + BUTTON_HEIGHT * 0.5),
                Vec2::new(BUTTON_WIDTH, BUTTON_HEIGHT),
            )
        })
        .collect()
}

impl Menu {
    pub fn new(level_names: &[&str], texts: &Texts, visible: bool) -> Self {
        let entries = level_names
            .iter()
            .map(|name| {
                (
                    (*name).to_owned(),
                    texts.get(&format!("level.{name}")).to_owned(),
                )
            })
            .collect();
        Self {
            visible,
            selected: 0,
            title: texts.get("menu.title").to_owned(),
            entries,
            settings: texts.get("menu.settings").to_owned(),
            exit: texts.get("menu.exit").to_owned(),
        }
    }

    fn button_rects(&self, screen: Rect) -> Vec<Rect> {
        stacked_rects(screen, self.entries.len() + 2)
    }

    pub fn move_selection(&mut self, delta: i32) {
        let count = (self.entries.len() + 2) as i32;
        self.selected = (self.selected as i32 + delta).rem_euclid(count) as usize;
    }

    pub fn hit_test(&mut self, position: egui::Pos2, screen: Rect) -> Option<MenuAction> {
        let rects = self.button_rects(screen);
        for (index, rect) in rects.iter().enumerate() {
            if !rect.contains(position) {
                continue;
            }
            self.selected = index;
            if index < self.entries.len() {
                return Some(MenuAction::Load(self.entries[index].0.clone()));
            }
            if index == self.entries.len() {
                return None;
            }
            return Some(MenuAction::Exit);
        }
        None
    }

    pub fn activate(&mut self) -> Option<MenuAction> {
        if self.selected < self.entries.len() {
            Some(MenuAction::Load(self.entries[self.selected].0.clone()))
        } else if self.selected == self.entries.len() {
            None
        } else {
            Some(MenuAction::Exit)
        }
    }

    pub fn draw(&mut self, ui: &mut egui::Ui) -> Option<MenuAction> {
        if !self.visible {
            return None;
        }
        let screen = ui.max_rect();
        let rects = self.button_rects(screen);
        let mut action = None;
        ui.painter().rect_filled(screen, 0.0, MENU_BACKGROUND);
        ui.painter().text(
            egui::pos2(screen.center().x, rects[0].top() - 40.0),
            egui::Align2::CENTER_BOTTOM,
            &self.title,
            egui::FontId::proportional(32.0),
            TITLE_COLOR,
        );
        for (index, (name, label)) in self.entries.iter().enumerate() {
            let button = button(label, index == self.selected);
            if ui.put(rects[index], button).clicked() {
                action = Some(MenuAction::Load(name.clone()));
            }
        }
        let settings_index = self.entries.len();
        let settings_rect = rects[settings_index];
        let _ = ui.put(
            settings_rect,
            button(&self.settings, self.selected == settings_index),
        );
        let exit_index = settings_index + 1;
        let exit_rect = rects[exit_index];
        if ui
            .put(exit_rect, button(&self.exit, self.selected == exit_index))
            .clicked()
        {
            action = Some(MenuAction::Exit);
        }
        action
    }
}
pub enum PauseAction {
    Resume,
    MainMenu,
}

pub struct PauseMenu {
    pub visible: bool,
    pub selected: usize,
    title: String,
    resume: String,
    main_menu: String,
}

impl PauseMenu {
    pub fn new(texts: &Texts) -> Self {
        Self {
            visible: false,
            selected: 0,
            title: texts.get("pause.title").to_owned(),
            resume: texts.get("pause.resume").to_owned(),
            main_menu: texts.get("pause.main_menu").to_owned(),
        }
    }

    pub fn move_selection(&mut self, delta: i32) {
        self.selected = (self.selected as i32 + delta).rem_euclid(2) as usize;
    }

    pub fn hit_test(&mut self, position: egui::Pos2, screen: Rect) -> Option<PauseAction> {
        let (resume_rect, menu_rect) = self.button_rects(screen);
        if resume_rect.contains(position) {
            self.selected = 0;
            return Some(PauseAction::Resume);
        }
        if menu_rect.contains(position) {
            self.selected = 1;
            return Some(PauseAction::MainMenu);
        }
        None
    }

    pub fn activate(&mut self) -> Option<PauseAction> {
        Some(if self.selected == 0 {
            PauseAction::Resume
        } else {
            PauseAction::MainMenu
        })
    }

    fn panel(&self, screen: Rect) -> Rect {
        Rect::from_center_size(screen.center(), Vec2::new(360.0, 260.0))
    }

    fn button_rects(&self, screen: Rect) -> (Rect, Rect) {
        let panel = self.panel(screen);
        (
            Rect::from_center_size(
                panel.center() + Vec2::new(0.0, -22.0),
                Vec2::new(280.0, 42.0),
            ),
            Rect::from_center_size(
                panel.center() + Vec2::new(0.0, 34.0),
                Vec2::new(280.0, 42.0),
            ),
        )
    }

    pub fn draw(&mut self, ui: &mut egui::Ui) -> Option<PauseAction> {
        if !self.visible {
            return None;
        }
        let screen = ui.max_rect();
        let panel = self.panel(screen);
        let (resume_rect, menu_rect) = self.button_rects(screen);
        let mut action = None;
        ui.painter().rect_filled(panel, 10.0, PANEL_BACKGROUND);
        ui.painter().text(
            panel.center_top() + Vec2::new(0.0, 28.0),
            egui::Align2::CENTER_TOP,
            &self.title,
            egui::FontId::proportional(26.0),
            TITLE_COLOR,
        );
        if ui
            .put(resume_rect, button(&self.resume, self.selected == 0))
            .clicked()
        {
            action = Some(PauseAction::Resume);
        }
        if ui
            .put(menu_rect, button(&self.main_menu, self.selected == 1))
            .clicked()
        {
            action = Some(PauseAction::MainMenu);
        }
        action
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn click(position: egui::Pos2) -> Option<MenuAction> {
        let texts = crate::text::Texts::load("en").unwrap();
        let mut menu = Menu::new(&["utilization", "foundry"], &texts, true);
        let screen = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1280.0, 720.0));
        let context = egui::Context::default();
        let mut action = None;
        for frame in 0..4 {
            let events = match frame {
                0 => vec![egui::Event::PointerMoved(position)],
                1 => vec![
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::default(),
                    },
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::default(),
                    },
                ],
                _ => Vec::new(),
            };
            let input = egui::RawInput {
                screen_rect: Some(screen),
                events,
                ..Default::default()
            };
            let output = context.run_ui(input, |ui| {
                if let Some(found) = menu.draw(ui) {
                    action = Some(found);
                }
            });
            output.drop_without_applying_deltas();
        }
        action
    }

    #[test]
    fn level_button_is_clickable() {
        let texts = crate::text::Texts::load("en").unwrap();
        let menu = Menu::new(&["utilization", "foundry"], &texts, true);
        let screen = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1280.0, 720.0));
        let button = menu.button_rects(screen)[1].center();
        match click(button) {
            Some(MenuAction::Load(name)) => assert_eq!(name, "foundry"),
            _ => panic!("level button did not react to a click"),
        }
    }

    #[test]
    fn pause_buttons_are_clickable() {
        let texts = crate::text::Texts::load("en").unwrap();
        let mut pause = PauseMenu::new(&texts);
        pause.visible = true;
        let screen = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1280.0, 720.0));
        let (resume_rect, menu_rect) = pause.button_rects(screen);
        for (position, expected) in [
            (resume_rect.center(), "resume"),
            (menu_rect.center(), "menu"),
        ] {
            let context = egui::Context::default();
            let mut action = None;
            for frame in 0..4 {
                let events = match frame {
                    0 => vec![egui::Event::PointerMoved(position)],
                    1 => vec![
                        egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: egui::Modifiers::default(),
                        },
                        egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers: egui::Modifiers::default(),
                        },
                    ],
                    _ => Vec::new(),
                };
                let input = egui::RawInput {
                    screen_rect: Some(screen),
                    events,
                    ..Default::default()
                };
                let output = context.run_ui(input, |ui| {
                    if let Some(found) = pause.draw(ui) {
                        action = Some(found);
                    }
                });
                output.drop_without_applying_deltas();
            }
            match (expected, action) {
                ("resume", Some(PauseAction::Resume)) => {}
                ("menu", Some(PauseAction::MainMenu)) => {}
                (_, found) => panic!("pause button {expected} did not react: {}", found.is_some()),
            }
        }
    }

    #[test]
    fn exit_button_is_clickable() {
        let texts = crate::text::Texts::load("en").unwrap();
        let menu = Menu::new(&["utilization"], &texts, true);
        let screen = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1280.0, 720.0));
        let button = menu.button_rects(screen)[2].center();
        assert!(matches!(click(button), Some(MenuAction::Exit)));
    }
}
#[cfg(test)]
mod keyboard_tests {
    use super::*;

    #[test]
    fn arrows_and_enter_pick_a_level() {
        let texts = crate::text::Texts::load("en").unwrap();
        let mut menu = Menu::new(&["utilization", "foundry"], &texts, true);
        assert!(matches!(menu.activate(), Some(MenuAction::Load(name)) if name == "utilization"));
        menu.move_selection(1);
        assert!(matches!(menu.activate(), Some(MenuAction::Load(name)) if name == "foundry"));
        menu.move_selection(1);
        assert!(menu.activate().is_none(), "settings must do nothing");
        menu.move_selection(1);
        assert!(matches!(menu.activate(), Some(MenuAction::Exit)));
        menu.move_selection(1);
        assert_eq!(menu.selected, 0);
    }

    #[test]
    fn pause_keyboard_switches_between_resume_and_menu() {
        let texts = crate::text::Texts::load("en").unwrap();
        let mut pause = PauseMenu::new(&texts);
        assert!(matches!(pause.activate(), Some(PauseAction::Resume)));
        pause.move_selection(1);
        assert!(matches!(pause.activate(), Some(PauseAction::MainMenu)));
        pause.move_selection(1);
        assert!(matches!(pause.activate(), Some(PauseAction::Resume)));
    }
}
#[cfg(test)]
mod hit_tests {
    use super::*;

    #[test]
    fn hit_test_matches_the_drawn_buttons() {
        let texts = crate::text::Texts::load("en").unwrap();
        let screen = Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1280.0, 720.0));
        let mut menu = Menu::new(&["utilization", "foundry"], &texts, true);
        let rects = menu.button_rects(screen);
        assert!(matches!(
            menu.hit_test(rects[1].center(), screen),
            Some(MenuAction::Load(name)) if name == "foundry"
        ));
        assert!(matches!(
            menu.hit_test(rects[3].center(), screen),
            Some(MenuAction::Exit)
        ));
        assert!(menu.hit_test(egui::pos2(5.0, 5.0), screen).is_none());

        let mut pause = PauseMenu::new(&texts);
        let (resume, main_menu) = pause.button_rects(screen);
        assert!(matches!(
            pause.hit_test(resume.center(), screen),
            Some(PauseAction::Resume)
        ));
        assert!(matches!(
            pause.hit_test(main_menu.center(), screen),
            Some(PauseAction::MainMenu)
        ));
    }
}
