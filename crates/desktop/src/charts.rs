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
            .filter(|o| o.day + horizon >= world.day)
            .map(|o| (o.day, o.treasury.0 as f64))
            .collect()
    } else {
        world
            .public_report(content, id)
            .ok()?
            .history
            .iter()
            .filter(|o| o.day + horizon >= world.day)
            .map(|o| {
                (
                    o.day,
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
}
impl Chart {
    pub fn show(&mut self, ui: &mut egui::Ui, metric: Metric, series: &[Series]) {
        ui.strong(metric.label());
        let Some((first, last, low, high)) = bounds(series) else {
            ui.label("No published observations in this period. Advance to the first statistical review; unknown history is not a zero series.");
            return;
        };
        let padding = ((high - low) * 0.08).max(1.0);
        let (min, max) = if matches!(metric, Metric::Approval | Metric::Needs) {
            (0.0, 100.0)
        } else {
            ((low - padding).max(0.0), high + padding)
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
        {
            self.cursor = Some(
                first
                    + (((pointer.x - plot.left()) / plot.width()).clamp(0.0, 1.0)
                        * (last - first) as f32)
                        .round() as u32,
            );
        }
        if response.has_focus() {
            let delta = ui.input(|i| {
                i32::from(i.key_pressed(egui::Key::ArrowRight))
                    - i32::from(i.key_pressed(egui::Key::ArrowLeft))
            });
            if delta != 0 {
                self.cursor = Some(
                    (self.cursor.unwrap_or(first) as i64 + i64::from(delta) * 30)
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
                "Inspection date {} · nearest available observation per series",
                sim_core::date(day)
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
                    format!("{}: {} observations", s.label, s.points.len()),
                );
            }
        });
        ui.small("Monthly point observations; output is that release day's flow. Click then use Left/Right to inspect; vertical axes show units and scale.");
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
