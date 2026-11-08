use eframe::egui::{self, Ui};
use sim_core::*;
#[derive(Default)]
pub struct Panel {
    targets: Option<FiscalTargets>,
    annual: bool,
    inspect_law: Option<LawId>,
}
impl Panel {
    pub fn take_law_request(&mut self) -> Option<LawId> {
        self.inspect_law.take()
    }
    pub fn show(&mut self, ui: &mut Ui, content: &Content, world: &World) -> Option<Action> {
        content.fiscal_plans.as_ref()?;
        let c = world.player_country();
        let state = c.fiscal_plans.as_ref()?;
        let mut action = None;
        ui.heading("Annual & supplementary budget");
        ui.label("Submit tax, service and welfare settings as one fiscal package. Review rechecks institutional backing; enactment pays for real implementation programmes. Recurring services and transfers still need daily cash. This plan is not a hard spending ceiling.");
        if let Some(b) = state.bills.last() {
            ui.label(format!(
                "Latest budget #{} for {}: {} · review {} · legal start {}",
                b.id.0,
                b.year,
                b.status.label(),
                date(b.review_day),
                date(b.effective_day)
            ));
        }
        if !state.pending() {
            let targets = self
                .targets
                .get_or_insert_with(|| FiscalTargets::current(c));
            ui.horizontal_wrapped(|ui| {
                ui.label(if content.income_taxes.is_some() {
                    "Standard income tax %"
                } else {
                    "Income tax %"
                });
                ui.add(egui::DragValue::new(&mut targets.income_tax).range(10..=45));
                ui.label("Services % GDP");
                ui.add(egui::DragValue::new(&mut targets.services).range(4..=18));
                ui.label("Welfare % GDP");
                ui.add(egui::DragValue::new(&mut targets.welfare).range(0..=12));
                if ui.button("Use operating settings").clicked() {
                    *targets = FiscalTargets::current(c);
                }
            });
            ui.checkbox(
                &mut self.annual,
                "Next-year annual budget (otherwise current-year supplementary)",
            );
            let year = date(world.day)[..4].parse::<u32>().expect("year") + u32::from(self.annual);
            // Only changed settings are dispatched as programmes by a fiscal plan.
            for (policy, value, current) in [
                ("income_tax", targets.income_tax, c.tax),
                ("services", targets.services, c.services),
                ("welfare", targets.welfare, c.welfare),
            ] {
                if value != current {
                    self.inspect_law =
                        super::legislation::requirements(ui, content, world, &policy.into(), value)
                            .or(self.inspect_law.take());
                }
            }
            match world.preview_budget(content, &world.player, year, targets) {
                Ok(p) => {
                    ui.label(format!("Fiscal year {year} · {} · review {} · legal start {}. Pay {}m administration now; reserve {}m implementation cash.",p.route,date(p.review_day),date(p.effective_day),p.fee.0,p.implementation_cost.0));
                    if let Some(interest) = &p.planned_forecast.interest_accrual {
                        ui.small(format!("Interest mean {:.4}m/day; whole amount below is due on the next settlement with the current carry. Later daily dues vary as fractions accumulate. National balance figures round down to whole units.",interest.average_per_day()));
                    }
                    ui.label(format!("Conditional daily forecast (consolidated) after all programmes operate: revenue {}m to {}m; spending {}m to {}m; interest {}m. Opening GDP, current supply/collection and funded purchasing are held fixed; future borrowing and shocks are excluded.",p.baseline_forecast.daily_revenue.0,p.planned_forecast.daily_revenue.0,p.baseline_forecast.daily_spending.0,p.planned_forecast.daily_spending.0,p.planned_forecast.daily_interest.0));
                    ui.label(format!("National daily balance after protected local tax shares: {:+}m to {:+}m. Consolidated flows above include local purchasing; these local funds cannot finance national obligations.",p.baseline_national_balance.0,p.planned_national_balance.0));
                    if !p.party_preference_feedback.is_empty() {
                        ui.label(&p.party_preference_feedback);
                    }
                    if p.planned_national_balance.0 < 0 {
                        ui.colored_label(egui::Color32::LIGHT_YELLOW,"This plan forecasts a recurring national funding gap. Reserved implementation cash does not finance daily spending.");
                    }
                    for (policy, value, due) in &p.programmes {
                        ui.small(format!(
                            "{} to {value}: conditional operation {}",
                            content.policy(policy).expect("policy").name,
                            date(*due)
                        ));
                    }
                    if ui.button("Submit budget proposal").clicked() {
                        action = Some(Action::ProposeBudget {
                            year,
                            targets: targets.clone(),
                        });
                    }
                }
                Err(reason) => {
                    ui.label(format!("Cannot propose: {reason}"));
                }
            }
        }
        egui::CollapsingHeader::new("Fiscal plans & actual programme delivery").default_open(!state.bills.is_empty()).show(ui,|ui|{
            if state.bills.is_empty(){ui.label("No combined fiscal plans submitted. Existing cash execution continues under operating settings.");}
            for b in state.bills.iter().rev(){ui.push_id(b.id.0,|ui|{
                ui.strong(format!("Budget #{} · {} · {}",b.id.0,b.year,b.status.label()));
                ui.label(format!("Targets: {} {}%, services {}% GDP, welfare {}% GDP. Review {} · legal start {}. Administration {}m; implementation {}m.",if content.income_taxes.is_some(){"standard income-tax rate"}else{"income tax"},b.targets.income_tax,b.targets.services,b.targets.welfare,date(b.review_day),date(b.effective_day),b.fee.0,b.implementation_cost.0));
                for p in &b.programmes {if let Some(r)=c.commitments.as_ref().and_then(|r|r.iter().find(|r|r.id==p.project)){ui.label(format!("{} · policy #{} · {:?} · due {}",content.policy(&p.policy).expect("policy").name,p.project.0,r.stage(),date(r.project.due_day)));}}
                if b.status==BudgetBillStatus::Pending && ui.button(format!("Withdraw budget #{}",b.id.0)).clicked(){action=Some(Action::WithdrawBudget{request:b.id});}
            });}
            ui.label(format!("Operating now: {} {}%, services {}% GDP, welfare {}% GDP. The execution and year-end reports below record actual receipts, payments, capital, financing and arrears.",if content.income_taxes.is_some(){"standard income-tax rate"}else{"income tax"},c.tax,c.services,c.welfare));
        });
        ui.separator();
        action
    }
}
fn published_value(policy: &PolicyId, value: i32) -> String {
    match policy.as_str() {
        "income_tax" => format!("{value}%"),
        "services" | "welfare" => format!("{value}% GDP"),
        _ => value.to_string(),
    }
}
/// Public setting history only; this component has no world or command entry point.
pub fn published_delivery(
    ui: &mut Ui,
    content: &Content,
    country: &CountryId,
    plan: &PublishedFiscalPlan,
    open: bool,
) {
    egui::CollapsingHeader::new("Policy setting changes · schedule & observed operation")
        .id_salt(("public-fiscal-delivery",country.0.as_str(),plan.year,plan.enacted_day.units()))
        .default_open(open).show(ui, |ui| {
            if plan.delivery.is_empty() {
                ui.label("No setting-operation details are published.");
                return;
            }
            egui::ScrollArea::horizontal().id_salt(("public-fiscal-table",country.0.as_str(),plan.year,plan.enacted_day.units())).show(ui, |ui| {
                egui::Grid::new(("public-fiscal-grid",country.0.as_str(),plan.year,plan.enacted_day.units())).striped(true).show(ui, |ui| {
                    for title in ["Policy setting", "Approved target", "Scheduled change", "Observed setting change"] {ui.strong(title);}
                    ui.end_row();
                    for row in &plan.delivery {
                        ui.label(content.policy(&row.policy).map_or(row.policy.as_str(),|p|p.name.as_str()));
                        ui.label(published_value(&row.policy,row.target));
                        ui.label(date(row.scheduled_day));
                        ui.label(row.operation.as_ref().map_or_else(||"No published setting change".into(),|operation|format!("{} · {}",published_value(&row.policy,operation.value),date(operation.day))));
                        ui.end_row();
                    }
                });
            });
            ui.small("These are historical policy-setting changes. Daily funding and country outcomes are separate; later decisions can change operating settings again.");
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_setting_history_shows_partial_operation_and_dates_at_two_scales() {
        for scale in [1.0, 1.4] {
            let content = Content::bundled().unwrap();
            let mut world = World::new(&content, CountryId("FRA".into()), 17).unwrap();
            world
                .advance(&content, sim_core::DaySpan::new(108))
                .unwrap();
            let hash = world.hash();
            let foreign = CountryId("GBR".into());
            let report = world.public_report(&content, &foreign).unwrap();
            let plan = &report.fiscal_plans.as_ref().unwrap()[0];
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let output = ctx.run(
                egui::RawInput {
                    focused: true,
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1440.0 / scale, 940.0 / scale),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        published_delivery(ui, &content, &foreign, plan, true)
                    });
                },
            );
            let labels = output
                .shapes
                .iter()
                .filter_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape {
                        Some(text.galley.text())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            for expected in [
                "30% · 2000-04-18",
                "No published setting change",
                "8% GDP",
                "2000-05-18",
                "historical policy-setting changes",
            ] {
                assert!(
                    labels.contains(expected),
                    "{expected} missing at scale {scale}"
                );
            }
            assert_eq!(world.hash(), hash);
        }
    }

    #[test]
    fn blocked_fiscal_target_requests_law_inspection_without_submitting_or_replacing_the_draft() {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-legislation-test.json"
        ))
        .unwrap();
        let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        let before = world.clone();
        for scale in [1., 1.4] {
            let targets = FiscalTargets {
                services: 14,
                ..FiscalTargets::current(world.player_country())
            };
            let mut panel = Panel {
                targets: Some(targets.clone()),
                ..Default::default()
            };
            let mut action = None;
            super::super::legislation::click_required_law(scale, |ui| {
                action = panel.show(ui, &content, &world).or(action.take());
            });
            assert!(action.is_none());
            assert_eq!(
                panel.take_law_request(),
                Some(LawId("high_service_authority".into()))
            );
            assert!(panel.take_law_request().is_none());
            assert_eq!(panel.targets, Some(targets));
            assert_eq!(world, before);
        }
    }

    #[test]
    fn fiscal_preview_is_read_only_and_submission_is_explicit_at_two_scales() {
        for scale in [1.0, 1.4] {
            let c = Content::from_json(include_str!(
                "../../../data/synthetic-fiscal-plans-test.json"
            ))
            .unwrap();
            let w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
            let hash = w.hash();
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut panel = Panel {
                targets: Some(FiscalTargets {
                    income_tax: 28,
                    services: 10,
                    welfare: 3,
                }),
                annual: false,
                ..Default::default()
            };
            let input = egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1440.0 / scale, 940.0 / scale),
                )),
                ..Default::default()
            };
            let draw = |p: &mut Panel, input: egui::RawInput| {
                let mut action = None;
                let output = ctx.run(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        action = p.show(ui, &c, &w);
                    });
                });
                (output, action)
            };
            let (o, a) = draw(&mut panel, input.clone());
            assert!(a.is_none());
            assert_eq!(w.hash(), hash);
            let labels = o
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
            assert!(labels.contains("reserve"));
            assert!(labels.contains("Conditional daily forecast"));
            assert!(labels.contains("Recurring services and transfers still need daily cash"));
            let button = o
                .shapes
                .iter()
                .find_map(|s| {
                    if let egui::Shape::Text(t) = &s.shape {
                        if t.galley.text() == "Submit budget proposal" {
                            let p = t.pos + t.galley.size() / 2.0;
                            assert!(s.clip_rect.contains(p));
                            Some(p)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .expect("visible submit");
            let mut action = None;
            for pressed in [true, false] {
                let mut i = input.clone();
                i.events = vec![
                    egui::Event::PointerMoved(button),
                    egui::Event::PointerButton {
                        pos: button,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ];
                let (_, a) = draw(&mut panel, i);
                action = action.or(a);
            }
            assert!(matches!(
                action,
                Some(Action::ProposeBudget { year: 2000, .. })
            ));
            assert_eq!(w.hash(), hash);
        }
    }
}
