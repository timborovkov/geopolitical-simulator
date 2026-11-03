use crate::charts::{Chart, Series};
use eframe::egui::{self, Color32, Ui};
use sim_core::*;

/// Treasury calls this only after its foreign-information guard.
pub fn show(ui: &mut Ui, content: &Content, world: &World, amount: i64, chart: &mut Chart) -> bool {
    let country = world.player_country();
    let pending = country
        .financing
        .as_ref()
        .and_then(|state| state.pending.as_ref());
    let loan = pending
        .map(|request| LoanAssumption {
            amount: request.amount,
            decision_day: request.decision_day,
        })
        .or_else(|| {
            world
                .preview_borrowing(content, &world.player, Money(amount))
                .ok()
                .map(|preview| LoanAssumption {
                    amount: preview.amount,
                    decision_day: preview.decision_day,
                })
        });
    let horizon = loan
        .map_or(90, |loan| (loan.decision_day - world.day + 30).max(90))
        .min(MAX_DAY - world.day)
        .min(365);
    let outlook = match world.funding_outlook(content, horizon, loan) {
        Ok(outlook) => outlook,
        Err(error) => {
            ui.label(error);
            return false;
        }
    };
    ui.strong("Funding outlook · unchanged-policy scenario");
    ui.label(format!(
        "Uncommitted cash {}m · current no-shock balance {:+}m/day.",
        outlook.available_cash.0, outlook.daily_balance.0
    ));
    match outlook.covered_days {
        Some(days) => {
            ui.colored_label(if days < 30 { Color32::LIGHT_YELLOW } else { Color32::LIGHT_GRAY }, format!("Available cash covers {days} full days of this deficit. Pending borrowing is excluded."));
        }
        None => {
            ui.label(
                "Current run rate is balanced or positive; this does not guarantee future funding.",
            );
        }
    }
    if country.arrears.0 > 0 {
        ui.colored_label(Color32::LIGHT_YELLOW, format!("Outstanding arrears {}m are additional claims; their repayment is excluded from this simplified outlook.", country.arrears.0));
    }
    let mut review_support = false;
    if world
        .governing_rules(content, &world.player)
        .is_ok_and(|government| government.check(country, "borrow", 0).is_err())
        && content.policy("coalition").is_some()
    {
        ui.colored_label(
            Color32::LIGHT_YELLOW,
            "Governing backing currently blocks ordinary borrowing.",
        );
        review_support = ui.button("Review governing-support agreement").clicked();
    }
    egui::CollapsingHeader::new("Compare conditional financing · funding chart").show(ui, |ui| {
        ui.small("Forecast, not treasury cash or an approval guarantee. Holds current policy and receipts constant. Excludes supply shocks, market-price changes, arrears repayment and pending-program delivery; those may change the result.");
        if let Some(loan) = outlook.loan {
            ui.label(format!("{} {}m if approved on {} · additional interest {}m/day after disbursement.", if pending.is_some() { "Pending request:" } else { "Selected ordinary borrowing:" }, loan.amount.0, date(loan.decision_day), outlook.additional_daily_interest.0));
            if let Some(before) = outlook.points.iter().find(|point| point.day + 1 == loan.decision_day)
                && before.without_loan.0 < 0 {
                ui.colored_label(Color32::LIGHT_YELLOW, "The current deficit would exhaust available cash before the borrowing decision. Review spending, revenues or the explicit emergency route and its political costs.");
            }
        } else {
            ui.label("No ordinary loan is currently available for this comparison. The line excludes new borrowing.");
        }
        let mut series = vec![Series { label: "Without new borrowing".into(), points: outlook.points.iter().map(|point| (point.day, point.without_loan.0 as f64)).collect() }];
        if outlook.loan.is_some() {
            series.push(Series { label: "If this loan is approved".into(), points: outlook.points.iter().map(|point| (point.day, point.if_approved.0 as f64)).collect() });
        }
        chart.show_funding_forecast(ui, &series);
    });
    ui.separator();
    review_support
}
