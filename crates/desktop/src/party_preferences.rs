//! Public authored positions are separate from the owner's operating-policy report.
use eframe::egui;
use sim_core::*;

pub fn show_reference(ui: &mut egui::Ui, content: &Content, country: &CountryId) {
    if content.party_preferences.is_none() {
        return;
    }
    ui.collapsing("Party economic policy positions", |ui| {
        reference_body(ui, content, country)
    });
}
fn reference_body(ui: &mut egui::Ui, content: &Content, country: &CountryId) {
    let Some(rules) = &content.party_preferences else {
        return;
    };
    let Some(definition) = content.country(country) else {
        return;
    };
    ui.label("Scenario policy positions are independent of ideology labels. This initial model covers standard income tax, public-service spending and welfare; other preference dimensions remain unknown.");
    if let Some(rule) = rules.influences.iter().find(|r| &r.country == country) {
        ui.label(format!("Maximum target reductions: legislative coalition {} points; ruling-party leadership {} points. Other governing routes keep their independent backing rules.", rule.legislative_max_penalty, rule.party_leadership_max_penalty));
    }
    ui.small(format!(
        "Influence model: {}@{}",
        rules.source, rules.source_version
    ));
    for profile in rules.profiles.iter().filter(|p| &p.country == country) {
        let Some(party) = definition.party(&profile.party) else {
            continue;
        };
        ui.strong(format!("{} [{}]", party.name, profile.party.0));
        ui.small(format!(
            "Positions: {:?} · {}@{}",
            profile.quality, profile.source, profile.source_version
        ));
        for position in &profile.positions {
            let name = content
                .policy(&position.policy)
                .map_or(position.policy.as_str(), |p| p.name.as_str());
            let unit = if position.policy == "income_tax" {
                "% standard rate"
            } else {
                "% opening GDP"
            };
            ui.label(match &position.preferred {
                Some(range) => format!(
                    "{name}: preferred {}..{} {unit} · relative importance {}",
                    range.min, range.max, position.importance
                ),
                None => format!(
                    "{name}: Unknown · relative importance {}",
                    position.importance
                ),
            });
        }
    }
    ui.small("Unknown positions create no modeled pressure; coverage remains explicit. These are target adjustments, not roll-call votes or extra legal powers.");
}
pub fn show_owned(ui: &mut egui::Ui, content: &Content, world: &World) {
    if content.party_preferences.is_none() {
        return;
    }
    match world.party_alignment_for(
        content,
        &world.player,
        &FiscalTargets::current(world.player_country()),
    ) {
        Ok(Some(report)) => alignment_body(ui, content, &report),
        Ok(None) => {}
        Err(reason) => {
            ui.label(format!("Party alignment unavailable: {reason}"));
        }
    }
}
fn alignment_body(ui: &mut egui::Ui, content: &Content, report: &PartyAlignmentReport) {
    ui.strong("Operating party alignment");
    if let Some(metric) = report.metric {
        ui.label(format!(
            "{} target reduction: {} points of a maximum {}.",
            metric.label(),
            report.penalty,
            report.max_penalty
        ));
        ui.label(format!(
            "Known position coverage: {}.{:02}%",
            report.known_positions_bps.units() / 100,
            report.known_positions_bps.units() % 100
        ));
        ui.small("Only operating policies affect this reduction. Held backing moves at monthly political reviews alongside opinion, funding and institutional effects. Coalition-seat weights apply to legislatures; party leadership uses the ruling party.");
        for row in &report.parties {
            let name = content
                .country(&report.country)
                .and_then(|c| c.party(&row.party))
                .map_or(row.party.0.as_str(), |p| p.name.as_str());
            ui.label(format!(
                "{name}: sponsor weight {} · {}",
                row.weight,
                row.distance_bps.map_or_else(
                    || "policy positions unknown".into(),
                    |distance| format!(
                        "policy distance {}.{:02}%",
                        distance.units() / 100,
                        distance.units() % 100
                    )
                )
            ));
        }
    } else {
        ui.label("Party positions are advisory for this governing route. Council and executive backing remain separate.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_reference_and_owned_alignment_are_read_only_and_keep_unknowns_at_two_scales() {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-party-preferences-test.json"
        ))
        .unwrap();
        let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        let before = world.hash();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            for id in ["AUR", "BOR", "COR"] {
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1280.0 / scale, 1000.0 / scale),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            reference_body(ui, &content, &CountryId(id.into()))
                        });
                    },
                );
                let text: String = output
                    .shapes
                    .iter()
                    .filter_map(|s| {
                        if let egui::Shape::Text(t) = &s.shape {
                            Some(t.galley.text())
                        } else {
                            None
                        }
                    })
                    .collect();
                assert!(text.contains("Influence model:"));
                assert!(text.contains("relative importance"));
                if id == "BOR" {
                    assert!(text.contains("Unknown"));
                }
                for private in [
                    "Treasury",
                    "Household cash",
                    "AI review",
                    "Operating party alignment",
                ] {
                    assert!(!text.contains(private));
                }
            }
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1280.0 / scale, 1000.0 / scale),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| show_owned(ui, &content, &world));
                },
            );
            assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text().contains("target reduction: 0 points"))));
        }
        assert_eq!(world.hash(), before);
    }
}
