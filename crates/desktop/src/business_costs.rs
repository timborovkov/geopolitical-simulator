use eframe::egui::{self, Color32};
use sim_core::*;

pub fn show(ui: &mut egui::Ui, content: &Content, world: &World, country: &CountryId) {
    if content.business_costs.is_none() {
        return;
    }
    egui::CollapsingHeader::new("Business production costs & earnings")
        .default_open(true)
        .show(ui, |ui| {
            if let Err(e) = body(ui, content, world, country) {
                ui.colored_label(Color32::LIGHT_YELLOW, e);
            }
        });
}
fn body(
    ui: &mut egui::Ui,
    content: &Content,
    world: &World,
    country: &CountryId,
) -> Result<(), String> {
    let report =
        world.business_profit_report(content, Viewer::Government(world.player.clone()), country)?;
    let Some(detail) = report.owned else {
        ui.label("Current business costs and results are private to this government.");
        return Ok(());
    };
    let (all, today) = (&detail.cumulative, &detail.current_day);
    ui.label(format!(
        "{} · millions of constant-2000 USD-equivalent game credits",
        date(report.day)
    ));
    ui.small("Modeled business result after actual factor costs and stock consumption. Cash receipts can be advances; cash purchases can remain in stock. This is not a complete corporate income statement.");
    egui::Grid::new(("business_cost_statement", country.0.as_str()))
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Account");
            ui.strong("Current day");
            ui.strong("Since opening");
            ui.end_row();
            let rows = [
                ("Household and external sales", today.sales, all.sales),
                ("Public cash receipts", today.public_cash, all.public_cash),
                (
                    "Change in deferred capital",
                    today.deferred_capital,
                    all.deferred_capital,
                ),
                (
                    "Change in maintenance receivables",
                    today.receivables,
                    all.receivables,
                ),
                ("Revenue earned", today.revenue()?, all.revenue()?),
                (
                    "Period factor costs",
                    today.period_factors,
                    all.period_factors,
                ),
                (
                    "Cost of food delivered",
                    today.food_sold_cost,
                    all.food_sold_cost,
                ),
                (
                    "Energy used in production",
                    today.energy_used_cost,
                    all.energy_used_cost,
                ),
                (
                    "Food spoilage cost",
                    today.food_spoiled_cost,
                    all.food_spoiled_cost,
                ),
                (
                    "Import payments without whole receipts",
                    today.unstocked_import_cost,
                    all.unstocked_import_cost,
                ),
                (
                    "Production costs expensed",
                    today.expenses()?,
                    all.expenses()?,
                ),
                ("Modeled business result", today.result()?, all.result()?),
            ];
            for (label, t, a) in rows {
                ui.label(label);
                ui.label(t.0.to_string());
                ui.label(a.0.to_string());
                ui.end_row();
            }
            Ok::<(), String>(())
        })
        .inner?;
    let revenue = today.revenue()?.0;
    let expenses = today.expenses()?.0;
    let scale = revenue.max(expenses).max(1) as f32;
    ui.label(format!("Current-day revenue {revenue}m"));
    bar(ui, revenue, scale, Color32::from_rgb(45, 180, 140));
    ui.label(format!("Current-day production expenses {expenses}m"));
    bar(ui, expenses, scale, Color32::from_rgb(220, 170, 65));
    ui.label(format!("Cash/profit bridge since opening: cash change {}m + owner payments {}m + inventory {}m + receivables {}m − deferred capital {}m = business result {}m",all.cash_change.0,all.distributions.0,all.inventory_value.0,all.receivables.0,all.deferred_capital.0,all.result()?.0));
    ui.small("Owner payments reduce equity, not profit. Inventory carrying values are acquisition costs, not cash or market prices. Public industrial stocks and equipment remain public and are excluded.");
    ui.collapsing(
        "Stock costs and public contract obligations",
        |ui| -> Result<(), String> {
            ui.label(format!(
                "Food stock cost {}m · energy stock cost {}m",
                detail.costs.food_inventory()?.0,
                detail.costs.energy_inventory()?.0
            ));
            ui.label(format!(
                "Contract capital recognized on delivery {}m · retained after withdrawal {}m",
                detail.delivered_contract_capital.0, detail.retained_withdrawal_capital.0
            ));
            for p in &detail.pending_capital {
                ui.label(format!(
                    "Commitment #{} · paid capital awaiting delivery {}m",
                    p.commitment, p.amount.0
                ));
            }
            if let Some(day) = &detail.latest_costs {
                ui.label(format!(
                    "{} production receipt · actual private factor payments {}m",
                    date(day.day),
                    day.factors_paid.0
                ));
                stock(ui, "Food", &day.food, Food::units);
                stock(ui, "Energy", &day.energy, Energy::units);
            } else {
                ui.label("Opening state: no production cost receipt until the first settled day.");
            }
            Ok(())
        },
    )
    .body_returned
    .transpose()?;
    let d = content.business_costs.as_ref().expect("cost definition");
    ui.small(format!("Authored direct factor allocation: food {} basis points, energy {}; remaining and idle allocations are expensed today. Shares round down. Stock value uses a moving weighted average; whole-credit rounding is explicit.",d.food_factor_bps.units(),d.energy_factor_bps.units()));
    ui.small("Public reserve releases have zero business acquisition cost. Policy administration earns the standard 10% at proposal; paid capital stays deferred until actual delivery or funded withdrawal. Outstanding maintenance claims are receivables. Private fixed-asset depreciation, unpaid contractual payroll, corporate tax, investment and insolvency remain unfinished.");
    if content.business_finance.is_some() {
        ui.small("Current payout policy uses prior closed-day retained earnings and protects both recent payments and unfinished client advances; inspect Business receipts, wages & cash below.");
    } else {
        ui.small("Existing autonomous cash payouts use their earlier cash policy.");
    }
    Ok(())
}
fn bar(ui: &mut egui::Ui, value: i64, scale: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(
        egui::Vec2::new(ui.available_width().max(1.), 18.),
        egui::Sense::hover(),
    );
    ui.painter().rect_filled(rect, 0., Color32::from_gray(8));
    if value > 0 {
        let width = rect.width() * (value as f32 / scale).clamp(0., 1.);
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min, egui::Vec2::new(width, rect.height())),
            0.,
            color,
        );
    }
}
fn stock<Q: Copy>(ui: &mut egui::Ui, name: &str, v: &ValuedBusinessStock<Q>, units: fn(Q) -> i64) {
    let p = &v.physical;
    ui.label(format!("{name} · opening {} units / {}m; produced {} / {}m; imports {} / {}m; public reserve supply {} units / 0m",units(p.opening_stock),v.opening_value.0,units(p.production),v.production_cost.0,units(p.imports),v.import_cost.0,p.reserve_release.map_or(0,units)));
    ui.label(format!(
        "Used {} units / {}m; spoiled {} / {}m; closing {} / {}m; import rounding expense {}m",
        units(p.delivered),
        v.used_cost.0,
        units(p.loss),
        v.lost_cost.0,
        units(p.closing_stock),
        v.closing_value.0,
        v.unstocked_import_cost.0
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn render(ctx: &egui::Context, c: &Content, w: &World, id: &CountryId) -> String {
        let output = ctx.run(
            egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::Vec2::new(1300., 1800.),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    body(ui, c, w, id).unwrap();
                });
            },
        );
        output
            .shapes
            .iter()
            .filter_map(|s| {
                if let egui::Shape::Text(t) = &s.shape {
                    Some(t.galley.text().to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    #[test]
    fn production_costs_profit_bridge_and_foreign_privacy_render_without_mutation_at_two_scales() {
        let c = Content::business_costs_bundled().unwrap();
        let mut w = World::new_scenario(
            &c,
            CountryId("GBR".into()),
            17,
            Scenario::FiscalSupplyCrisis,
        )
        .unwrap();
        w.advance(&c, DaySpan::new(1)).unwrap();
        let before = persistence::encode(&w, &c).unwrap();
        for scale in [1., 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut text = String::new();
            for _ in 0..3 {
                text = render(&ctx, &c, &w, &w.player);
            }
            for label in [
                "Modeled business result",
                "Cost of food delivered",
                "Current-day revenue",
                "Cash/profit bridge",
                "Owner payments reduce equity",
            ] {
                assert!(text.contains(label), "{label}: {text}");
            }
            let foreign = render(&ctx, &c, &w, &CountryId("USA".into()));
            assert!(foreign.contains("private to this government"));
            assert!(!foreign.contains("Cash/profit bridge"));
            assert!(!foreign.contains("Public cash receipts"));
        }
        assert_eq!(persistence::encode(&w, &c).unwrap(), before);
    }
}
