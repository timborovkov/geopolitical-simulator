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
        .map_or(90, |loan| {
            ((loan.decision_day).units() - world.day.units() + 30).max(90)
        })
        .min(MAX_DAY - world.day.units())
        .min(365);
    let outlook = match world.funding_outlook(content, horizon, loan) {
        Ok(outlook) => outlook,
        Err(error) => {
            ui.label(error);
            return false;
        }
    };
    ui.strong("Funding outlook · unchanged-policy scenario");
    if country.subnational.is_some() {
        ui.small("National funding excludes protected local funds and grant escrow. This run rate subtracts local tax entitlements; local spending from those protected funds is accounted for separately.");
    }
    if let Some(interest) = &outlook.interest_accrual {
        ui.label(format!("Uncommitted cash {}m · current mean no-shock balance {:+.4}m/day. Interest due over {horizon} days: {}m without borrowing, {}m if approved; existing fractional carry is included.",outlook.available_cash.0,interest.national_daily_numerator as f64 / f64::from(INTEREST_DENOMINATOR),interest.without_loan_due.0,interest.if_approved_due.0));
    } else {
        ui.label(format!(
            "Uncommitted cash {}m · current no-shock balance {:+}m/day.",
            outlook.available_cash.0, outlook.daily_balance.0
        ));
    }
    if country
        .cash_management
        .as_ref()
        .is_some_and(|s| s.active().is_some())
    {
        ui.colored_label(Color32::LIGHT_YELLOW, format!("Temporary retention: {}m actual cash excluded above. This comparison holds that protection constant; it does not simulate future capture or automatic release. Inspect the retention controls before using this forecast.", country.retained_cash().0));
    }
    if outlook.interest_accrual.is_some() {
        ui.label(outlook.covered_days.map_or_else(|| format!("No funding gap within this {horizon}-day scenario; later funding is not guaranteed."),|days|format!("First projected funding gap after {days} fully covered days. Pending borrowing is excluded.")));
    } else {
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
    }
    if content
        .finance
        .as_ref()
        .is_some_and(|rules| rules.maturity_days.is_some())
    {
        ui.colored_label(
            Color32::LIGHT_YELLOW,
            format!(
                "Principal due within {horizon} days: {}m · including conditional borrowing: {}m.",
                outlook.principal_due.0, outlook.conditional_principal_due.0
            ),
        );
    }
    if country.arrears.0 > 0 {
        ui.colored_label(Color32::LIGHT_YELLOW, format!("Outstanding arrears {}m are additional claims; their repayment is excluded from this simplified outlook.", country.arrears.0));
    }
    let mut review_support = false;
    if world
        .governing_rules(content, &world.player)
        .is_ok_and(|government| government.check(country, "borrow", 0).is_err())
        && content.policy(&"coalition".into()).is_some()
    {
        ui.colored_label(
            Color32::LIGHT_YELLOW,
            "Governing backing currently blocks ordinary borrowing.",
        );
        review_support = ui.button("Review governing-support agreement").clicked();
    }
    egui::CollapsingHeader::new("Compare conditional financing · funding chart").show(ui, |ui| {
        ui.small("Forecast, not treasury cash or an approval guarantee. Holds current policy and receipts constant. Excludes supply shocks, market-price changes, arrears repayment and pending-program delivery; those may change the result.");
        if content.finance.as_ref().is_some_and(|rules|rules.maturity_days.is_some()) {
            ui.small("Includes full dated principal and existing overdue principal from tomorrow. Recurring interest stays at the current run rate after repayment, conservatively overstating later interest. Negative balances are funding gaps; refinancing requires approval.");
        }
        if let Some(loan) = outlook.loan {
            let rate = outlook.interest_accrual.as_ref().map_or_else(||format!("{}m/day",outlook.additional_daily_interest.0), |i|format!("{:.4}m/day mean",i.additional_daily_numerator as f64 / f64::from(INTEREST_DENOMINATOR)));
            ui.label(format!("{} {}m if approved on {} · additional interest {rate} after disbursement.", if pending.is_some() { "Pending request:" } else { "Selected ordinary borrowing:" }, loan.amount.0, date(loan.decision_day)));
            if let Some(before) = outlook.points.iter().find(|point| point.day.units() + 1 == loan.decision_day.units())
                && before.without_loan.0 < 0 {
                ui.colored_label(Color32::LIGHT_YELLOW, "The current deficit would exhaust available cash before the borrowing decision. Review spending, revenues or the explicit emergency route and its political costs.");
            }
        } else {
            ui.label("No ordinary loan is currently available for this comparison. The line excludes new borrowing.");
        }
        let mut series = vec![Series { label: "Without new borrowing".into(), points: outlook.points.iter().map(|point| (point.day.units(), point.without_loan.0 as f64)).collect() }];
        if outlook.loan.is_some() {
            series.push(Series { label: "If this loan is approved".into(), points: outlook.points.iter().map(|point| (point.day.units(), point.if_approved.0 as f64)).collect() });
        }
        chart.show_funding_forecast(ui, &series);
    });
    ui.separator();
    review_support
}
