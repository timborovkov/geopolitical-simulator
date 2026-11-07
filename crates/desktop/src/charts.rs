use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use sim_core::{Content, CountryId, World};
#[derive(Clone, Copy, PartialEq)]
pub enum Metric {
    Approval,
    Needs,
    Debt,
    Output,
    Treasury,
}
impl Metric {
    pub fn label(self) -> &'static str {
        match self {
            Self::Approval => "Approval (%)",
            Self::Needs => "Needs met (%)",
            Self::Debt => "Debt principal (m credits)",
            Self::Output => "Daily output (m credits)",
            Self::Treasury => "Treasury cash (m credits, own government)",
        }
    }
}
pub struct Series {
    pub label: String,
    pub points: Vec<(u32, f64)>,
}
pub fn series(
    world: &World,
    content: &Content,
    id: &CountryId,
    metric: Metric,
    horizon: u32,
) -> Option<Series> {
    let name = content.country(id)?.name.clone();
    let points = if metric == Metric::Treasury {
        if id != &world.player {
            return None;
        }
        world
            .player_country()
            .history
            .iter()
            .filter(|o| o.day.units() + horizon >= world.day.units())
            .map(|o| (o.day.units(), o.treasury.0 as f64))
            .collect()
    } else {
        world
            .public_report(content, id)
            .ok()?
            .history
            .iter()
            .filter(|o| o.day.units() + horizon >= world.day.units())
            .map(|o| {
                (
                    o.day.units(),
                    match metric {
                        Metric::Approval => f64::from(o.approval),
                        Metric::Needs => f64::from(o.needs),
                        Metric::Debt => o.debt.0 as f64,
                        Metric::Output => o.daily_output.0 as f64,
                        Metric::Treasury => unreachable!(),
                    },
                )
            })
            .collect()
    };
    Some(Series {
        label: name,
        points,
    })
}
/// Archived samples remain a separately labelled series rather than silently
/// replacing recent point observations with calendar-year means.
pub fn archived_series(
    world: &World,
    content: &Content,
    id: &CountryId,
    metric: Metric,
    horizon: u32,
) -> Option<Series> {
    let name = content.country(id)?.name.clone();
    let points: Vec<_> = if metric == Metric::Treasury {
        if id != &world.player {
            return None;
        }
        world
            .player_country()
            .statistical_archive
            .as_ref()?
            .years
            .iter()
            .filter(|summary| summary.last_day.units().saturating_add(horizon) >= world.day.units())
            .filter_map(|summary| {
                summary
                    .mean()
                    .map(|mean| (mean.day.units(), mean.treasury.0 as f64))
            })
            .collect()
    } else {
        world
            .public_report(content, id)
            .ok()?
            .archived_reviews?
            .iter()
            .filter(|summary| summary.last_day.units().saturating_add(horizon) >= world.day.units())
            .map(|summary| {
                (
                    summary.last_day.units(),
                    match metric {
                        Metric::Approval => f64::from(summary.mean.approval),
                        Metric::Needs => f64::from(summary.mean.needs),
                        Metric::Debt => summary.mean.debt.0 as f64,
                        Metric::Output => summary.mean.daily_output.0 as f64,
                        Metric::Treasury => unreachable!(),
                    },
                )
            })
            .collect()
    };
    (!points.is_empty()).then(|| Series {
        label: format!("{name} · archived review-sample means"),
        points,
    })
}
fn bounds(series: &[Series]) -> Option<(u32, u32, f64, f64)> {
    let mut points = series.iter().flat_map(|s| s.points.iter());
    let first = points.next()?;
    let (mut min_day, mut max_day, mut min, mut max) = (first.0, first.0, first.1, first.1);
    for (day, value) in points {
        min_day = min_day.min(*day);
        max_day = max_day.max(*day);
        min = min.min(*value);
        max = max.max(*value);
    }
    Some((min_day, max_day.max(min_day + 1), min, max))
}
fn nearest(points: &[(u32, f64)], day: u32) -> Option<(u32, f64)> {
    points
        .iter()
        .min_by_key(|(date, _)| date.abs_diff(day))
        .copied()
}
#[derive(Default)]
pub struct Chart {
    cursor: Option<u32>,
    hover_position: Option<Pos2>,
}
impl Chart {
    pub fn show(&mut self, ui: &mut egui::Ui, metric: Metric, series: &[Series]) {
        self.draw(
            ui,
            metric.label(),
            series,
            matches!(metric, Metric::Approval | Metric::Needs),
            false,
        );
    }
    pub fn show_funding_forecast(&mut self, ui: &mut egui::Ui, series: &[Series]) {
        self.draw(
            ui,
            "Uncommitted funding balance · hypothetical forecast · million credits",
            series,
            false,
            true,
        );
    }
    fn draw(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        series: &[Series],
        percent: bool,
        forecast: bool,
    ) {
        ui.strong(title);
        let Some((first, last, low, high)) = bounds(series) else {
            ui.label("No published observations in this period. Advance to the first statistical review; unknown history is not a zero series.");
            return;
        };
        let padding = ((high - low) * 0.08).max(1.0);
        let (min, max) = if percent {
            (0.0, 100.0)
        } else {
            (
                if forecast {
                    low - padding
                } else {
                    (low - padding).max(0.0)
                },
                high + padding,
            )
        };
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 220.0), Sense::click());
        if response.clicked() {
            response.request_focus();
        }
        let plot = Rect::from_min_max(
            rect.min + Vec2::new(76.0, 12.0),
            rect.max - Vec2::new(12.0, 30.0),
        );
        let painter = ui.painter_at(rect);
        let map = |day: u32, value: f64| {
            Pos2::new(
                plot.left() + (day - first) as f32 / (last - first) as f32 * plot.width(),
                plot.bottom() - ((value - min) / (max - min)) as f32 * plot.height(),
            )
        };
        for tick in 0..=4 {
            let fraction = f64::from(tick) / 4.0;
            let value = min + (max - min) * fraction;
            let y = plot.bottom() - fraction as f32 * plot.height();
            painter.line_segment(
                [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
                Stroke::new(0.5_f32, Color32::from_gray(65)),
            );
            painter.text(
                Pos2::new(plot.left() - 7.0, y),
                egui::Align2::RIGHT_CENTER,
                format!("{value:.0}"),
                egui::FontId::monospace(11.0),
                Color32::LIGHT_GRAY,
            );
        }
        if forecast && min < 0.0 && max > 0.0 {
            let y = map(first, 0.0).y;
            painter.line_segment(
                [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
                Stroke::new(1.5_f32, Color32::LIGHT_YELLOW),
            );
            painter.text(
                Pos2::new(plot.right(), y - 2.0),
                egui::Align2::RIGHT_BOTTOM,
                "0 · funding threshold",
                egui::FontId::monospace(11.0),
                Color32::LIGHT_YELLOW,
            );
        }
        for (day, align) in [
            (first, egui::Align2::LEFT_TOP),
            (first + (last - first) / 2, egui::Align2::CENTER_TOP),
            (last, egui::Align2::RIGHT_TOP),
        ] {
            let position = map(day, min);
            painter.text(
                position + Vec2::new(0.0, 10.0),
                align,
                sim_core::date(day),
                egui::FontId::monospace(11.0),
                Color32::LIGHT_GRAY,
            );
        }
        let colors = [
            Color32::from_rgb(104, 224, 186),
            Color32::from_rgb(245, 181, 105),
            Color32::from_rgb(130, 185, 250),
            Color32::from_rgb(215, 165, 245),
        ];
        for (index, series) in series.iter().enumerate() {
            let color = colors[index % colors.len()];
            let points: Vec<_> = series.points.iter().map(|(d, v)| map(*d, *v)).collect();
            if points.len() > 1 {
                painter.add(egui::Shape::line(
                    points.clone(),
                    Stroke::new(if index == 0 { 2.5_f32 } else { 1.5_f32 }, color),
                ));
            }
            for p in points {
                if index.is_multiple_of(2) {
                    painter.circle_filled(p, 2.5, color);
                } else {
                    painter.rect_filled(Rect::from_center_size(p, Vec2::splat(5.0)), 0.0, color);
                }
            }
        }
        if response.hovered()
            && let Some(pointer) = response.hover_pos()
            && (self.hover_position != Some(pointer) || response.clicked())
        {
            self.cursor = Some(
                first
                    + (((pointer.x - plot.left()) / plot.width()).clamp(0.0, 1.0)
                        * (last - first) as f32)
                        .round() as u32,
            );
        }
        self.hover_position = response.hover_pos();
        if response.has_focus() {
            let delta = ui.input(|i| {
                i32::from(i.key_pressed(egui::Key::ArrowRight))
                    - i32::from(i.key_pressed(egui::Key::ArrowLeft))
            });
            if delta != 0 {
                self.cursor = Some(
                    (self.cursor.unwrap_or(first) as i64
                        + i64::from(delta) * if forecast { 1 } else { 30 })
                    .clamp(i64::from(first), i64::from(last)) as u32,
                );
            }
        }
        if let Some(day) = self.cursor {
            let day = day.clamp(first, last);
            let x = map(day, min).x;
            painter.line_segment(
                [Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())],
                Stroke::new(1.0_f32, Color32::WHITE),
            );
            ui.label(format!(
                "Inspection date {} · nearest available {} per series",
                sim_core::date(day),
                if forecast {
                    "scenario point"
                } else {
                    "observation"
                }
            ));
            for (index, s) in series.iter().enumerate() {
                if let Some((observed, value)) = nearest(&s.points, day) {
                    ui.colored_label(
                        colors[index % colors.len()],
                        format!("{} · {}: {value:.0}", s.label, sim_core::date(observed)),
                    );
                }
            }
        }
        ui.horizontal_wrapped(|ui| {
            for (index, s) in series.iter().enumerate() {
                ui.colored_label(
                    colors[index % colors.len()],
                    format!("{}: {} points", s.label, s.points.len()),
                );
            }
        });
        ui.small(if forecast { "Daily hypothetical funding balances; negative values are funding gaps, not overdrafts. Click then use Left/Right to inspect one day at a time." } else { "Review points show the release day; output is that day's flow. Separately labelled archive series show floor-rounded review-sample means at the last sampled date, not annual totals or daily time averages. Click then use Left/Right to inspect." });
        egui::CollapsingHeader::new("Inspect chart values").show(ui, |ui| {
            for s in series {
                ui.strong(&s.label);
                for (day, value) in &s.points {
                    ui.label(format!("{} · {value:.0}", sim_core::date(*day)));
                }
            }
        });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keyboard_inspection_keeps_its_date_when_the_mouse_stays_still() {
        let ctx = egui::Context::default();
        let mut chart = Chart::default();
        let data = vec![Series {
            label: "forecast".into(),
            points: (0..=90)
                .map(|day| (day, 1000.0 - f64::from(day) * 20.0))
                .collect(),
        }];
        let input = egui::RawInput {
            focused: true,
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 500.0))),
            ..Default::default()
        };
        let draw = |chart: &mut Chart, ctx: &egui::Context| {
            egui::CentralPanel::default().show(ctx, |ui| chart.show_funding_forecast(ui, &data));
        };
        let _ = ctx.run(input.clone(), |ctx| draw(&mut chart, ctx));
        let position = Pos2::new(450.0, 100.0);
        let mut click = input.clone();
        click.events = vec![
            egui::Event::PointerMoved(position),
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::PointerButton {
                pos: position,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        let _ = ctx.run(click, |ctx| draw(&mut chart, ctx));
        let initial = chart.cursor.unwrap();
        assert!(initial > 0 && initial < 89);
        let mut arrow = input.clone();
        arrow.events = vec![egui::Event::Key {
            key: egui::Key::ArrowRight,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }];
        let _ = ctx.run(arrow, |ctx| draw(&mut chart, ctx));
        assert_eq!(chart.cursor, Some(initial + 1));
        let _ = ctx.run(input, |ctx| draw(&mut chart, ctx));
        assert_eq!(chart.cursor, Some(initial + 1));
    }
    #[test]
    fn date_coordinates_preserve_irregular_observations() {
        let data = vec![Series {
            label: "test".into(),
            points: vec![(30, 2.0), (60, 4.0), (61, 3.0)],
        }];
        assert_eq!(bounds(&data), Some((30, 61, 2.0, 4.0)));
        assert_eq!(nearest(&data[0].points, 59), Some((60, 4.0)));
        assert!(bounds(&[]).is_none());
    }
    #[test]
    fn foreign_treasury_is_not_a_comparison_series() {
        let c = Content::bundled().unwrap();
        let w = World::new(&c, CountryId("GBR".into()), 4).unwrap();
        assert!(series(&w, &c, &CountryId("CHN".into()), Metric::Treasury, 360).is_none());
        assert!(
            series(&w, &c, &w.player, Metric::Treasury, 360)
                .unwrap()
                .points
                .is_empty()
        );
    }
}
