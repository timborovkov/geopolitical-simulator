use eframe::egui::{self, Align2, Color32, FontId, Pos2, Stroke, Vec2};
use sim_core::*;

const CURVE: Color32 = Color32::from_rgb(104, 224, 186);
const GRID: Color32 = Color32::from_rgb(100, 120, 130);

pub fn show(ui: &mut egui::Ui, content: &Content, world: &World, country: &CountryId) {
    if country != &world.player {
        return;
    }
    egui::CollapsingHeader::new("Income distribution · cohort means")
        .id_salt(("income_distribution", &country.0))
        .show(ui, |ui| {
            if world.content_hash != content.hash() {
                ui.label("Income distribution requires the campaign's exact content.");
                return;
            }
            match group_income_distribution(&world.player_country().households) {
                Ok(Some(distribution)) => body(ui, content, country, world.day, &distribution),
                Ok(None) => { ui.label("Income shares are undefined when total disposable income is zero, a group has negative disposable income or income belongs to an empty group."); }
                Err(error) => { ui.colored_label(Color32::LIGHT_YELLOW, error); }
            }
        });
}

fn name<'a>(content: &'a Content, country: &CountryId, group: &'a HouseholdGroupId) -> &'a str {
    content
        .country(country)
        .and_then(|c| c.households.iter().find(|h| &h.id == group))
        .map_or(group.0.as_str(), |definition| definition.name.as_str())
}

fn body(
    ui: &mut egui::Ui,
    content: &Content,
    country: &CountryId,
    day: CampaignDay,
    distribution: &GroupIncomeDistribution,
) {
    ui.strong(format!(
        "{} · realized disposable income · group-mean Gini {:.3}",
        date(day),
        f64::from(distribution.group_mean_gini_bps.units()) / 10000.0
    ));
    ui.label("Cumulative population from lowest to highest mean income per resident, against cumulative income after tax. Marked curve: cohort means. Diagonal: equal mean income per resident.");
    let (outer, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().min(720.0), 250.0),
        egui::Sense::hover(),
    );
    let rect = outer.shrink2(Vec2::new(35.0, 25.0));
    let position = |population: u32, income: u32| {
        Pos2::new(
            rect.left() + rect.width() * population as f32 / 10000.0,
            rect.bottom() - rect.height() * income as f32 / 10000.0,
        )
    };
    for tick in [0, 5000, 10000] {
        let vertical = position(tick, 0);
        let horizontal = position(0, tick);
        ui.painter().line_segment(
            [vertical, position(tick, 10000)],
            Stroke::new(0.5_f32, GRID),
        );
        ui.painter().line_segment(
            [horizontal, position(10000, tick)],
            Stroke::new(0.5_f32, GRID),
        );
        ui.painter().text(
            vertical + Vec2::new(0.0, 5.0),
            Align2::CENTER_TOP,
            format!("{}%", tick / 100),
            FontId::proportional(11.0),
            GRID,
        );
        ui.painter().text(
            horizontal - Vec2::new(5.0, 0.0),
            Align2::RIGHT_CENTER,
            format!("{}%", tick / 100),
            FontId::proportional(11.0),
            GRID,
        );
    }
    ui.painter().line_segment(
        [position(0, 0), position(10000, 10000)],
        Stroke::new(1.0_f32, Color32::LIGHT_GRAY),
    );
    let mut points = vec![position(0, 0)];
    points.extend(
        distribution
            .boundaries
            .iter()
            .map(|point| position(point.population_share.units(), point.income_share.units())),
    );
    ui.painter().add(egui::Shape::line(
        points.clone(),
        Stroke::new(2.5_f32, CURVE),
    ));
    for point in &points {
        ui.painter().circle_filled(*point, 3.5, CURVE);
    }
    if let Some(pointer) = response.hover_pos()
        && let Some(boundary) = distribution.boundaries.iter().min_by(|a, b| {
            position(a.population_share.units(), a.income_share.units())
                .distance_sq(pointer)
                .total_cmp(
                    &position(b.population_share.units(), b.income_share.units())
                        .distance_sq(pointer),
                )
        })
    {
        let marker = position(
            boundary.population_share.units(),
            boundary.income_share.units(),
        );
        ui.painter()
            .circle_stroke(marker, 6.0, Stroke::new(1.5_f32, Color32::WHITE));
        response.on_hover_ui(|ui| {
            ui.strong(format!(
                "Nearest cohort boundary · {}",
                name(content, country, &boundary.group)
            ));
            ui.label(format!(
                "Cumulative residents {} ({:.2}%) · disposable income {}m/day ({:.2}%)",
                boundary.cumulative_population.0,
                f64::from(boundary.population_share.units()) / 100.0,
                boundary.cumulative_income.0,
                f64::from(boundary.income_share.units()) / 100.0
            ));
            ui.label(format!(
                "This cohort's mean: {:.2} game credits per resident per day",
                boundary.disposable_income.0 as f64 * 1_000_000.0 / boundary.population.0 as f64
            ));
        });
    }
    ui.small("Hover for a cohort boundary; the same values are listed below. Shares round down to whole basis points. This is a simulated distribution of disjoint cohort averages, not a household survey; inequality within each group is unobserved.");
    for point in &distribution.boundaries {
        ui.label(format!(
            "{}: cumulative population {} ({:.2}%) · disposable income {}m/day ({:.2}%)",
            name(content, country, &point.group),
            point.cumulative_population.0,
            f64::from(point.population_share.units()) / 100.0,
            point.cumulative_income.0,
            f64::from(point.income_share.units()) / 100.0
        ));
        ui.small(format!(
            "Cohort mean: {:.2} game credits per resident per day",
            point.disposable_income.0 as f64 * 1_000_000.0 / point.population.0 as f64
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chart_opens_with_real_pointer_input_at_both_scales_and_foreign_data_stays_hidden() {
        let content = Content::business_finance_bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        world.advance(&content, DaySpan::new(1)).unwrap();
        let before = persistence::encode(&world, &content).unwrap();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            ctx.style_mut(|style| style.animation_time = 0.0);
            let render = |events| {
                ctx.run(
                    egui::RawInput {
                        focused: true,
                        events,
                        screen_rect: Some(egui::Rect::from_min_size(
                            Pos2::ZERO,
                            Vec2::new(1280.0 / scale, 800.0 / scale),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default()
                            .show(ctx, |ui| show(ui, &content, &world, &world.player));
                    },
                )
            };
            let _ = render(vec![]);
            let closed = render(vec![]);
            let header = closed
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text)
                        if text
                            .galley
                            .text()
                            .contains("Income distribution · cohort means") =>
                    {
                        Some(text.pos + Vec2::new(10.0, 6.0))
                    }
                    _ => None,
                })
                .expect("visible chart header");
            for pressed in [true, false] {
                let _ = render(vec![
                    egui::Event::PointerMoved(header),
                    egui::Event::PointerButton {
                        pos: header,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]);
            }
            let opened = render(vec![]);
            let labels = opened
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.text()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            for label in [
                "realized disposable income",
                "50%",
                "cumulative population",
                "inequality within each group is unobserved",
            ] {
                assert!(labels.contains(label), "{scale}: {label}: {labels}");
            }
            let foreign = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    show(ui, &content, &world, &CountryId("USA".into()))
                });
            });
            assert!(!foreign.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text().contains("Income distribution"))));
        }
        assert_eq!(persistence::encode(&world, &content).unwrap(), before);
    }
}
