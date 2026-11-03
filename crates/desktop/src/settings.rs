use eframe::egui::{self, Key};
use serde_json::{Value, json};
use std::path::Path;
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub pause: Key,
    pub save: Key,
    pub load: Key,
    pub next_event: Key,
    pub ui_scale: f32,
    pub pause_on_focus_loss: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            pause: Key::Space,
            save: Key::F5,
            load: Key::F9,
            next_event: Key::N,
            ui_scale: 1.0,
            pause_on_focus_loss: true,
        }
    }
}
fn allowed(key: Key) -> bool {
    !matches!(
        key,
        Key::Escape
            | Key::Tab
            | Key::Enter
            | Key::ArrowLeft
            | Key::ArrowRight
            | Key::ArrowUp
            | Key::ArrowDown
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
            | Key::F1
    )
}
impl Settings {
    fn validate(&self) -> Result<(), String> {
        if !self.ui_scale.is_finite() || !(0.75..=2.0).contains(&self.ui_scale) {
            return Err("UI scale must be 0.75–2.0".into());
        }
        let keys = [self.pause, self.save, self.load, self.next_event];
        if keys.iter().any(|k| !allowed(*k)) {
            return Err("Navigation keys and F1 are reserved".into());
        }
        if keys.iter().enumerate().any(|(i, k)| keys[..i].contains(k)) {
            return Err("Each action needs a different shortcut".into());
        }
        Ok(())
    }
    fn json(&self) -> Value {
        json!({"version":1,"pause":self.pause.name(),"save":self.save.name(),"load":self.load.name(),"next_event":self.next_event.name(),"ui_scale":self.ui_scale,"pause_on_focus_loss":self.pause_on_focus_loss})
    }
    fn parse(value: &Value) -> Result<Self, String> {
        if value["version"].as_u64() != Some(1) {
            return Err("Unsupported UI settings version".into());
        }
        let key = |name: &str| {
            value[name]
                .as_str()
                .and_then(Key::from_name)
                .ok_or_else(|| format!("Invalid {name} shortcut"))
        };
        let settings = Self {
            pause: key("pause")?,
            save: key("save")?,
            load: key("load")?,
            next_event: key("next_event")?,
            ui_scale: value["ui_scale"].as_f64().ok_or("Missing UI scale")? as f32,
            pause_on_focus_loss: value["pause_on_focus_loss"]
                .as_bool()
                .ok_or("Missing focus preference")?,
        };
        settings.validate()?;
        Ok(settings)
    }
    fn read(path: &Path) -> Result<Self, String> {
        if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 65536 {
            return Err("UI settings file exceeds 64 KiB".into());
        }
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        Self::parse(&serde_json::from_slice(&bytes).map_err(|e| e.to_string())?)
    }
    fn write(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let temp = path.with_extension("tmp");
        std::fs::write(
            &temp,
            serde_json::to_vec_pretty(&self.json()).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(temp, path).map_err(|e| e.to_string())
    }
}
pub fn shortcut(ctx: &egui::Context, key: Key) -> bool {
    let function_key = key
        .name()
        .strip_prefix('F')
        .is_some_and(|name| name.parse::<u8>().is_ok());
    if ctx.wants_keyboard_input() && !function_key {
        return false;
    }
    ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, key))
}
#[derive(Default)]
pub struct Preferences {
    pub active: Settings,
    draft: Settings,
    message: String,
}
impl Preferences {
    pub fn load() -> Self {
        let path = Path::new("saves/ui-settings.json");
        let (active, message) = if path.exists() {
            match Settings::read(path) {
                Ok(settings) => (settings, String::new()),
                Err(error) => (
                    Settings::default(),
                    format!(
                        "Invalid preferences: {error}. Defaults are active; the file is retained until you apply new settings."
                    ),
                ),
            }
        } else {
            (Settings::default(), String::new())
        };
        Self {
            draft: active.clone(),
            active,
            message,
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.heading("Controls & accessibility");
        ui.label("F1 always opens settings. Escape pauses and returns to the map. Navigation keys remain available to widgets, the map and chart inspection.");
        for (label, key) in [
            ("Pause / play", &mut self.draft.pause),
            ("Save", &mut self.draft.save),
            ("Load", &mut self.draft.load),
            ("Next important event", &mut self.draft.next_event),
        ] {
            egui::ComboBox::from_label(label)
                .selected_text(key.name())
                .show_ui(ui, |ui| {
                    for candidate in Key::ALL.iter().filter(|k| allowed(**k)) {
                        ui.selectable_value(key, *candidate, candidate.name());
                    }
                });
        }
        ui.add(egui::Slider::new(&mut self.draft.ui_scale, 0.75..=2.0).text("Interface scale"));
        ui.checkbox(
            &mut self.draft.pause_on_focus_loss,
            "Pause when the game window loses focus",
        );
        let validation = self.draft.validate();
        if let Err(error) = &validation {
            ui.colored_label(egui::Color32::LIGHT_YELLOW, error);
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(validation.is_ok(), egui::Button::new("Apply settings"))
                .clicked()
            {
                self.active = self.draft.clone();
                ui.ctx().set_zoom_factor(self.active.ui_scale);
                self.message = match self.active.write(Path::new("saves/ui-settings.json")) {
                    Ok(()) => {
                        "Applied and saved UI settings. Campaign rules and state are unchanged."
                            .into()
                    }
                    Err(error) => {
                        format!("Applied for this session; preferences could not be saved: {error}")
                    }
                };
            }
            if ui.button("Restore default draft").clicked() {
                self.draft = Settings::default();
            }
        });
        ui.label(&self.message);
        ui.separator();
        ui.label("Charts include inspectable text values and map overlays include numeric labels. High-DPI scaling uses the operating system plus this interface scale. Full screen-reader certification and localization remain under development.");
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn controls_roundtrip_and_reject_conflicts_or_reserved_navigation() {
        let mut settings = Settings {
            pause: Key::P,
            save: Key::F6,
            load: Key::F8,
            ..Default::default()
        };
        assert_eq!(Settings::parse(&settings.json()).unwrap(), settings);
        settings.load = Key::F6;
        assert!(settings.validate().is_err());
        settings.load = Key::Tab;
        assert!(settings.validate().is_err());
        settings.load = Key::F8;
        settings.ui_scale = f32::NAN;
        assert!(settings.validate().is_err());
    }
}
