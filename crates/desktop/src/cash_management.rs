use eframe::egui::{self, Color32, Ui};
use sim_core::*;

pub struct Panel {
    target: i64,
    days: u32,
    plan: Option<Draft>,
    reveal: bool,
}
struct Draft {
    country: CountryId,
    content_hash: u64,
    policy: PolicyId,
    value: i32,
}
pub enum Choice {
    Command(Action),
    Compare { policy: PolicyId, value: i32 },
}

impl Default for Panel {
    fn default() -> Self {
        Self {
            target: 100,
            days: 90,
            plan: None,
            reveal: false,
        }
    }
}
impl Panel {
    pub fn prepare(
        &mut self,
        content: &Content,
        world: &World,
        policy: &PolicyId,
        value: i32,
    ) -> Result<(), String> {
        let rules = content
            .cash_management
            .as_ref()
            .ok_or("This campaign has no temporary cash retention")?;
        let plan = world.policy_funding_plan(content, policy, value)?;
        let max = (content
            .country(&world.player)
            .ok_or("Missing country definition")?
            .gdp
            * rules.max_target_bps.units_i64()
            / 10000)
            .max(1);
        if plan.preview.cost.0 > max {
            return Err("This decision costs more than the temporary retention limit; review other financing".into());
        }
        self.target = plan.preview.cost.0;
        self.days = rules.max_days.units();
        self.reveal = true;
        self.plan = Some(Draft {
            country: world.player.clone(),
            content_hash: world.content_hash,
            policy: policy.clone(),
            value,
        });
        Ok(())
    }
    /// Called only inside the controlled government's Treasury workspace.
    pub fn show(&mut self, ui: &mut Ui, content: &Content, world: &World) -> Option<Choice> {
        let rules = content.cash_management.as_ref()?;
        let c = world.player_country();
        let state = c.cash_management.as_ref()?;
        let mut action = None;
        ui.separator();
        let heading = ui.strong("Keep cash for an upcoming decision");
        if self.reveal {
            heading.scroll_to_me(Some(egui::Align::Min));
            self.reveal = false;
        }
        if self
            .plan
            .as_ref()
            .is_some_and(|p| p.country != world.player || p.content_hash != world.content_hash)
        {
            self.plan = None;
        }
        if let Some(draft) = &self.plan {
            let definition = content.policy(&draft.policy);
            let name = definition.map_or(draft.policy.as_str(), |p| p.name.as_str());
            let target = if matches!(draft.policy.as_str(), "income_tax" | "services" | "welfare") {
                format!(" · proposed {}%", draft.value)
            } else {
                String::new()
            };
            ui.strong(format!("Planned decision: {name}{target}"));
            match world.policy_funding_plan(content, &draft.policy, draft.value) {
                Ok(plan) => {
                    ui.label(format!("Implementation cost {}m · spendable cash {}m · actual retained cash {}m · spendable shortfall {}m. Retained cash must be released before it can fund this decision.",plan.preview.cost.0,plan.available_cash.0,plan.retained_cash.0,plan.shortfall.0));
                }
                Err(reason) => {
                    ui.colored_label(
                        Color32::LIGHT_YELLOW,
                        format!("This decision now needs review: {reason}"),
                    );
                }
            }
            ui.small("After releasing enough cash, compare and approve the policy on its normal screen. Future receipts and a retention target are not spendable cash.");
            if ui.button("Compare planned decision").clicked() {
                action = Some(Choice::Compare {
                    policy: draft.policy.clone(),
                    value: draft.value,
                });
            }
        }
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
                action = Some(Choice::Command(Action::ReleaseCash { request: active.id }));
            }
        } else {
            let max = (content.country(&world.player)?.gdp * rules.max_target_bps.units_i64()
                / 10000)
                .max(1);
            self.target = self.target.clamp(1, max);
            self.days = self.days.clamp(1, rules.max_days.units());
            ui.horizontal_wrapped(|ui| {
                ui.label("Target (million credits)");
                ui.add(egui::DragValue::new(&mut self.target).range(1..=max));
                ui.label("Days");
                ui.add(egui::DragValue::new(&mut self.days).range(1..=rules.max_days.units()));
            });
            match world.preview_cash_hold(content, &world.player, Money(self.target), self.days) {
                Ok(preview) => {
                    ui.label(format!("Protect {}m now; remaining {}m must come from future actual cash. Approval -{} (floor zero). Expires {}; reuse from {}. A target is not funded cash.", preview.retained_now.0, self.target - preview.retained_now.0, preview.approval_cost, date(preview.expiry_day), date(preview.next_allowed_day)));
                    if ui
                        .button("Retain actual cash with these consequences")
                        .clicked()
                    {
                        action = Some(Choice::Command(Action::HoldCash {
                            target: Money(self.target),
                            days: self.days,
                        }));
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
    fn preparing_a_different_decision_keeps_an_existing_retention_and_filters_campaign_context() {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-cash-management-test.json"
        ))
        .unwrap();
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        world
            .issue(
                &content,
                Action::HoldCash {
                    target: Money(100),
                    days: 90,
                },
            )
            .unwrap();
        let before = world.clone();
        let mut panel = Panel::default();
        panel
            .prepare(&content, &world, &"income_tax".into(), 30)
            .unwrap();
        assert_eq!(panel.target, 20);
        let ctx = egui::Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                assert!(panel.show(ui, &content, &world).is_none());
            });
        });
        let text = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t.galley.text()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Implementation cost 20m"));
        assert!(text.contains("actual cash 100m / target 100m"));
        assert_eq!(world, before);
        let other = World::new(&content, CountryId("BOR".into()), 17).unwrap();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                assert!(panel.show(ui, &content, &other).is_none());
            });
        });
        assert!(panel.plan.is_none());
        let old = Content::debt_bundled().unwrap();
        let old_world = World::new(&old, CountryId("GBR".into()), 17).unwrap();
        assert!(
            panel
                .prepare(&old, &old_world, &"income_tax".into(), 31)
                .is_err()
        );
    }
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
