use eframe::egui::{self, Color32, Ui};
use sim_core::*;

pub struct Panel {
    target: i64,
    days: u32,
}

impl Default for Panel {
    fn default() -> Self {
        Self {
            target: 100,
            days: 90,
        }
    }
}
impl Panel {
    /// Called only inside the controlled government's Treasury workspace.
    pub fn show(&mut self, ui: &mut Ui, content: &Content, world: &World) -> Option<Action> {
        let rules = content.cash_management.as_ref()?;
        let c = world.player_country();
        let state = c.cash_management.as_ref()?;
        let mut action = None;
        ui.separator();
        ui.strong("Keep cash for an upcoming decision");
        ui.label("Temporary retention sets aside real national cash. It can defer wages, welfare and interest into arrears, and leave matured principal overdue. Protected local funds and existing project reservations remain payable.");
        if let Some(active) = state.active() {
            ui.colored_label(
                Color32::LIGHT_YELLOW,
                format!(
                    "Retention #{} · actual cash {}m / target {}m · unfilled {}m · expires {}",
                    active.id.0,
                    c.retained_cash().0,
                    active.target.0,
                    active.target.0 - c.retained_cash().0,
                    date(active.expiry_day)
                ),
            );
            ui.label("Release the actual cash before approving a decision while paused. Release creates no money and grants no policy authority; costs and implementation delays still apply. Expiry or a successor government releases it automatically.");
            if ui
                .button(format!("Release {}m of actual cash", c.retained_cash().0))
                .clicked()
            {
                action = Some(Action::ReleaseCash { request: active.id });
            }
        } else {
            let max = (content.country(&world.player)?.gdp * rules.max_target_bps / 10000).max(1);
            self.target = self.target.clamp(1, max);
            self.days = self.days.clamp(1, rules.max_days);
            ui.horizontal_wrapped(|ui| {
                ui.label("Target (million credits)");
                ui.add(egui::DragValue::new(&mut self.target).range(1..=max));
                ui.label("Days");
                ui.add(egui::DragValue::new(&mut self.days).range(1..=rules.max_days));
            });
            match world.preview_cash_hold(content, &world.player, Money(self.target), self.days) {
                Ok(preview) => {
                    ui.label(format!("Protect {}m now; remaining {}m must come from future actual cash. Approval -{} (floor zero). Expires {}; reuse from {}. A target is not funded cash.", preview.retained_now.0, self.target - preview.retained_now.0, preview.approval_cost, date(preview.expiry_day), date(preview.next_allowed_day)));
                    if ui
                        .button("Retain actual cash with these consequences")
                        .clicked()
                    {
                        action = Some(Action::HoldCash {
                            target: Money(self.target),
                            days: self.days,
                        });
                    }
                }
                Err(reason) => {
                    ui.label(reason);
                }
            }
        }
        if !state.records.is_empty() {
            egui::CollapsingHeader::new("Cash-retention history").show(ui, |ui| {
                for record in state.records.iter().rev() {
                    ui.label(format!(
                        "#{} · target {}m · {} to {} · {}",
                        record.id.0,
                        record.target.0,
                        date(record.start_day),
                        date(record.expiry_day),
                        record.ended.as_ref().map_or_else(
                            || "active".into(),
                            |(day, reason)| format!("{} on {}", reason.label(), date(*day))
                        )
                    ));
                }
            });
        }
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn treasury_shows_actual_cash_tradeoffs_and_release_at_both_scales() {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-cash-management-test.json"
        ))
        .unwrap();
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        for active in [false, true] {
            if active {
                world
                    .issue(
                        &content,
                        Action::HoldCash {
                            target: Money(100),
                            days: 90,
                        },
                    )
                    .unwrap();
            }
            let before = world.clone();
            for scale in [1.0, 1.4] {
                let ctx = egui::Context::default();
                ctx.set_zoom_factor(scale);
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1100.0 / scale, 940.0 / scale),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            assert!(Panel::default().show(ui, &content, &world).is_none());
                        });
                    },
                );
                let text = output
                    .shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::Shape::Text(t) => Some(t.galley.text()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                assert!(text.contains("Keep cash for an upcoming decision"));
                assert!(text.contains("defer wages"));
                assert!(text.contains("existing project reservations"));
                assert!(text.contains(if active {
                    "Release 100m of actual cash"
                } else {
                    "Approval -3"
                }));
                for shape in &output.shapes {
                    if let egui::Shape::Text(t) = &shape.shape
                        && t.galley.text().contains(if active {
                            "Release 100m"
                        } else {
                            "Retain actual cash with"
                        })
                    {
                        assert!(
                            t.pos.y + t.galley.size().y <= 940.0 / scale,
                            "decision clipped at {scale}"
                        );
                    }
                }
            }
            assert_eq!(world, before);
        }
    }
}
