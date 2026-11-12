use egui::{Align2, Color32, FontId, Rect, pos2, vec2};

use crate::dispatcher::Dispatcher;
use crate::text::Texts;

const CROSSHAIR_RADIUS: f32 = 2.0;
const HEALTH_BAR_SIZE: egui::Vec2 = vec2(240.0, 14.0);
const HEALTH_BAR_MARGIN: f32 = 24.0;
const HEALTH_BAR_BACK: Color32 = Color32::from_rgb(40, 10, 10);
const HEALTH_BAR_FILL: Color32 = Color32::from_rgb(220, 40, 30);
const DEATH_TINT: Color32 = Color32::from_rgba_premultiplied(120, 0, 0, 120);
const PANEL_BACK: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 190);
const SUBTITLE_COLOR: Color32 = Color32::from_rgb(255, 214, 120);
const SUBTITLE_SIZE: f32 = 22.0;
const SUBTITLE_BOTTOM_OFFSET: f32 = 80.0;
const REPORT_COLOR: Color32 = Color32::from_rgb(235, 235, 225);
const REPORT_TITLE_SIZE: f32 = 34.0;
const REPORT_LINE_SIZE: f32 = 24.0;
const REPORT_LINE_SPACING: f32 = 12.0;
const PANEL_PADDING: f32 = 16.0;
const KEY_SIZE: f32 = 18.0;
const KEY_COLOR: Color32 = Color32::from_rgb(255, 216, 64);
const KEY_MARGIN: f32 = 24.0;
const KEY_LINE_SPACING: f32 = 4.0;

pub struct Hud {
    pub health_fraction: f32,
    pub dead: bool,
    pub cheats: Option<String>,
    pub subtitle: Option<String>,
    pub keys: Vec<String>,
    pub report: Option<Vec<String>>,
    pub weapon: Option<(String, f32, f32)>,
}

pub fn draw_hud(ui: &egui::Ui, hud: &Hud) {
    let screen = ui.max_rect();
    let painter = ui.painter();
    if hud.dead {
        painter.rect_filled(screen, 0.0, DEATH_TINT);
    }
    draw_crosshair(painter, screen);
    draw_health(painter, screen, hud.health_fraction);
    if let Some((name, ready, charge)) = &hud.weapon {
        let origin = pos2(screen.left() + 24.0, screen.bottom() - 65.0);
        painter.text(
            origin,
            Align2::LEFT_BOTTOM,
            name,
            FontId::monospace(18.0),
            REPORT_COLOR,
        );
        let bar = Rect::from_min_size(origin + vec2(0.0, 5.0), vec2(240.0, 4.0));
        painter.rect_filled(bar, 0.0, PANEL_BACK);
        let mut fill = bar;
        fill.set_width(bar.width() * if *charge > 0.0 { *charge } else { *ready });
        painter.rect_filled(
            fill,
            0.0,
            if *charge > 0.0 {
                Color32::from_rgb(255, 150, 45)
            } else {
                Color32::from_rgb(60, 200, 210)
            },
        );
    }
    if let Some(cheats) = &hud.cheats {
        painter.text(
            screen.left_top() + egui::vec2(16.0, 16.0),
            egui::Align2::LEFT_TOP,
            cheats,
            egui::FontId::monospace(14.0),
            Color32::from_rgb(255, 210, 90),
        );
    }
    draw_keys(painter, screen, &hud.keys);
    if let Some(subtitle) = &hud.subtitle {
        draw_subtitle(painter, screen, subtitle);
    }
    if let Some(lines) = &hud.report {
        draw_report(painter, screen, lines);
    }
}

fn draw_crosshair(painter: &egui::Painter, screen: Rect) {
    let center = screen.center();
    painter.circle_filled(center, CROSSHAIR_RADIUS + 1.0, Color32::BLACK);
    painter.circle_filled(center, CROSSHAIR_RADIUS, Color32::WHITE);
}

fn draw_health(painter: &egui::Painter, screen: Rect, fraction: f32) {
    let bar = Rect::from_min_size(
        pos2(
            screen.left() + HEALTH_BAR_MARGIN,
            screen.bottom() - HEALTH_BAR_MARGIN - HEALTH_BAR_SIZE.y,
        ),
        HEALTH_BAR_SIZE,
    );
    painter.rect_filled(bar, 0.0, HEALTH_BAR_BACK);
    let mut fill = bar;
    fill.set_width(bar.width() * fraction.clamp(0.0, 1.0));
    painter.rect_filled(fill, 0.0, HEALTH_BAR_FILL);
}

fn draw_keys(painter: &egui::Painter, screen: Rect, keys: &[String]) {
    let mut bottom = screen.bottom() - KEY_MARGIN;
    for key in keys {
        let galley = painter.layout_no_wrap(key.clone(), FontId::proportional(KEY_SIZE), KEY_COLOR);
        let height = galley.size().y;
        let rect = Align2::RIGHT_BOTTOM
            .anchor_size(pos2(screen.right() - KEY_MARGIN, bottom), galley.size());
        painter.rect_filled(rect.expand(4.0), 3.0, PANEL_BACK);
        painter.galley(rect.min, galley, KEY_COLOR);
        bottom -= height + KEY_LINE_SPACING;
    }
}

fn draw_subtitle(painter: &egui::Painter, screen: Rect, text: &str) {
    let galley = painter.layout_no_wrap(
        text.to_owned(),
        FontId::proportional(SUBTITLE_SIZE),
        SUBTITLE_COLOR,
    );
    let anchor = pos2(screen.center().x, screen.bottom() - SUBTITLE_BOTTOM_OFFSET);
    let rect = Align2::CENTER_BOTTOM.anchor_size(anchor, galley.size());
    painter.rect_filled(rect.expand(PANEL_PADDING * 0.5), 4.0, PANEL_BACK);
    painter.galley(rect.min, galley, SUBTITLE_COLOR);
}

fn draw_report(painter: &egui::Painter, screen: Rect, lines: &[String]) {
    let galleys: Vec<_> = lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let size = if index == 0 {
                REPORT_TITLE_SIZE
            } else {
                REPORT_LINE_SIZE
            };
            painter.layout_no_wrap(line.clone(), FontId::proportional(size), REPORT_COLOR)
        })
        .collect();
    let width = galleys
        .iter()
        .map(|galley| galley.size().x)
        .fold(0.0, f32::max);
    let height = galleys.iter().map(|galley| galley.size().y).sum::<f32>()
        + REPORT_LINE_SPACING * galleys.len().saturating_sub(1) as f32;
    let panel = Align2::CENTER_CENTER.anchor_size(screen.center(), vec2(width, height));
    painter.rect_filled(panel.expand(PANEL_PADDING), 6.0, PANEL_BACK);
    let mut top = panel.top();
    for galley in galleys {
        let line_height = galley.size().y;
        let left = screen.center().x - galley.size().x * 0.5;
        painter.galley(pos2(left, top), galley, REPORT_COLOR);
        top += line_height + REPORT_LINE_SPACING;
    }
}

pub fn build_hud(game: &game::Game, dispatcher: &Dispatcher, texts: &Texts) -> Hud {
    let report = game.result.map(|result| {
        let mut lines = vec![
            texts.get("report.title").to_owned(),
            format!(
                "{}: {}",
                texts.get("report.time"),
                format_duration(result.seconds)
            ),
            format!("{}: {}", texts.get("report.kills"), result.kills),
            texts.get("report.restart").to_owned(),
        ];
        if game.level().next_level.is_some() {
            lines.push(texts.get("report.next").to_owned());
        }
        lines
    });
    let weapon_name = match game.weapon.selected {
        game::WeaponKind::RivetGun => "weapon.rivet",
        game::WeaponKind::Sandblaster => "weapon.sand",
        game::WeaponKind::PlasmaCutter => "weapon.plasma",
    };
    Hud {
        weapon: Some((
            texts.get(weapon_name).to_owned(),
            game.ready_fraction(),
            game.charge_fraction(),
        )),
        health_fraction: game.player.health / game.config.health.max_health,
        dead: game.restart_countdown_seconds.is_some(),
        cheats: game.cheats.then(|| texts.get("hud.cheats").to_owned()),
        subtitle: dispatcher
            .current_key()
            .map(|key| texts.get(key).to_owned()),
        keys: game
            .keys
            .iter()
            .filter(|key| key.collected)
            .map(|key| texts.get(&format!("key.{}", key.name)).to_owned())
            .collect(),
        report,
    }
}

fn format_duration(seconds: f32) -> String {
    let tenths = (seconds * 10.0).round() as u32;
    format!(
        "{:02}:{:02}.{}",
        tenths / 600,
        tenths / 10 % 60,
        tenths % 10
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_is_formatted_as_minutes_seconds_tenths() {
        assert_eq!(format_duration(0.0), "00:00.0");
        assert_eq!(format_duration(83.46), "01:23.5");
    }
}
