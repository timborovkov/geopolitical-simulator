use eframe::egui::{self, Color32, Ui};
use sim_core::*;

/// Called only by the owner-only treasury workspace.
pub fn show(ui: &mut Ui, content: &Content, world: &World, money: &mut i64) -> Option<Action> {
    let state = world.player_country().financing.as_ref()?;
    let definition = content.country(&world.player)?;
    let mut action = None;
    ui.separator();
    ui.strong("Borrowing authority & debt");
    ui.label("Pending borrowing is not available cash. Approval rechecks governing backing, the debt ceiling and the finite creditor's cash.");
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
        ui.small("Constant-2000 reporting credits, not local-currency FX. Fixed 4% perpetual debt; voluntary repayment clears oldest principal first. Opening debt is an authored bookmark calibration, not a claim about historical issuance dates.");
        for loan in &state.instruments {
            let origin = match loan.origin {
                DebtOrigin::OpeningCalibration => "Opening calibration",
                DebtOrigin::AcceptedRequest { contested: false } => "Authorized borrowing",
                DebtOrigin::AcceptedRequest { contested: true } => "Contested borrowing",
            };
            ui.label(format!("#{} · {origin} · outstanding {}m / original {}m · {} · creditor {}", loan.id.0, loan.outstanding.0, loan.original_principal.0, date(loan.issued_day), loan.creditor.0));
        }
    });
    action
}
