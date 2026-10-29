use crate::render::{LightingFile, LightingStyle};

pub struct LightingLab {
    pub visible: bool,
    file: LightingFile,
    selected: usize,
    current: LightingStyle,
}

impl LightingLab {
    pub fn new(file: LightingFile) -> Self {
        let selected = file.active_index();
        let current = file.presets[selected].style.clone();
        Self {
            visible: false,
            file,
            selected,
            current,
        }
    }

    pub fn preset_name(&self, index: usize) -> &str {
        &self.file.presets[index].name
    }

    pub fn select(&mut self, index: usize) {
        if index != self.selected {
            self.selected = index;
            self.current = self.file.presets[index].style.clone();
        }
    }

    pub fn style(&self) -> &LightingStyle {
        &self.current
    }

    pub fn reload(&mut self, file: LightingFile) {
        let selected_name = self.file.presets[self.selected].name.clone();
        self.selected = file
            .presets
            .iter()
            .position(|preset| preset.name == selected_name)
            .unwrap_or_else(|| file.active_index());
        self.current = file.presets[self.selected].style.clone();
        self.file = file;
    }

    pub fn file_with_current_saved(&mut self) -> LightingFile {
        let preset = &mut self.file.presets[self.selected];
        preset.style = self.current.clone();
        self.file.active = preset.name.clone();
        self.file.clone()
    }

    pub fn ui(&mut self, context: &egui::Context) -> bool {
        let mut save = false;
        egui::Window::new("lighting lab")
            .default_width(320.0)
            .show(context, |ui| {
                let previous = self.selected;
                egui::ComboBox::from_label("preset")
                    .selected_text(&self.file.presets[self.selected].name)
                    .show_ui(ui, |ui| {
                        for (index, preset) in self.file.presets.iter().enumerate() {
                            ui.selectable_value(&mut self.selected, index, &preset.name);
                        }
                    });
                if self.selected != previous {
                    self.current = self.file.presets[self.selected].style.clone();
                }
                style_controls(ui, &mut self.current);
                ui.separator();
                ui.horizontal(|ui| {
                    save = ui.button("save into preset").clicked();
                    if ui.button("reset").clicked() {
                        self.current = self.file.presets[self.selected].style.clone();
                    }
                });
            });
        save
    }
}

fn style_controls(ui: &mut egui::Ui, style: &mut LightingStyle) {
    ui.heading("light");
    ui.checkbox(&mut style.linear_lighting, "linear light + tone mapping");
    ui.add(egui::Slider::new(&mut style.exposure, 0.2..=4.0).text("exposure"));
    ui.add(egui::Slider::new(&mut style.lamp_brightness, 0.0..=3.0).text("lamp brightness"));
    ui.add(egui::Slider::new(&mut style.ambient_brightness, 0.0..=12.0).text("ambient"));
    ui.horizontal(|ui| {
        ui.color_edit_button_rgb(&mut style.ambient_sky);
        ui.label("ambient from above");
        ui.color_edit_button_rgb(&mut style.ambient_ground);
        ui.label("from below");
    });

    ui.heading("surfaces");
    ui.add(egui::Slider::new(&mut style.texture_strength, 0.0..=1.0).text("pixel texture"));
    ui.add(
        egui::Slider::new(&mut style.texture_pixels_per_meter, 2.0..=32.0)
            .text("texture pixels per meter"),
    );
    ui.add(egui::Slider::new(&mut style.face_shading, 0.0..=1.0).text("face shading"));
    ui.add(egui::Slider::new(&mut style.saturation, 0.0..=2.0).text("saturation"));

    ui.heading("retro");
    ui.add(
        egui::Slider::new(&mut style.light_texel_size, 0.0..=1.0).text("light texel, m (0 off)"),
    );
    ui.add(egui::Slider::new(&mut style.light_bands, 0..=16).text("light bands (0 off)"));

    ui.heading("shadows");
    ui.checkbox(&mut style.shadows, "shadows");
    ui.add(
        egui::Slider::new(&mut style.shadow_resolution, 32..=256)
            .step_by(32.0)
            .text("shadow resolution"),
    );
    ui.checkbox(&mut style.shadow_soft, "soft edges");
    ui.add(egui::Slider::new(&mut style.shadow_strength, 0.0..=1.0).text("shadow strength"));

    ui.heading("global illumination (baked)");
    ui.checkbox(&mut style.gi, "bounced light");
    ui.add(egui::Slider::new(&mut style.gi_strength, 0.0..=4.0).text("bounce strength"));
    ui.add(egui::Slider::new(&mut style.gi_texel_size, 0.25..=2.0).text("bake texel, m"));
    ui.add(egui::Slider::new(&mut style.gi_bounces, 1..=4).text("bounces"));

    ui.heading("atmosphere");
    ui.add(egui::Slider::new(&mut style.fog_distance, 0.2..=3.0).text("fog distance"));
    ui.checkbox(&mut style.lamp_fixtures, "visible lamp fixtures");
    ui.add(egui::Slider::new(&mut style.fixture_size, 0.1..=1.5).text("fixture size"));
}
