//! A guide derived from the controlled government's real records. Navigation
//! and time requests return to the normal client; this view cannot issue orders.
use crate::Tab;
use eframe::egui::{self, Ui};
use sim_core::*;

pub enum Navigation {
    Open(Tab),
    Draft { policy: PolicyId, value: i32 },
    Advance(DaySpan),
}
#[derive(Default)]
pub struct Panel {
    selected: Option<CommitmentId>,
    latest_seen: Option<CommitmentId>,
}
fn suggestion(content: &Content, world: &World) -> Option<(PolicyId, i32, String)> {
    let c = world.player_country();
    let balance = world
        .funding_outlook(content, 0, None)
        .ok()?
        .daily_balance
        .0;
    let government = world.governing_rules(content, &world.player).ok()?;
    let backing_blocked = if government.route == DecisionRoute::Legislature {
        c.legislative_support < government.threshold
    } else {
        c.elite_support < government.threshold
    } || c.council_support < government.council_threshold;
    let candidate = if backing_blocked {
        (
            "coalition",
            1,
            "Current governing backing blocks ordinary decisions. Inspect support talks first."
                .into(),
        )
    } else if c.report.energy_coverage < 90 && c.energy_capacity < 200 {
        (
            "energy",
            1,
            format!(
                "Last settlement met {}% of energy needs. Inspect capacity, imports and the current disruption before choosing investment.",
                c.report.energy_coverage
            ),
        )
    } else if c.report.food_coverage < 90 && c.food_capacity < 200 {
        (
            "agriculture",
            1,
            format!(
                "Last settlement met {}% of food needs. Inspect supply and household affordability before choosing investment.",
                c.report.food_coverage
            ),
        )
    } else if balance < 0 && c.tax < 45 {
        ("income_tax", (c.tax + 3).min(45), "Current recurring national obligations exceed the unchanged-policy funding forecast. Compare a tax change with household income and national funding needs.".into())
    } else if c.administration < 100 {
        ("administration", 1, "Collection capacity is below its ceiling. Compare modernization with its cost, delay and effects on households.".into())
    } else {
        return None;
    };
    let (policy, value, reason) = candidate;
    let policy = PolicyId::from(policy);
    content.policy(&policy).map(|_| (policy, value, reason))
}
impl Panel {
    pub fn show(
        &mut self,
        ui: &mut Ui,
        content: &Content,
        world: &World,
        own: bool,
    ) -> Option<Navigation> {
        ui.heading("Your first governing session");
        if !own {
            ui.label(
                "This walkthrough follows your own government and its private decision records.",
            );
            return ui
                .button("Return to my governing session")
                .clicked()
                .then_some(Navigation::Open(Tab::Guide));
        }
        ui.label("Inspect a problem, compare a feasible intervention, follow its delivery, then review observed results. You choose and approve every order on the normal governing screens.");
        let mut navigation = None;
        let country = world.player_country();
        if world.day.units() == content.start_day().expect("validated bookmark") {
            ui.strong("1 · Get a settled report");
            ui.label("Opening figures are starting assumptions. Advance one day to see actual receipts, payments and delivered needs.");
            if ui.button("Advance one day for the first report").clicked() {
                navigation = Some(Navigation::Advance(DaySpan::new(1)));
            }
            return navigation;
        }
        let records: Vec<_> = country
            .commitments
            .iter()
            .flatten()
            .filter(|record| {
                world
                    .journal_entry(&record.proposal_command)
                    .is_some_and(|entry| {
                        !entry.ai && !entry.developer && entry.command.issuer == world.player
                    })
            })
            .collect();
        ui.strong(format!(
            "1 · Read the situation · {}",
            date(world.day.units())
        ));
        ui.label(format!("Energy needs met {}% · food needs met {}% · approval {}%. Last-day receipts {}m; payments and interest {}m.",country.report.energy_coverage,country.report.food_coverage,country.approval,country.report.revenue.0,country.report.spending.0+country.report.interest.0));
        ui.label(format!("Uncommitted national cash {}m · operating arrears {}m. A recurring forecast is not cash available to spend.",country.available_cash().0,country.arrears.0));
        if let Ok(outlook) = world.funding_outlook(content, 0, None) {
            let balance = outlook.interest_accrual.as_ref().map_or_else(
                || format!("{:+}", outlook.daily_balance.0),
                |i| {
                    format!(
                        "{:+.4} mean",
                        i.national_daily_numerator as f64 / f64::from(INTEREST_DENOMINATOR)
                    )
                },
            );
            ui.label(format!("Unchanged-policy national balance {balance}m/day (forecast, after protected local claims)."));
        }
        if let Some(financing) = &country.financing
            && content
                .finance
                .as_ref()
                .is_some_and(|rules| rules.maturity_days.is_some())
        {
            ui.label(format!("Overdue/due principal {}m. The five-year review also requires no overdue principal.",financing.principal_due_through(world.day.units()).0));
        }
        ui.horizontal_wrapped(|ui| {
            for (label, tab) in [
                ("Inspect needs & households", Tab::Society),
                ("Inspect funding & debt", Tab::Treasury),
                ("Inspect governing authority", Tab::Politics),
            ] {
                if ui.button(label).clicked() {
                    navigation = Some(Navigation::Open(tab));
                }
            }
        });
        ui.separator();
        if records.last().is_none_or(|record| {
            matches!(
                record.stage(),
                CommitmentStage::Withdrawn | CommitmentStage::Refused
            )
        }) {
            ui.strong("2 · Compare a decision before approving it");
            if let Some((policy, value, reason)) = suggestion(content, world) {
                ui.label(reason);
                match world.preview(content, &world.player, &policy, value) {
                    Ok(preview) => {
                        ui.label(format!("One option to compare: {} · {}m implementation cost · {} days to scheduled operation. A proposal would pay {}m upfront; remaining funding and authority still matter.",content.policy(&policy).unwrap().name,preview.cost.0,preview.delay.units(),preview.upfront_payment.0));
                    }
                    Err(reason) => {
                        ui.label(format!("This option is currently blocked: {reason}. Inspect the standard decision screen to see its requirements."));
                        if content.cash_management.is_some() {
                            ui.label("Treasury can retain actual receipts for a later decision, at an approval cost and with deferred-payment consequences. Retention grants no cash or authority.");
                            if ui
                                .button("Inspect funding and temporary retention")
                                .clicked()
                            {
                                navigation = Some(Navigation::Open(Tab::Treasury));
                            }
                        }
                    }
                }
                if ui
                    .button(format!("Compare {}", content.policy(&policy).unwrap().name))
                    .clicked()
                {
                    navigation = Some(Navigation::Draft {
                        policy: policy.clone(),
                        value,
                    });
                }
            } else {
                ui.label("No single starting option is suggested. Use your reports and policy comparisons to choose a decision.");
            }
            if ui.button("Browse policy comparisons").clicked() {
                navigation = Some(Navigation::Open(Tab::Policies));
            }
            ui.small("These starting prompts are a learning aid, not an optimal strategy. Other choices may fit your priorities better.");
        } else {
            ui.strong("2 · An accepted decision is recorded");
            if ui.button("Compare another policy decision").clicked() {
                navigation = Some(Navigation::Open(Tab::Policies));
            }
        }
        ui.separator();
        ui.strong("3 · Follow an accepted programme");
        if records.is_empty() {
            ui.label(if country.commitments.is_none() {"This preserved campaign has no detailed programme-history system. Use Policies and Chronicle to follow its decisions."}else{"No programme is accepted yet. Comparing a draft spends nothing; approve a feasible proposal on its normal screen when you are ready."});
            return navigation;
        }
        let latest = records.last().map(|record| record.id);
        if latest != self.latest_seen {
            self.selected = latest;
            self.latest_seen = latest;
        }
        if self
            .selected
            .is_none_or(|id| !records.iter().any(|record| record.id == id))
        {
            self.selected = records.last().map(|record| record.id);
        }
        let selected = self.selected.unwrap();
        egui::ComboBox::from_id_salt("guided-programme")
            .selected_text(format!("Programme #{}", selected.0))
            .show_ui(ui, |ui| {
                for record in records.iter().rev() {
                    let name = content
                        .policy(&record.project.policy)
                        .map_or(record.project.policy.as_str(), |policy| {
                            policy.name.as_str()
                        });
                    ui.selectable_value(
                        &mut self.selected,
                        Some(record.id),
                        format!("#{} · {name} · {:?}", record.id.0, record.stage()),
                    );
                }
            });
        let record = records
            .iter()
            .find(|record| Some(record.id) == self.selected)
            .unwrap();
        let name = content
            .policy(&record.project.policy)
            .map_or(record.project.policy.as_str(), |policy| {
                policy.name.as_str()
            });
        ui.label(format!(
            "{} · {:?} · paid implementation {}m",
            name,
            record.stage(),
            record.paid.0
        ));
        let operation = record
            .milestones
            .iter()
            .find(|milestone| milestone.stage == CommitmentStage::Operating);
        if operation.is_none() {
            if matches!(
                record.stage(),
                CommitmentStage::Withdrawn | CommitmentStage::Refused
            ) {
                ui.label("This programme did not reach operation. Inspect the decision history and compare another intervention; approval or a matching current setting is not delivery evidence.");
            } else if let Some(project) = country.project(&record.id) {
                let next = if project.stage == PolicyStage::AwaitingConsent {
                    project.consent_day
                } else {
                    project.due_day
                };
                let note = if project.stage == PolicyStage::AwaitingConsent {
                    "Review may refuse the proposal; the setting changes only when it actually operates."
                } else {
                    "Consent is complete. Funded work continues; the setting changes only when it actually operates."
                };
                ui.label(format!(
                    "{} · consent {} · scheduled operation {} · reserved capital {}m. {note}",
                    project.stage.label(),
                    date(project.consent_day),
                    date(project.due_day),
                    project.reserved.0
                ));
                if next > world.day.units()
                    && ui
                        .button(format!("Advance toward {}", date(next)))
                        .clicked()
                {
                    navigation = Some(Navigation::Advance(DaySpan::new(
                        (next - world.day.units()).min(365),
                    )));
                }
                ui.small("Advance up to one year at a time, stopping earlier for important events. Review the event and continue when ready.");
            }
        } else if let Some(operation) = operation {
            ui.label(format!(
                "Operating since {}. Recorded direct changes:",
                date(operation.day)
            ));
            for effect in &record.direct_effects {
                ui.label(format!(
                    "{}: {} to {}",
                    super::commitments::effect_label(&effect.field),
                    effect.before,
                    effect.after
                ));
            }
            if let Some(evaluation) = &record.evaluation {
                ui.separator();
                ui.strong(format!(
                    "4 · Review observed results · {}",
                    date(evaluation.day)
                ));
                if let Some(before) = &evaluation.before {
                    ui.small(format!(
                        "Before delivery: settled {}. Review: settled {}.",
                        date(operation.day.saturating_sub(1)),
                        date(evaluation.day)
                    ));
                    egui::ScrollArea::horizontal()
                        .id_salt("guided-review-table-scroll")
                        .show(ui, |ui| {
                            egui::Grid::new("guided-observed-review")
                                .striped(true)
                                .show(ui, |ui| {
                                    for title in
                                        ["Observed measure", "Before delivery", "Review day"]
                                    {
                                        ui.strong(title);
                                    }
                                    ui.end_row();
                                    for (label, first, last) in [
                                        (
                                            "Receipts (m/day)",
                                            before.revenue.0,
                                            evaluation.after.revenue.0,
                                        ),
                                        (
                                            "Payments + interest (m/day)",
                                            before.spending.0 + before.interest.0,
                                            evaluation.after.spending.0
                                                + evaluation.after.interest.0,
                                        ),
                                        (
                                            "Energy needs met (%)",
                                            i64::from(before.energy_coverage),
                                            i64::from(evaluation.after.energy_coverage),
                                        ),
                                        (
                                            "Food needs met (%)",
                                            i64::from(before.food_coverage),
                                            i64::from(evaluation.after.food_coverage),
                                        ),
                                        (
                                            "External disruption (capacity points)",
                                            i64::from(before.shock),
                                            i64::from(evaluation.after.shock),
                                        ),
                                        (
                                            "Approval (%)",
                                            i64::from(evaluation.approval_before),
                                            i64::from(evaluation.approval_after),
                                        ),
                                    ] {
                                        ui.label(label);
                                        ui.label(first.to_string());
                                        ui.label(last.to_string());
                                        ui.end_row();
                                    }
                                });
                        });
                } else {
                    ui.label("No settled pre-delivery baseline was available for this programme.");
                }
                ui.label(format!("{} other programme deliveries occurred during the review window. Prices, other policies and disruptions can also change outcomes; these observed differences do not isolate this programme's causal effect.",evaluation.other_deliveries));
                ui.strong("You have a complete decision record to review. Use the results to choose your next problem; the five-year mandate is a separate objective.");
            } else if let Some(day) = record.review_due {
                ui.label(format!("The observed review is due {}. Operation confirms a setting change; it does not yet establish the programme's wider results.",date(day)));
                if day > world.day.units()
                    && ui
                        .button(format!("Advance toward observed review {}", date(day)))
                        .clicked()
                {
                    navigation = Some(Navigation::Advance(DaySpan::new(day - world.day.units())));
                }
            }
        }
        if ui.button("Inspect full programme history").clicked() {
            navigation = Some(Navigation::Open(Tab::Policies));
        }
        navigation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Content {
        Content::from_json(include_str!("../../../data/synthetic-commitment-test.json")).unwrap()
    }
    fn render(
        panel: &mut Panel,
        world: &World,
        content: &Content,
        own: bool,
        scale: f32,
    ) -> String {
        let ctx = egui::Context::default();
        ctx.set_zoom_factor(scale);
        let hash = world.hash();
        let mut labels = String::new();
        for offset in [0.0, 10000.0] {
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
                        egui::ScrollArea::vertical()
                            .vertical_scroll_offset(offset)
                            .show(ui, |ui| {
                                assert!(panel.show(ui, content, world, own).is_none());
                            });
                    });
                },
            );
            labels.push_str(
                &output
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
                    .join("\n"),
            );
        }
        assert_eq!(world.hash(), hash);
        labels
    }

    #[test]
    fn walkthrough_follows_real_consent_operation_and_review_at_two_scales() {
        let content = fixture();
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        let mut panel = Panel::default();
        for scale in [1.0, 1.4] {
            assert!(
                render(&mut panel, &world, &content, true, scale)
                    .contains("Advance one day for the first report")
            );
        }
        world.advance(&content, sim_core::DaySpan::new(1)).unwrap();
        for scale in [1.0, 1.4] {
            assert!(
                render(&mut panel, &world, &content, true, scale)
                    .contains("No programme is accepted yet")
            );
        }
        world
            .issue(
                &content,
                Action::Enact {
                    policy: "energy".into(),
                    value: 1,
                },
            )
            .unwrap();
        let project = world.player_country().projects[0].clone();
        for scale in [1.0, 1.4] {
            let text = render(&mut panel, &world, &content, true, scale);
            assert!(text.contains(&format!("Advance toward {}", date(project.consent_day))));
            assert!(text.contains("setting changes only when it actually operates"));
            assert!(!text.contains("Operating since"));
        }
        world
            .advance(
                &content,
                sim_core::DaySpan::new(project.consent_day - world.day.units()),
            )
            .unwrap();
        for scale in [1.0, 1.4] {
            let text = render(&mut panel, &world, &content, true, scale);
            assert!(text.contains("Consent is complete"));
            assert!(!text.contains("Review may refuse"));
        }
        world
            .advance(
                &content,
                sim_core::DaySpan::new(project.due_day - world.day.units()),
            )
            .unwrap();
        let review = world.player_country().commitments.as_ref().unwrap()[0]
            .review_due
            .unwrap();
        for scale in [1.0, 1.4] {
            let text = render(&mut panel, &world, &content, true, scale);
            assert!(text.contains("80 to 95"));
            assert!(text.contains(&format!("Advance toward observed review {}", date(review))));
            assert!(!text.contains("complete decision record"));
        }
        world
            .advance(&content, sim_core::DaySpan::new(review - world.day.units()))
            .unwrap();
        for scale in [1.0, 1.4] {
            let text = render(&mut panel, &world, &content, true, scale);
            for expected in [
                "Before delivery",
                "Review day",
                "External disruption",
                "do not isolate",
                "complete decision record",
            ] {
                assert!(text.contains(expected), "{expected} at scale {scale}");
            }
        }
        assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    }
    #[test]
    fn withdrawn_and_foreign_views_never_claim_delivery_or_leak_private_report_data() {
        let content = fixture();
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        world.advance(&content, sim_core::DaySpan::new(1)).unwrap();
        world
            .issue(
                &content,
                Action::Enact {
                    policy: "energy".into(),
                    value: 1,
                },
            )
            .unwrap();
        let id = world.player_country().projects[0].id;
        world
            .issue(&content, Action::CancelCommitment { request: id })
            .unwrap();
        let mut panel = Panel::default();
        let text = render(&mut panel, &world, &content, true, 1.0);
        assert!(text.contains("did not reach operation"));
        assert!(!text.contains("Operating since"));
        world
            .issue(
                &content,
                Action::Enact {
                    policy: "energy".into(),
                    value: 1,
                },
            )
            .unwrap();
        let replacement = world.player_country().projects.last().unwrap().id;
        let replacement_text = render(&mut panel, &world, &content, true, 1.0);
        assert_eq!(panel.selected, Some(replacement));
        assert!(!replacement_text.contains("did not reach operation"));
        let foreign = render(&mut panel, &world, &content, false, 1.0);
        assert!(foreign.contains("Return to my governing session"));
        for private in [
            "Uncommitted national cash",
            "paid implementation",
            "5000m",
            "programme #",
        ] {
            assert!(!foreign.contains(private));
        }
        assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    }
}
