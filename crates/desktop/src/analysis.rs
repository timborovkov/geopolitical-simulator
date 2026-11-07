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
        ui.label("Compare dated public review statistics. Archive series are separately labelled sample means. Private treasury history is available only for your government.");
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
        for id in &ids {
            if let Some(data) = charts::series(world, content, id, self.metric, self.horizon) {
                series.push(data);
                if let Some(archive) =
                    charts::archived_series(world, content, id, self.metric, self.horizon)
                {
                    series.push(archive);
                }
            } else {
                ui.label(format!(
                    "{}: no permission to inspect treasury history",
                    id.0
                ));
            }
        }
        self.chart.show(ui, self.metric, &series);
        if world.rules >= 25 {
            egui::CollapsingHeader::new("Archived review sample coverage").show(ui, |ui| {
                ui.small("Each bucket summarizes retired review samples from one calendar year. The oldest retained year can be partial. These means do not reconstruct daily history or annual accounts.");
                for id in &ids {
                    if self.metric == Metric::Treasury && id != &world.player { continue; }
                    if let Ok(report) = world.public_report(content, id) {
                        for summary in report.archived_reviews.unwrap_or_default() {
                            if summary.last_day.units().saturating_add(self.horizon) >= world.day.units() {
                                ui.label(format!("{} · {}: {} review sample{}, {} through {}", report.name, summary.year, summary.samples, if summary.samples == 1 { "" } else { "s" }, sim_core::date(summary.first_day), sim_core::date(summary.last_day)));
                            }
                        }
                    }
                }
            });
        }

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
        ui.small("Opening GDP is a scenario calibration with provenance in the overview; current debt/approval/needs are published simulation results. Unknown observations sort last in either direction. Recent review history retains 120 samples; archive-enabled campaigns retain older calendar-year sample summaries.");
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn archived_means_and_coverage_are_inspectable_without_foreign_cash_or_orders_at_two_scales() {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-statistical-history-test.json"
        ))
        .unwrap();
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        world
            .advance(&content, sim_core::DaySpan::new(3660))
            .unwrap();
        world
            .advance(&content, sim_core::DaySpan::new(600))
            .unwrap();
        let before = sim_core::persistence::encode(&world, &content).unwrap();
        assert!(
            charts::archived_series(
                &world,
                &content,
                &CountryId("BOR".into()),
                Metric::Treasury,
                36525
            )
            .is_none()
        );
        assert!(
            charts::archived_series(&world, &content, &world.player, Metric::Treasury, 360)
                .is_none()
        );
        let own = charts::archived_series(&world, &content, &world.player, Metric::Treasury, 36525)
            .unwrap();
        assert!(own.label.contains("archived review-sample means"));
        assert!(!own.points.is_empty());
        for scale in [1., 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut panel = Analysis {
                metric: Metric::Treasury,
                horizon: 36525,
                pinned: vec![CountryId("BOR".into())],
                ..Default::default()
            };
            let mut selected = world.player.clone();
            let mut frame = |events, offset| {
                ctx.run(
                    egui::RawInput {
                        focused: true,
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1440. / scale, 940. / scale),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::SidePanel::left("test_sidebar")
                            .exact_width(230.)
                            .show(ctx, |_| {});
                        egui::CentralPanel::default().show(ctx, |ui| {
                            egui::ScrollArea::vertical()
                                .vertical_scroll_offset(offset)
                                .show(ui, |ui| {
                                    panel.show(ui, &content, Some(&world), &mut selected)
                                });
                        });
                    },
                )
            };
            let mut found = None;
            for offset in [0., 160., 320.] {
                frame(vec![], offset);
                let output = frame(vec![], offset);
                found = output.shapes.iter().find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text)
                        if text.galley.text() == "Archived review sample coverage" =>
                    {
                        let p = text.pos + text.galley.size() / 2.;
                        shape.clip_rect.contains(p).then_some((p, offset))
                    }
                    _ => None,
                });
                if found.is_some() {
                    break;
                }
            }
            let (point, offset) = found.expect("Visible archive coverage control");
            for pressed in [true, false] {
                frame(
                    vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    offset,
                );
            }
            let output = frame(vec![], offset);
            assert!(output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text().contains("review samples,"))));
            assert_eq!(selected, world.player);
            assert_eq!(
                sim_core::persistence::encode(&world, &content).unwrap(),
                before
            );
        }
    }
    #[test]
    fn unknown_is_not_sorted_as_zero_in_either_direction() {
        let c = Content::bundled().unwrap();
        let mut w = World::new(&c, CountryId("GBR".into()), 2).unwrap();
        let unknown = w.public_report(&c, &w.player).unwrap();
        w.advance(&c, sim_core::DaySpan::new(30)).unwrap();
        let known = w.public_report(&c, &w.player).unwrap();
        for ascending in [true, false] {
            assert_eq!(
                compare(&unknown, &known, Sort::Debt, ascending),
                std::cmp::Ordering::Greater
            );
        }
    }
}
