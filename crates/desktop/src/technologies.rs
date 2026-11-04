use eframe::egui::{Color32, Ui};
use sim_core::*;

/// The policy workspace has already enforced its controlled-government view.
pub fn show(ui: &mut Ui, content: &Content, world: &World, policy: &str) {
    if content.technologies.is_none() {
        return;
    }
    ui.strong("Technology prerequisites");
    match world.policy_technology_eligibility(content, &world.player, policy) {
        Ok(requirements) => {
            if requirements.is_empty() {
                ui.label("No technology prerequisite for this programme in the loaded scenario.");
            }
            for requirement in requirements {
                let status = if !requirement.country_eligible {
                    "Not eligible for this country in the loaded scenario".into()
                } else if !requirement.date_eligible {
                    format!(
                        "Available for commissioning from {}",
                        requirement.available_from
                    )
                } else {
                    "Available for commissioning".into()
                };
                ui.colored_label(
                    if requirement.available() {
                        Color32::LIGHT_GREEN
                    } else {
                        Color32::LIGHT_YELLOW
                    },
                    format!("{} · {status}", requirement.name),
                );
            }
            ui.small("Eligibility grants no cash or capacity. Governing consent, full funding and implementation are still required. Dates are scenario restrictions; research and adoption are separate.");
        }
        Err(error) => {
            ui.colored_label(Color32::LIGHT_YELLOW, error);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;
    #[test]
    fn policy_prerequisites_distinguish_date_availability_and_create_no_orders_at_two_scales() {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-technology-eligibility-test.json"
        ))
        .unwrap();
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        for day in [0, 30] {
            if day > world.day {
                world.advance(&content, day - world.day).unwrap();
            }
            let hash = world.hash();
            for scale in [1.0, 1.4] {
                let ctx = egui::Context::default();
                ctx.set_zoom_factor(scale);
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1200.0 / scale, 900.0 / scale),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default()
                            .show(ctx, |ui| show(ui, &content, &world, "energy"));
                    },
                );
                let labels = output
                    .shapes
                    .iter()
                    .filter_map(|s| {
                        if let egui::Shape::Text(t) = &s.shape {
                            Some(t.galley.text())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                assert!(labels.contains("Grid planning"));
                assert!(labels.contains("Eligibility grants no cash or capacity"));
                if day == 0 {
                    assert!(labels.contains("from 2000-01-31"));
                } else {
                    assert!(!labels.contains("from 2000-01-31"));
                    assert!(labels.contains("Available for commissioning"));
                }
            }
            assert_eq!(world.hash(), hash);
        }
    }
}
