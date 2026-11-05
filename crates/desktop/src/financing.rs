use eframe::egui::{self, Color32, Ui};
use sim_core::*;

/// Called only by the owner-only treasury workspace.
pub fn show(ui: &mut Ui, content: &Content, world: &World, money: &mut i64) -> Option<Action> {
    let state = world.player_country().financing.as_ref()?;
    let definition = content.country(&world.player)?;
    let mut action = None;
    ui.separator();
    ui.strong("Borrowing authority & debt");
    if let Some(interest) = &state.interest_accrual {
        ui.label(
            "Interest accrual · fixed 4% annual, actual/365; fractions carry without compounding.",
        );
        ui.label(format!(
            "Whole interest assessed {}m · actually paid {}m · unpaid whole interest {}m.",
            interest.whole_assessed().0,
            interest.paid.0,
            world.player_country().arrears_external.0
        ));
        ui.label(format!("Unsettled fraction {}/{} of 1m credits. This is a creditor claim, never available cash; it survives principal retirement until a whole unit becomes due.", interest.remainder, INTEREST_DENOMINATOR));
        if let Some(last) = &interest.last {
            ui.small(format!("Latest assessment {} · principal basis {}m · whole interest due {}m. Manual borrowing and repayments made afterward affect the next day.", date(last.day), last.principal.0, last.amount_due.0));
        }
    }
    ui.label("Pending borrowing is not available cash. Approval rechecks governing backing, the debt ceiling and the finite creditor's cash.");
    if let Some(term) = content
        .finance
        .as_ref()
        .and_then(|rules| rules.maturity_days)
    {
        ui.label(format!("New loans: fixed 4% annual interest; principal due {term} days after actual disbursement. Repayment uses uncommitted national cash after protected local claims, before interest and national purchases. Refinancing requires a separately approved loan."));
        let due = state.principal_due_through(world.day).0;
        let upcoming = state.principal_due_through(world.day.saturating_add(30)).0 - due;
        ui.colored_label(if due > 0 {Color32::LIGHT_RED} else {Color32::LIGHT_YELLOW},format!("Overdue/due principal {due}m · additional principal due in next 30 days {upcoming}m. Unpaid principal stays in debt; operating arrears are separate."));
    }
    if let Some(request) = &state.pending {
        ui.label(format!(
            "Pending #{} · {}m · decision {} · {}",
            request.id.0,
            request.amount.0,
            date(request.decision_day),
            request.authority.0
        ));
        if ui.button("Cancel pending loan").clicked() {
            action = Some(Action::CancelBorrow {
                request: request.id,
            });
        }
    }
    ui.horizontal_wrapped(|ui| {
        ui.label("Principal (million credits)");
        ui.add(
            egui::DragValue::new(money)
                .range(1..=definition.gdp / 10)
                .speed((definition.gdp / 10000).max(1) as f64),
        );
        if ui.button("Repay debt · oldest first").clicked() {
            action = Some(Action::Repay {
                amount: Money(*money),
            });
        }
    });
    match world.preview_borrowing(content, &world.player, Money(*money)) {
        Ok(preview) => {
            ui.label(format!(
                "{} · consent {} days · decision {} · fixed 4% annual interest",
                preview.route,
                preview.consent_days,
                date(preview.decision_day)
            ));
            if let Some(term) = content
                .finance
                .as_ref()
                .and_then(|rules| rules.maturity_days)
            {
                ui.label(format!(
                    "If approved on schedule: {}m principal due {}.",
                    money,
                    date(preview.decision_day + term)
                ));
            }
            if ui
                .button(if preview.consent_days == 0 {
                    "Issue authorized loan"
                } else {
                    "Request borrowing consent"
                })
                .clicked()
            {
                action = Some(Action::Borrow {
                    amount: Money(*money),
                });
            }
        }
        Err(reason) => {
            ui.label(format!("Ordinary route unavailable: {reason}"));
        }
    }
    egui::CollapsingHeader::new("Contested emergency financing").show(ui, |ui| {
        ui.colored_label(Color32::LIGHT_YELLOW, "Bypasses constitutional borrowing consent. Effective council backing and creditor limits still apply. This is a political act with consequences.");
        match world.preview_emergency_borrowing(content, &world.player, Money(*money)) {
            Ok(rule) => {
                ui.label(format!("Immediate {}m; approval -{}, legislative support -{}, elite backing -{}, council backing -{} (floor zero). Cooldown {} days.", money, rule.approval_cost, rule.legislative_cost, rule.elite_cost, rule.council_cost, rule.cooldown_days));
                if ui.button("Take contested loan with these costs").clicked() {
                    action = Some(Action::EmergencyBorrow { amount: Money(*money) });
                }
            }
            Err(reason) => { ui.label(reason); }
        }
    });
    egui::CollapsingHeader::new("Debt instruments · reconciled principal").show(ui, |ui| {
        ui.small("Constant-2000 reporting credits, not local-currency FX. Fixed 4% interest; voluntary repayment clears oldest principal first. Dated principal automatically falls due where specified. Opening debt is an authored bookmark calibration, not a claim about historical issuance dates.");
        for loan in &state.instruments {
            let origin = match loan.origin {
                DebtOrigin::OpeningCalibration => "Opening calibration",
                DebtOrigin::AcceptedRequest { contested: false } => "Authorized borrowing",
                DebtOrigin::AcceptedRequest { contested: true } => "Contested borrowing",
            };
            let maturity = loan.maturity_day.map_or_else(||"perpetual".into(), |day| format!("principal due {} · automatically repaid {}m{}",date(day),loan.maturity_repaid.unwrap_or(Money(0)).0,if day<=world.day && loan.outstanding.0>0 {" · OVERDUE"}else{""}));
            ui.label(format!("#{} · {origin} · outstanding {}m / original {}m · {} · creditor {}", loan.id.0, loan.outstanding.0, loan.original_principal.0, date(loan.issued_day), loan.creditor.0));
            ui.small(maturity);
        }
    });
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn carried_interest_screen_reports_actual_payments_and_fraction_at_two_scales() {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-interest-accrual-test.json"
        ))
        .unwrap();
        let mut world = World::new(&content, CountryId("COR".into()), 17).unwrap();
        world.advance(&content, sim_core::DaySpan::new(3)).unwrap();
        let before = world.hash();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1440.0 / scale, 940.0 / scale),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        assert!(show(ui, &content, &world, &mut 1000).is_none());
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
            for text in [
                "actual/365",
                "Whole interest assessed 10m",
                "actually paid 10m",
                "Unsettled fraction 3499600/3650000",
                "2000-01-04",
                "whole interest due 3m",
            ] {
                assert!(labels.contains(text), "{text} at {scale}");
            }
        }
        assert_eq!(world.hash(), before);
    }
    #[test]
    fn dated_debt_screen_exposes_maturity_and_refinancing_limits_without_issuing_commands() {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-debt-maturities-test.json"
        ))
        .unwrap();
        let mut world = World::new(&content, CountryId("COR".into()), 17).unwrap();
        world
            .issue(
                &content,
                Action::Borrow {
                    amount: Money(1000),
                },
            )
            .unwrap();
        let hash = world.hash();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1440.0 / scale, 940.0 / scale),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        assert!(show(ui, &content, &world, &mut 1000).is_none());
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
                "180 days after actual disbursement",
                "Overdue/due principal 0m",
                "separately approved loan",
                "2000-06-29",
            ] {
                assert!(labels.contains(expected), "{expected} at scale {scale}");
            }
        }
        assert_eq!(world.hash(), hash);
    }
}
