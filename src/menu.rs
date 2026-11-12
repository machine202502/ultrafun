use std::collections::HashMap;

use egui::{Color32, Rect, Vec2};

use crate::text::Texts;

const TILE_COLUMNS: usize = 3;
const TILE_WIDTH: f32 = 250.0;
const TILE_IMAGE_HEIGHT: f32 = 141.0;
const TILE_LABEL_HEIGHT: f32 = 30.0;
const TILE_GAP: f32 = 18.0;
const TITLE_TOP: f32 = 44.0;
const BOTTOM_ROW_TOP: f32 = 40.0;
const MENU_BACKGROUND: Color32 = Color32::from_rgb(12, 10, 9);
const PANEL_BACKGROUND: Color32 = Color32::from_black_alpha(214);
const TITLE_COLOR: Color32 = Color32::from_rgb(240, 230, 210);
const TILE_BACKGROUND: Color32 = Color32::from_rgb(26, 24, 22);
const TILE_SELECTED: Color32 = Color32::from_rgb(224, 122, 40);
const TILE_LABEL_COLOR: Color32 = Color32::from_rgb(216, 210, 196);
const SETTINGS_WIDTH: f32 = 190.0;
const BUTTON_HEIGHT: f32 = 42.0;
const BUTTON_GAP: f32 = 12.0;
const PREVIEW_DIRECTORY: &str = "assets/levels/previews";

pub enum MenuAction {
    Load(String),
    Exit,
}

pub struct Menu {
    pub visible: bool,
    pub selected: usize,
    title: String,
    entries: Vec<(String, String)>,
    previews: HashMap<String, Option<egui::TextureHandle>>,
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
            previews: HashMap::new(),
            settings: texts.get("menu.settings").to_owned(),
            exit: texts.get("menu.exit").to_owned(),
        }
    }

    fn tile_rects(&self, screen: Rect) -> Vec<Rect> {
        let count = self.entries.len();
        let grid_width = TILE_COLUMNS as f32 * TILE_WIDTH + (TILE_COLUMNS as f32 - 1.0) * TILE_GAP;
        let left = screen.center().x - grid_width * 0.5;
        let top = screen.top() + TITLE_TOP + 72.0;
        (0..count)
            .map(|index| {
                let column = index % TILE_COLUMNS;
                let row = index / TILE_COLUMNS;
                Rect::from_min_size(
                    egui::pos2(
                        left + column as f32 * (TILE_WIDTH + TILE_GAP),
                        top + row as f32 * (TILE_IMAGE_HEIGHT + TILE_LABEL_HEIGHT + TILE_GAP),
                    ),
                    Vec2::new(TILE_WIDTH, TILE_IMAGE_HEIGHT + TILE_LABEL_HEIGHT),
                )
            })
            .collect()
    }

    fn bottom_rects(&self, screen: Rect) -> (Rect, Rect) {
        let settings_left = screen.center().x - SETTINGS_WIDTH - BUTTON_GAP * 0.5;
        let exit_left = screen.center().x + BUTTON_GAP * 0.5;
        let top = screen.bottom() - BOTTOM_ROW_TOP - BUTTON_HEIGHT;
        (
            Rect::from_min_size(
                egui::pos2(settings_left, top),
                Vec2::new(SETTINGS_WIDTH, BUTTON_HEIGHT),
            ),
            Rect::from_min_size(
                egui::pos2(exit_left, top),
                Vec2::new(SETTINGS_WIDTH, BUTTON_HEIGHT),
            ),
        )
    }

    pub fn choice_rects(&self, screen: Rect) -> Vec<Rect> {
        let mut rects = self.tile_rects(screen);
        let (settings, exit) = self.bottom_rects(screen);
        rects.push(settings);
        rects.push(exit);
        rects
    }

    pub fn move_selection(&mut self, delta: i32) {
        let count = (self.entries.len() + 2) as i32;
        self.selected = (self.selected as i32 + delta).rem_euclid(count) as usize;
    }

    pub fn hit_test(&mut self, position: egui::Pos2, screen: Rect) -> Option<MenuAction> {
        let rects = self.choice_rects(screen);
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
        let rects = self.choice_rects(screen);
        let mut action = None;
        ui.painter().rect_filled(screen, 0.0, MENU_BACKGROUND);
        ui.painter().text(
            egui::pos2(screen.center().x, screen.top() + TITLE_TOP),
            egui::Align2::CENTER_TOP,
            &self.title,
            egui::FontId::proportional(52.0),
            TITLE_COLOR,
        );
        for (index, (name, label)) in self.entries.iter().enumerate() {
            let rect = rects[index];
            let image_rect =
                Rect::from_min_size(rect.min, Vec2::new(TILE_WIDTH, TILE_IMAGE_HEIGHT));
            let response = ui.allocate_rect(rect, egui::Sense::click());
            if response.clicked() {
                action = Some(MenuAction::Load(name.clone()));
            }
            if response.hovered() {
                self.selected = index;
            }
            ui.painter().rect_filled(image_rect, 4.0, TILE_BACKGROUND);
            let texture = self
                .previews
                .entry(name.clone())
                .or_insert_with(|| load_preview(ui.ctx(), name))
                .as_ref();
            match texture {
                Some(texture) => {
                    ui.painter().image(
                        texture.id(),
                        image_rect,
                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                None => {
                    let first = label.chars().next().unwrap_or('?');
                    ui.painter().text(
                        image_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        first.to_string(),
                        egui::FontId::proportional(48.0),
                        TILE_BACKGROUND,
                    );
                }
            }
            if self.selected == index {
                ui.painter().rect_stroke(
                    rect,
                    4.0,
                    egui::Stroke::new(3.0, TILE_SELECTED),
                    egui::StrokeKind::Middle,
                );
            }
            ui.painter().text(
                egui::pos2(rect.center().x, rect.bottom() - TILE_LABEL_HEIGHT * 0.5),
                egui::Align2::CENTER_CENTER,
                label,
                egui::FontId::proportional(19.0),
                if self.selected == index {
                    TILE_SELECTED
                } else {
                    TILE_LABEL_COLOR
                },
            );
        }
        let settings_index = self.entries.len();
        let exit_index = settings_index + 1;
        let _ = ui.put(
            rects[settings_index],
            button(&self.settings, self.selected == settings_index),
        );
        if ui
            .put(
                rects[exit_index],
                button(&self.exit, self.selected == exit_index),
            )
            .clicked()
        {
            action = Some(MenuAction::Exit);
        }
        action
    }
}

fn load_preview(context: &egui::Context, name: &str) -> Option<egui::TextureHandle> {
    let path = format!("{PREVIEW_DIRECTORY}/{name}.png");
    let bytes = std::fs::read(&path).ok()?;
    let image = image::load_from_memory(&bytes).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    let size = [width as usize, height as usize];
    Some(context.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
        egui::TextureOptions::default(),
    ))
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

    fn screen() -> Rect {
        Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1280.0, 720.0))
    }

    #[test]
    fn keyboard_walks_tiles_settings_and_exit() {
        let texts = crate::text::Texts::load("en").unwrap();
        let mut menu = Menu::new(&["utilization", "foundry"], &texts, true);
        assert!(matches!(
            menu.activate(),
            Some(MenuAction::Load(name)) if name == "utilization"
        ));
        menu.move_selection(1);
        assert!(matches!(
            menu.activate(),
            Some(MenuAction::Load(name)) if name == "foundry"
        ));
        menu.move_selection(1);
        assert!(menu.activate().is_none(), "settings must do nothing");
        menu.move_selection(1);
        assert!(matches!(menu.activate(), Some(MenuAction::Exit)));
        menu.move_selection(1);
        assert_eq!(menu.selected, 0);
    }

    #[test]
    fn hit_test_matches_the_drawn_choices() {
        let texts = crate::text::Texts::load("en").unwrap();
        let mut menu = Menu::new(&["utilization", "foundry"], &texts, true);
        let rects = menu.choice_rects(screen());
        assert!(matches!(
            menu.hit_test(rects[1].center(), screen()),
            Some(MenuAction::Load(name)) if name == "foundry"
        ));
        assert!(matches!(
            menu.hit_test(rects[3].center(), screen()),
            Some(MenuAction::Exit)
        ));
        assert!(menu.hit_test(egui::pos2(5.0, 5.0), screen()).is_none());

        let mut pause = PauseMenu::new(&texts);
        let (resume, main_menu) = pause.button_rects(screen());
        assert!(matches!(
            pause.hit_test(resume.center(), screen()),
            Some(PauseAction::Resume)
        ));
        assert!(matches!(
            pause.hit_test(main_menu.center(), screen()),
            Some(PauseAction::MainMenu)
        ));
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
