//! Public reference inspector, available before play and for foreign countries.
use eframe::egui::Ui;
use sim_core::*;

pub fn show(ui: &mut Ui, content: &Content, country: &CountryId) {
    let Ok(Some(report)) = content.bookmark_sovereignty(country) else {
        return;
    };
    ui.collapsing(
        format!("Sovereignty & recognition reference · {}", report.bookmark),
        |ui| body(ui, content, &report),
    );
}
fn territory(ui: &mut Ui, row: &PublishedTerritoryReference) {
    ui.strong(format!("{} · {}", row.territory.name, row.territory.id.0));
    ui.small(format!(
        "Territory definition: {:?} · {}@{}",
        row.territory.quality, row.territory.source, row.territory.source_version
    ));
    if let Some(r) = &row.reference {
        ui.label(format!("{} · {}", r.sovereignty.label(), r.control.label()));
        provenance(ui, &r.validity, r.quality, &r.source, &r.source_version);
    } else {
        ui.label("No reference at this date: sovereignty and control unknown.");
    }
}
fn provenance(
    ui: &mut Ui,
    validity: &Validity,
    quality: RecordQuality,
    source: &str,
    version: &str,
) {
    ui.small(format!(
        "{} until {} (end excluded) · {quality:?} · {source}@{version}",
        validity.valid_from,
        validity
            .valid_to
            .map_or_else(|| "open-ended".into(), |d| d.to_string())
    ));
}
fn body(ui: &mut Ui, content: &Content, report: &BookmarkSovereigntyReport) {
    ui.label("Bookmark references only. Live government and regime are separate. Future reference records do not force campaign transfers or recognition changes.");
    ui.label("Map outlines are display references; they do not establish sovereignty, control or recognition. These records grant no revenue, military powers or territorial orders.");
    if report.territories.is_empty() {
        ui.label("No territory reference links this country at the bookmark. This does not establish absence of territory.");
    }
    for row in &report.territories {
        territory(ui, row);
    }
    ui.strong("Directional recognition references");
    for r in &report.recognition {
        ui.label(format!(
            "{} about {}: {}",
            r.observer.0,
            r.subject.0,
            r.position.label()
        ));
        provenance(ui, &r.validity, r.quality, &r.source, &r.source_version);
    }
    ui.small("Unlisted recognition pairs are unknown. Recognition does not confer sovereignty or control.");
    ui.collapsing(
        "All scenario reference territories, including unknown records",
        |ui| {
            if let Some(catalogue) = &content.sovereignty {
                for row in catalogue.territories_at(report.bookmark) {
                    territory(ui, &row);
                }
            }
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;
    #[test]
    fn public_reference_distinguishes_occupation_claims_unknowns_and_bookmark_at_two_scales() {
        let c = Content::from_json(include_str!(
            "../../../data/synthetic-sovereignty-test.json"
        ))
        .unwrap();
        let report = c
            .bookmark_sovereignty(&CountryId("AUR".into()))
            .unwrap()
            .unwrap();
        let w = World::new(&c, CountryId("BOR".into()), 17).unwrap();
        let before = w.hash();
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
                    egui::CentralPanel::default().show(ctx, |ui| {
                        body(ui, &c, &report);
                        territory(
                            ui,
                            &c.sovereignty
                                .as_ref()
                                .unwrap()
                                .territories_at(report.bookmark)[4],
                        );
                    });
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
            for text in [
                "Sovereignty recorded for AUR",
                "Occupation recorded: BOR",
                "BOR about AUR: Not recognized",
                "Sovereignty unknown",
                "Control unknown",
                "Bookmark references only",
                "Future reference records",
                "authored-sovereignty-reference-v1",
            ] {
                assert!(labels.contains(text), "missing {text} at {scale}");
            }
        }
        assert_eq!(w.hash(), before);
    }
}
