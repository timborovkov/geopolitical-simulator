use eframe::egui::{self, Color32};
use sim_core::*;

pub fn show(ui: &mut egui::Ui, content: &Content, world: &World, country: &CountryId) {
    let Some(definition) = &content.private_investment else {
        return;
    };
    egui::CollapsingHeader::new("Private equipment & investment")
        .default_open(true)
        .show(ui, |ui| {
            let state = match world.private_investment_report(content, Viewer::Government(world.player.clone()), country) {
                Ok(Some(state)) => state,
                Ok(None) => { ui.label("Current equipment and investment decisions are private to this government."); return; },
                Err(error) => { ui.colored_label(Color32::LIGHT_YELLOW, error); return; },
            };
            ui.label("Businesses decide whether to invest. Government programmes, public equipment and public material stocks remain separate.");
            let totals = match state.totals() { Ok(t) => t, Err(error) => {ui.label(error); return;} };
            ui.label(format!("Paid equipment {}m · accumulated depreciation {}m · net fixed assets {}m", totals.acquisitions.0, totals.depreciation.0, totals.net_assets.0));
            ui.label(format!("Operating lots: food {} · energy {}. Each lot adds up to {} opening-basis capacity points, subject to paid-factor readiness, storage and shocks.", state.active_count(GoodKind::Food),state.active_count(GoodKind::Energy),definition.capacity_points));
            if let Some(review) = &state.last_review {
                ui.strong(format!("{} review · {}", date(review.day), review.reason.explanation()));
                ui.label(format!("Observed imports: food {} units / {}m · energy {} units / {}m",review.observed.food,review.observed.food_cost.0,review.observed.energy,review.observed.energy_cost.0));
                let candidate=match review.good {Some(GoodKind::Food)=>"Food",Some(GoodKind::Energy)=>"Energy",None=>"No eligible equipment"};
                ui.label(format!("Candidate {candidate} · equipment price {}m · conditional daily import saving {}m",review.price.0,review.expected_daily_saving.0));
                ui.label(format!("Retained earnings after dividends {}m · cash available after reserves {}m", review.retained_earnings.0,review.available_cash.0));
                ui.label(format!("Protected recent payments {}m + unfinished client advances {}m",review.payment_buffer.0,review.client_advances.0));
            } else { ui.label("First investment review follows 30 elapsed days of actual operation."); }
            ui.collapsing("Equipment receipts & service life", |ui| {
                for equipment in state.equipment.iter().rev() {
                    ui.label(format!("#{} · {:?} · paid {}m on {} · operates {} to {}",equipment.id.0,equipment.good,equipment.cost.0,date(equipment.acquired),date(equipment.commissioned),date(equipment.retired)));
                }
                if state.equipment.is_empty() { ui.label("No private equipment purchased. Opening private equipment is explicitly zero."); }
            });
            ui.small(format!("Authored equipment market: at most one paid lot per country per 30-day review, {} days to install and {} days of service. Straight-line depreciation creates no cash. Expected savings use the latest actual import receipt and must cover the price within {} days; future demand and prices can change.", definition.installation_days.units(),definition.lifetime_days.units(),definition.maximum_payback_days.units()));
            ui.small("Private equipment does not create another payroll, jobs or GDP. Full corporate finance, taxation, mergers and formal insolvency remain unfinished.");
            ui.small(format!("Assumptions: {} · {}",definition.source,definition.source_version));
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equipment_decisions_and_foreign_privacy_render_at_both_scales() {
        let c = Content::private_investment_bundled().unwrap();
        let mut w = World::new(&c, CountryId("GBR".into()), 17).unwrap();
        w.advance(&c, DaySpan::new(30)).unwrap();
        let before = persistence::encode(&w, &c).unwrap();
        for scale in [1., 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            for id in [&w.player, &CountryId("USA".into())] {
                let mut text = String::new();
                for _ in 0..3 {
                    let output = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(1280., 1000.),
                            )),
                            ..Default::default()
                        },
                        |ctx| {
                            egui::CentralPanel::default().show(ctx, |ui| show(ui, &c, &w, id));
                        },
                    );
                    text = output
                        .shapes
                        .iter()
                        .filter_map(|s| match &s.shape {
                            egui::Shape::Text(t) => Some(t.galley.text()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                }
                if id == &w.player {
                    for label in [
                        "Paid equipment",
                        "Operating lots",
                        "review",
                        "cash available after reserves",
                        "Protected recent payments",
                    ] {
                        assert!(text.contains(label), "{label}: {text}");
                    }
                } else {
                    assert!(text.contains("private to this government"));
                    assert!(!text.contains("Paid equipment"));
                    assert!(!text.contains("cash available"));
                }
            }
        }
        assert_eq!(persistence::encode(&w, &c).unwrap(), before);
    }
}
