use eframe::egui;
use sim_core::*;

pub struct Inspector {
    field: DeveloperField,
    value: i32,
    target: Option<CountryId>,
}
impl Default for Inspector {
    fn default() -> Self {
        Self {
            field: DeveloperField::Tax,
            value: 20,
            target: None,
        }
    }
}
impl Inspector {
    /// Called only by a host launched with the explicit developer capability.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        content: &Content,
        world: Option<&World>,
        target: &CountryId,
        can_edit: bool,
    ) -> Option<DeveloperEdit> {
        ui.heading("Developer state inspector");
        ui.colored_label(egui::Color32::LIGHT_YELLOW, "DEVELOPER VIEW · live world truth, including private foreign accounts. This is separate from government knowledge.");
        let Some(world) = world else {
            ui.label("Begin or load a campaign first.");
            return None;
        };
        let country = &world.countries[target];
        ui.label(format!(
            "{} · {} · rules {} · snapshot writer format {}",
            content.country(target).expect("country").name,
            date(world.day),
            world.rules,
            persistence::snapshot_schema(world)
        ));
        ui.label(if world.is_developer_modified() {
            "This campaign is developer-modified. Ordinary mandate certification is disabled."
        } else {
            "No developer edits recorded. Applying an edit permanently marks this campaign."
        });
        egui::Grid::new("developer_balances")
            .striped(true)
            .show(ui, |ui| {
                for (name, value) in [
                    ("Treasury", country.accounts.treasury.0),
                    ("Households", country.accounts.households.0),
                    ("Firms", country.accounts.firms.0),
                    ("External", country.accounts.external.0),
                    ("Debt", country.debt.0),
                    ("Reserved cash", country.committed_cash.0),
                    ("Arrears", country.arrears.0),
                ] {
                    ui.label(name);
                    ui.monospace(format!("{value}m"));
                    ui.end_row();
                }
            });
        ui.small(format!(
            "Next command {} · next project {} · {} command records · campaign checksum {:016x}",
            world.next_command,
            world.next_project,
            world.journal.len(),
            world.hash()
        ));
        ui.separator();
        let previous = self.field;
        egui::ComboBox::from_label("Direct scenario field")
            .selected_text(self.field.name())
            .show_ui(ui, |ui| {
                for field in DeveloperField::ALL {
                    ui.selectable_value(&mut self.field, field, field.name());
                }
            });
        if self.target.as_ref() != Some(target) || self.field != previous {
            self.value = self.field.read(country);
            self.target = Some(target.clone());
        }
        ui.label(format!(
            "Current {}: {} · allowed {:?}",
            self.field.name(),
            self.field.read(country),
            self.field.bounds()
        ));
        ui.add(egui::DragValue::new(&mut self.value).range(self.field.bounds()));
        ui.label("Direct edits bypass policy authority, capital costs and delays. Rates/capacities affect subsequent settlement; changing approval sets national and all household approvals. Cash creation is unavailable.");
        let chosen = ui
            .add_enabled(
                can_edit,
                egui::Button::new("Apply developer edit · mark campaign"),
            )
            .clicked()
            .then_some(DeveloperEdit {
                field: self.field,
                value: self.value,
            });
        if !can_edit {
            ui.label("Wait for the active advance to stop before editing.");
        }
        ui.collapsing("Command audit · latest 20", |ui| {
            for entry in world.journal.iter().rev().filter(|entry| &entry.command.issuer == target
                || matches!(&entry.command.action, Action::Developer { target: edited, .. } if edited == target)).take(20) {
                ui.label(format!("{} · #{} · {} · {:?}", date(entry.command.day), entry.command.id,
                    if entry.developer { "DEVELOPER" } else if entry.ai { "AI" } else { "PLAYER" }, entry.command.action));
            }
        });
        ui.collapsing("Private economic ledger · latest 20", |ui| {
            for entry in world
                .ledger
                .iter()
                .rev()
                .filter(|entry| &entry.country == target)
                .take(20)
            {
                ui.label(format!(
                    "{} · {:?} → {:?} · {}m · {}",
                    date(entry.day),
                    entry.from,
                    entry.to,
                    entry.amount.0,
                    entry.reason
                ));
            }
        });
        ui.collapsing("Country events / AI reasoning · latest 20", |ui| {
            for entry in world
                .events
                .iter()
                .rev()
                .filter(|entry| &entry.country == target)
                .take(20)
            {
                if let Some(id) = &entry.id {
                    ui.label(format!("{id} · {} · {}", date(entry.day), entry.text));
                } else {
                    ui.label(format!("{} · {}", date(entry.day), entry.text));
                }
            }
        });
        chosen
    }
}
