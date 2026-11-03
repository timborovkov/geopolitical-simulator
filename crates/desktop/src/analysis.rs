use crate::charts::{self, Chart, Metric};
use eframe::egui;
use sim_core::{Content, CountryId, PublicCountryReport, World};
#[derive(Clone, Copy, PartialEq)]
enum Sort {
    Name,
    Gdp,
    Debt,
    Approval,
    Needs,
}
fn value(report: &PublicCountryReport, sort: Sort) -> Option<i64> {
    match sort {
        Sort::Name => None,
        Sort::Gdp => Some(report.opening_gdp.0),
        Sort::Debt => report
            .latest
            .as_ref()
            .map(|o| o.debt.0 * 100 / report.opening_gdp.0),
        Sort::Approval => report.latest.as_ref().map(|o| i64::from(o.approval)),
        Sort::Needs => report.latest.as_ref().map(|o| i64::from(o.needs)),
    }
}
fn compare(
    a: &PublicCountryReport,
    b: &PublicCountryReport,
    sort: Sort,
    ascending: bool,
) -> std::cmp::Ordering {
    if sort == Sort::Name {
        return if ascending {
            a.name.cmp(&b.name)
        } else {
            b.name.cmp(&a.name)
        };
    }
    match (value(a, sort), value(b, sort)) {
        (Some(a), Some(b)) => {
            if ascending {
                a.cmp(&b)
            } else {
                b.cmp(&a)
            }
        }
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, None) => a.name.cmp(&b.name),
    }
}
pub struct Analysis {
    metric: Metric,
    horizon: u32,
    pinned: Vec<CountryId>,
    sort: Sort,
    ascending: bool,
    chart: Chart,
}
impl Default for Analysis {
    fn default() -> Self {
        Self {
            metric: Metric::Approval,
            horizon: 1080,
            pinned: Vec::new(),
            sort: Sort::Gdp,
            ascending: false,
            chart: Chart::default(),
        }
    }
}
impl Analysis {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        content: &Content,
        world: Option<&World>,
        selected: &mut CountryId,
    ) {
        ui.heading("Trends & country comparisons");
        ui.label("Compare published observations at their actual release dates. Private treasury history is available only for your government.");
        let Some(world) = world else {
            ui.label("Begin a campaign to collect observations. Countries without a release remain unknown.");
            return;
        };
        ui.horizontal_wrapped(|ui| {
            if ui.button(format!("Pin {}", selected.0)).clicked() && !self.pinned.contains(selected)
            {
                if self.pinned.len() == 3 {
                    self.pinned.remove(0);
                }
                self.pinned.push(selected.clone());
            }
            if ui.button("Clear comparisons").clicked() {
                self.pinned.clear();
            }
            for id in self.pinned.clone() {
                if ui.button(format!("{} ×", id.0)).clicked() {
                    self.pinned.retain(|p| p != &id);
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_label("Metric")
                .selected_text(self.metric.label())
                .show_ui(ui, |ui| {
                    for metric in [
                        Metric::Approval,
                        Metric::Needs,
                        Metric::Debt,
                        Metric::Output,
                        Metric::Treasury,
                    ] {
                        ui.selectable_value(&mut self.metric, metric, metric.label());
                    }
                });
            egui::ComboBox::from_label("History")
                .selected_text(match self.horizon {
                    360 => "Last year",
                    1080 => "Last three years",
                    _ => "All retained",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.horizon, 360, "Last year");
                    ui.selectable_value(&mut self.horizon, 1080, "Last three years");
                    ui.selectable_value(&mut self.horizon, 36525, "All retained");
                });
        });
        let mut ids = vec![selected.clone()];
        for id in &self.pinned {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        let mut series = Vec::new();
        for id in ids {
            if let Some(data) = charts::series(world, content, &id, self.metric, self.horizon) {
                series.push(data);
            } else {
                ui.label(format!(
                    "{}: no permission to inspect treasury history",
                    id.0
                ));
            }
        }
        self.chart.show(ui, self.metric, &series);
        ui.separator();
        ui.heading("Country comparison · click a heading to sort");
        let mut reports: Vec<_> = content
            .countries
            .iter()
            .filter_map(|c| world.public_report(content, &c.id).ok())
            .collect();
        reports.sort_by(|a, b| {
            compare(a, b, self.sort, self.ascending).then_with(|| a.country.cmp(&b.country))
        });
        egui::ScrollArea::horizontal()
            .id_salt("comparison_table")
            .show(ui, |ui| {
                egui::Grid::new("comparison_rows")
                    .striped(true)
                    .min_col_width(90.0)
                    .show(ui, |ui| {
                        for (sort, label) in [
                            (Sort::Name, "Country"),
                            (Sort::Gdp, "Opening GDP m/year"),
                            (Sort::Debt, "Debt/GDP %"),
                            (Sort::Approval, "Approval %"),
                            (Sort::Needs, "Needs met %"),
                        ] {
                            let marker = if self.sort == sort {
                                if self.ascending { " (asc)" } else { " (desc)" }
                            } else {
                                ""
                            };
                            if ui.button(format!("{label}{marker}")).clicked() {
                                if self.sort == sort {
                                    self.ascending = !self.ascending;
                                } else {
                                    self.sort = sort;
                                    self.ascending = sort == Sort::Name;
                                }
                            }
                        }
                        ui.strong("Published");
                        ui.end_row();
                        for report in reports {
                            if ui
                                .selectable_label(report.country == *selected, &report.name)
                                .clicked()
                            {
                                *selected = report.country.clone();
                            }
                            ui.label(report.opening_gdp.0.to_string());
                            for sort in [Sort::Debt, Sort::Approval, Sort::Needs] {
                                ui.label(
                                    value(&report, sort)
                                        .map_or_else(|| "Unknown".into(), |v| v.to_string()),
                                );
                            }
                            ui.label(
                                report
                                    .latest
                                    .map_or_else(|| "No release".into(), |o| sim_core::date(o.day)),
                            );
                            ui.end_row();
                        }
                    });
            });
        ui.small("Opening GDP is a scenario calibration with provenance in the overview; current debt/approval/needs are published simulation results. Unknown observations sort last in either direction. Monthly history retains up to ten years.");
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_is_not_sorted_as_zero_in_either_direction() {
        let c = Content::bundled().unwrap();
        let mut w = World::new(&c, CountryId("GBR".into()), 2).unwrap();
        let unknown = w.public_report(&c, &w.player).unwrap();
        w.advance(&c, 30).unwrap();
        let known = w.public_report(&c, &w.player).unwrap();
        for ascending in [true, false] {
            assert_eq!(
                compare(&unknown, &known, Sort::Debt, ascending),
                std::cmp::Ordering::Greater
            );
        }
    }
}
