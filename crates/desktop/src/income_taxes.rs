use eframe::egui::{self, Ui};
use sim_core::*;
fn ratio(bps: u64) -> String {
    format!("{}.{:02}%", bps / 100, bps % 100)
}
fn percent(bps: u32) -> String {
    ratio(u64::from(bps))
}
fn income_table(
    ui: &mut Ui,
    key: (&str, bool),
    definition: &CountryDefinition,
    rows: &[(&HouseholdGroupId, &IncomeTaxAssessment, Money)],
    last_column: &str,
) {
    egui::ScrollArea::horizontal().id_salt(key).show(ui, |ui| {
        egui::Grid::new((key, "grid")).striped(true).show(ui, |ui| {
            for label in [
                "Cohort",
                "Private gross",
                "Mean / benchmark",
                "Potential",
                last_column,
            ] {
                ui.strong(label);
            }
            ui.end_row();
            for (group, assessment, paid) in rows {
                ui.label(&definition.household(group).expect("cohort").name);
                ui.label(assessment.gross.0.to_string());
                ui.label(ratio(assessment.annualised_reference_bps));
                ui.label(assessment.potential.0.to_string());
                ui.label(paid.0.to_string());
                ui.end_row();
            }
        });
    });
}
/// Called only inside the host's owner-information guard; rendering issues no command.
pub fn show(ui: &mut Ui, content: &Content, world: &World, draft: Option<i32>) -> bool {
    let Some(rules) = &content.income_taxes else {
        return false;
    };
    let c = world.player_country();
    let d = content.country(&world.player).expect("country");
    let mut review = false;
    egui::CollapsingHeader::new("Private-income tax bands & withholding")
        .id_salt(("income-bands", draft.is_some()))
        .show(ui, |ui| {
            let standard = draft.unwrap_or(c.tax);
            ui.label(format!("Operating standard marginal rate {}%. {} Collection capacity {}%.",c.tax,draft.map_or(String::new(),|v|format!("Preview standard {v}%.")),c.administration));
            ui.label("Bands apply progressively to each cohort's representative resident. A higher band taxes only income in that band. This does not estimate income differences within a cohort or reproduce historical filing law.");
            ui.small("Private income is the taxable base. Public/local service payments and welfare are outside this model's base. The benchmark is opening GDP per resident, using fixed 365-day annualisation.");
            egui::Grid::new(("income-band-schedule", draft.is_some())).striped(true).show(ui, |ui| {
                for label in ["Band", "Annual private income / benchmark", "Marginal rate"] { ui.strong(label); }
                ui.end_row();
                for (index, band) in rules.bands.iter().enumerate() {
                    let lower = percent(band.lower_gdp_per_resident_bps);
                    let range = rules.bands.get(index + 1).map_or(format!("{lower} and above"), |next| format!("{lower} to {}", percent(next.lower_gdp_per_resident_bps)));
                    ui.label(&band.id.0);
                    ui.label(range);
                    ui.label(percent(rules.marginal_rate_bps(band, standard).expect("validated band")));
                    ui.end_row();
                }
            });
            ui.strong("Conditional unchanged-supply assessment · m credits/day");
            ui.small("Holds opening GDP, expected private-income funding, current supply and collection capacity fixed. This is a forecast, not spendable treasury cash or an implementation guarantee.");
            match world.preview_income_taxes(content, &world.player, standard) {
                Ok(rows) => {
                    let display: Vec<_> = rows.iter().map(|(group,a)| (group,a,a.collectible)).collect();
                    income_table(ui, ("income-tax-projection", draft.is_some()), d, &display, "Withholding");
                    ui.label(format!("Expected withholding {}m/day. Only actual received private income and cash transfers create tax receipts.", rows.iter().map(|(_,a)|a.collectible.0).sum::<i64>()));
                }
                Err(reason) => { ui.label(reason); }
            }
            if let Some(report) = &c.income_tax_report {
                ui.strong(format!("Realized withholding {} · settled standard {}% · collection capacity {}%", date(report.day), report.standard_rate, report.administration));
                ui.label(format!("Potential {}m; actually withheld {}m. Uncollected potential is retained by households; it is not cash or a treasury arrears claim.", report.potential().0, report.paid().0));
                let display: Vec<_> = report.groups.iter().map(|line| (&line.group, &line.assessment, line.paid)).collect();
                income_table(ui, ("income-tax-actual", draft.is_some()), d, &display, "Paid");
                egui::CollapsingHeader::new("Inspect actual marginal slices").id_salt(("income-slices",draft.is_some())).show(ui,|ui| {
                    ui.small("These annualised group bases are calculation inputs, not cash payments. Capacity is applied before rounding each cohort's daily withholding to whole million-credit units.");
                    for line in &report.groups {
                        let name=&d.household(&line.group).expect("cohort").name;
                        ui.strong(format!("{name}: private gross {}m/day",line.assessment.gross.0));
                        for slice in &line.assessment.slices {
                            ui.label(format!("{}: annualised group basis {}m at marginal {}",slice.band.0,slice.annual_base.0,percent(slice.marginal_rate_bps)));
                        }
                    }
                });
            } else {
                ui.label("Opening snapshot: no realized withholding yet. Advance one day for actual receipts.");
            }
            if draft.is_none() && ui.button("Review the standard income-tax rate").clicked() { review = true; }
            ui.small("Assessments round per cohort in whole million-credit units. Displayed rounded potential does not define cash withholding.");
            ui.small("Authored illustrative bands. The policy and fiscal plan change the standard reference rate through ordinary consent, funding and delivery; individual bands are fixed in this scenario.");
        });
    ui.separator();
    review
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn marginal_rates_and_dated_withholding_are_read_only_at_two_scales() {
        for scale in [1.0, 1.4] {
            let content = Content::from_json(include_str!(
                "../../../data/synthetic-income-bands-test.json"
            ))
            .unwrap();
            let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
            world.advance(&content, sim_core::DaySpan::new(1)).unwrap();
            let hash = world.hash();
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let input = egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1440.0 / scale, 940.0 / scale),
                )),
                ..Default::default()
            };
            let draw = |i: egui::RawInput| {
                ctx.run(i, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        assert!(!show(ui, &content, &world, Some(30)));
                    });
                })
            };
            let output = draw(input.clone());
            let header = output
                .shapes
                .iter()
                .find_map(|s| {
                    if let egui::Shape::Text(t) = &s.shape {
                        (t.galley.text() == "Private-income tax bands & withholding")
                            .then(|| t.pos + t.galley.size() / 2.0)
                    } else {
                        None
                    }
                })
                .expect("worksheet header");
            for pressed in [true, false] {
                let mut i = input.clone();
                i.events = vec![
                    egui::Event::PointerMoved(header),
                    egui::Event::PointerButton {
                        pos: header,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ];
                draw(i);
            }
            let output = draw(input);
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
            assert!(labels.contains("60.00%"));
            assert!(labels.contains("Realized withholding 2000-01-02"));
            assert!(labels.contains("Conditional unchanged-supply assessment"));
            assert!(labels.contains("not cash or a treasury arrears claim"));
            assert!(labels.contains("Mean / benchmark"));
            assert_eq!(world.hash(), hash);
        }
    }
}
