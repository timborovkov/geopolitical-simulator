use eframe::egui;
use sim_core::*;

pub fn show(ui: &mut egui::Ui, content: &Content, world: &World, id: &CountryId, viewer: Viewer) {
    let Ok(Some(report)) = world.demographic_report(content, viewer, id) else {
        return;
    };
    ui.heading("Population change & measurement");
    if let Some(latest) = report.published.last() {
        ui.label(format!("Published {}: {} residents · rounding interval {}–{} · resolution {} people · statistical capacity {}%", date(latest.day), latest.population.0, latest.lower.0, latest.upper.0, latest.resolution.0, latest.statistical_capacity));
        ui.small("This dated rounding interval is an authored measurement limit, not statistical confidence or a guarantee about today's population. Charts show published estimates only.");
    } else {
        ui.label("No population measurement has been published. The opening reference remains available; unknown is not zero residents.");
    }
    if let Some(groups) = report.owned_groups {
        let total = |stock: fn(&OwnedDemographicGroup) -> u64| {
            groups.iter().map(|g| u128::from(stock(g))).sum::<u128>()
        };
        ui.strong(format!(
            "Owned stocks on {} · young 0–14: {} · working age 15–64: {} · older 65+: {}",
            date(report.as_of),
            total(|g| g.ages.young.0),
            total(|g| g.ages.working_age.0),
            total(|g| g.ages.older.0)
        ));
        ui.label(format!("Aggregate household units: {}. These count families/households, not dwellings, assets or additional residents.", total(|g| g.households.0)));
        if let Some(next) = report.next_attempt {
            ui.label(format!("Next census attempt: {}. Actual administration at the attempt determines precision and the following interval; zero capacity publishes no observation.", date(next)));
        }
        for group in groups {
            let name = content
                .country(id)
                .and_then(|country| country.household(&group.group))
                .map_or(group.group.0.as_str(), |group| group.name.as_str());
            egui::CollapsingHeader::new(format!("{name} · age stocks and cumulative flows"))
                .id_salt(("demographic-group", &group.group.0))
                .show(ui, |ui| {
                    egui::Grid::new(("demographic-balance", &group.group.0))
                        .num_columns(2)
                        .show(ui, |ui| {
                            for (label, value) in [
                                (
                                    "Young / working age / older",
                                    format!(
                                        "{} / {} / {}",
                                        group.ages.young.0,
                                        group.ages.working_age.0,
                                        group.ages.older.0
                                    ),
                                ),
                                ("Births since opening", group.flows.births.0.to_string()),
                                (
                                    "Deaths: young / working age / older",
                                    format!(
                                        "{} / {} / {}",
                                        group.flows.deaths.young.0,
                                        group.flows.deaths.working_age.0,
                                        group.flows.deaths.older.0
                                    ),
                                ),
                                (
                                    "Aged into working / older bands",
                                    format!(
                                        "{} / {}",
                                        group.flows.into_working_age.0,
                                        group.flows.into_older_age.0
                                    ),
                                ),
                                (
                                    "Arriving / departing residents",
                                    format!(
                                        "{} / {}",
                                        group.flows.arrivals.total().expect("validated ages").0,
                                        group.flows.departures.total().expect("validated ages").0
                                    ),
                                ),
                                ("Household units now", group.households.0.to_string()),
                                (
                                    "Households formed / dissolved",
                                    format!(
                                        "{} / {}",
                                        group.flows.households_formed.0,
                                        group.flows.households_dissolved.0
                                    ),
                                ),
                                (
                                    "Households arriving / departing",
                                    format!(
                                        "{} / {}",
                                        group.flows.arriving_households.0,
                                        group.flows.departing_households.0
                                    ),
                                ),
                            ] {
                                ui.label(label);
                                ui.label(value);
                                ui.end_row();
                            }
                        });
                });
        }
        ui.small("Working age is not employment. Existing opening job capacity and paid income constrain modeled employment; births and migration create no jobs, factories or cash. Group savings and arrears remain pooled accounts. Construction still needs normal funding and delivery.");
    }
    ui.small(format!("Authored demographic calibration: {} / {}. Coarse age bands and fixed transition assumptions are not historical national vital statistics or migration estimates. Regional placement, housing, individual labor mobility and endogenous fertility remain unfinished.", report.source, report.source_version));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn labels(output: egui::FullOutput) -> String {
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(t) => Some(t.galley.text()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    #[test]
    fn owned_population_balances_and_dated_public_estimates_stay_separate_at_two_scales() {
        let content = Content::demographic_bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        world.advance(&content, DaySpan::new(20)).unwrap();
        let before = world.clone();
        for scale in [1., 1.4] {
            for viewer in [
                Viewer::Government(world.player.clone()),
                Viewer::Observer,
                Viewer::Government(CountryId("USA".into())),
            ] {
                let owner = viewer == Viewer::Government(world.player.clone());
                let ctx = egui::Context::default();
                ctx.set_zoom_factor(scale);
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1440. / scale, 1000. / scale),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            show(ui, &content, &world, &world.player, viewer.clone())
                        });
                    },
                );
                let text = labels(output);
                assert!(text.contains("Published 2000-01-01"));
                assert!(text.contains("rounding interval"));
                assert_eq!(text.contains("Owned stocks on 2000-01-21"), owner);
                assert_eq!(text.contains("Next census attempt"), owner);
                assert_eq!(text.contains("age stocks and cumulative flows"), owner);
            }
        }
        assert_eq!(world, before);
    }
    #[test]
    fn population_chart_uses_published_dates_and_values_without_filling_unknown_history() {
        let content = Content::demographic_bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        world.advance(&content, DaySpan::new(20)).unwrap();
        for id in [world.player.clone(), CountryId("USA".into())] {
            let series = super::super::charts::series(
                &world,
                &content,
                &id,
                super::super::charts::Metric::Population,
                365,
            )
            .unwrap();
            let reports = world
                .demographic_report(&content, Viewer::Observer, &id)
                .unwrap()
                .unwrap();
            assert_eq!(
                series.points,
                reports
                    .published
                    .iter()
                    .map(|p| (p.day.units(), p.population.0 as f64))
                    .collect::<Vec<_>>()
            );
            assert_eq!(series.points.len(), 1);
            let bounds = super::super::charts::population_intervals(&world, &content, &id, 365);
            assert_eq!(bounds.len(), 2);
            for (bound, upper) in bounds.iter().zip([false, true]) {
                assert_eq!(
                    bound.points,
                    reports
                        .published
                        .iter()
                        .map(|p| (
                            p.day.units(),
                            if upper { p.upper.0 } else { p.lower.0 } as f64
                        ))
                        .collect::<Vec<_>>()
                );
            }
            assert_ne!(series.points[0].1, world.countries[&id].population.0 as f64);
            assert!(
                super::super::charts::archived_series(
                    &world,
                    &content,
                    &id,
                    super::super::charts::Metric::Population,
                    365
                )
                .is_none()
            );
        }
        let old = Content::bundled().unwrap();
        let old_world = World::new(&old, world.player.clone(), 17).unwrap();
        assert!(
            super::super::charts::series(
                &old_world,
                &old,
                &world.player,
                super::super::charts::Metric::Population,
                365
            )
            .is_none()
        );
    }
}
