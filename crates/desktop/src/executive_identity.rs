use eframe::egui;
use sim_core::*;

/// Called only after the owning-country privacy guard in the Institutions screen.
pub fn show(ui: &mut egui::Ui, content: &Content, world: &World) {
    let view = match world.executive_identity_view(content, &world.player) {
        Ok(Some(view)) => view,
        Ok(None) => return,
        Err(reason) => {
            ui.label(format!("Executive profile unavailable: {reason}"));
            return;
        }
    };
    ui.heading("Executive authority");
    ui.strong(&view.name);
    let kind = match &view.holder {
        ExecutiveHolder::Person { .. } => "Person",
        ExecutiveHolder::Collective { .. } => match &view.body_origin {
            Some(ExecutiveBodyOrigin::Generated { .. }) => "Simulated governing body",
            _ => "Opening reference collective",
        },
        ExecutiveHolder::Vacant => "Vacant office",
        ExecutiveHolder::Unknown => "Incumbent unknown",
    };
    ui.label(format!(
        "{kind} · held since {} · government #{} from {}",
        date(view.holder_since.units()),
        view.government.0,
        date(view.mandate_since.units())
    ));
    if matches!(view.holder, ExecutiveHolder::Collective { .. }) {
        if view.members.is_empty() {
            ui.label("Individual membership is unspecified.");
        } else {
            ui.label(format!(
                "Recorded members: {}",
                view.members
                    .iter()
                    .map(|(_, name)| name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    ui.small("Current powers follow the governing constitution. Dated reference terms do not automatically replace the live executive.");
    egui::CollapsingHeader::new(format!("Executive history ({} retained mandates)", view.tenures.len()))
        .id_salt("executive_history").show(ui, |ui| {
            for entry in view.tenures.iter().rev() {
                let tenure = &entry.tenure;
                ui.label(format!("Government #{} · {} · {} to {}", tenure.government.0, entry.name, date(tenure.from_day.units()), tenure.until_day.map(|day| date(day.units())).unwrap_or_else(|| "present".into())));
            }
            ui.small("Older closed mandates may leave this bounded history. Governing-body identities are retained and never reassigned.");
        });
    egui::CollapsingHeader::new("Executive reference and provenance")
        .id_salt("executive_source")
        .show(ui, |ui| {
            ui.label(format!("Opening reference: {}", view.opening_office_title));
            ui.label(format!(
                "Source {} / {} · {:?}",
                view.source, view.source_version, view.quality
            ));
            if let Some(term) = &view.opening_reference_term {
                ui.small(format!("Opening reference term: {term}"));
            }
            ui.small(format!(
                "Opening office reference: {}",
                view.opening_office.0
            ));
            match &view.holder {
                ExecutiveHolder::Person { person } => {
                    ui.small(format!("Person reference: {}", person.0));
                }
                ExecutiveHolder::Collective { body } => {
                    ui.small(format!("Governing-body reference: {}", body.0));
                }
                _ => {}
            }
        });
    ui.separator();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_show_people_unknown_and_collective_without_mutating_at_both_scales() {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-executive-identities-test.json"
        ))
        .unwrap();
        for scale in [1.0, 1.4] {
            for (country, expected) in [
                ("AUR", "Person · held since"),
                ("BOR", "Incumbent unknown · held since"),
                ("COR", "Opening reference collective · held since"),
            ] {
                let world = World::new(&content, CountryId(country.into()), 17).unwrap();
                let before = world.clone();
                let ctx = egui::Context::default();
                ctx.set_zoom_factor(scale);
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1440.0 / scale, 940.0 / scale),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| show(ui, &content, &world));
                    },
                );
                let labels = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) => Some(text.galley.text()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                assert!(labels.contains(expected), "{labels}");
                assert!(labels.contains("Dated reference terms do not automatically replace"));
                if country == "COR" {
                    assert!(labels.contains("membership is unspecified"));
                }
                assert_eq!(world, before);
            }
        }
    }
}
