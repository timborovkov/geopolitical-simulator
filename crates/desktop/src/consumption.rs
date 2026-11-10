use eframe::egui::{self, Color32, Rect, Sense, Vec2};
use sim_core::*;
pub fn show(ui: &mut egui::Ui, content: &Content, world: &World, country: &CountryId) {
    if content.consumption.is_none() {
        return;
    }
    egui::CollapsingHeader::new("Planned spending, delivered food & cash kept")
        .default_open(true)
        .show(ui, |ui| body(ui, content, world, country));
}
fn body(ui: &mut egui::Ui, content: &Content, world: &World, country: &CountryId) {
    let report = match world.consumption_report(
        content,
        Viewer::Government(world.player.clone()),
        country,
    ) {
        Ok(r) => r,
        Err(e) => {
            ui.colored_label(Color32::LIGHT_YELLOW, e);
            return;
        }
    };
    let Some(receipt) = report.owned else {
        ui.label("Current household orders and payments are private to this government.");
        return;
    };
    let lines = receipt.groups;
    if lines.is_empty() {
        ui.label("Opening state: advance one day for actual household orders, food and payments.");
        return;
    }
    ui.label(format!(
        "{} settlement · cash in millions of constant-2000 USD-equivalent game credits",
        date(report.day)
    ));
    ui.small("Desired spending is the basket/income plan. Planned cash is limited by the group's cash at clearing. Food is allocated once across those affordable orders; undelivered food is not charged. Other basket spending remains abstract.");
    let (mut need, mut affordable, mut delivered, mut desired, mut planned, mut paid, mut kept) =
        (0_i64, 0_i64, 0_i64, 0_i128, 0_i128, 0_i128, 0_i128);
    egui::ScrollArea::horizontal().id_salt(("consumption_table",country.0.as_str())).show(ui,|ui|{
        egui::Grid::new("consumption_receipts").striped(true).show(ui,|ui|{
            for label in ["Group","Food need","Affordable","Delivered","Desired m","Plan m","Paid m","Kept m"]{ui.strong(label);}ui.end_row();
            for line in &lines{
                let p=&line.receipt;need+=p.food_need.units();affordable+=p.affordable_food.units();delivered+=p.delivered_food.units();desired+=i128::from(p.desired_spending.0);planned+=i128::from(p.planned_cash.0);paid+=i128::from(p.food_paid.0)+i128::from(p.other_paid.0);kept+=i128::from(p.cash_kept.0);
                ui.label(&line.name).on_hover_text(format!("Desired spending {}m; quoted food budget {}m; actual food payment {}m; other basket payment {}m.",p.desired_spending.0,p.food_budget.0,p.food_paid.0,p.other_paid.0));
                for value in [p.food_need.units(),p.affordable_food.units(),p.delivered_food.units(),p.desired_spending.0,p.planned_cash.0,p.food_paid.0+p.other_paid.0,p.cash_kept.0]{ui.label(value.to_string());}ui.end_row();
            }
        });
    });
    let supplied = receipt.food_supply.expect("settled food supply").units();
    ui.label(format!(
        "Physical food supply {supplied} · full food need {need}"
    ));
    let poverty = need - affordable;
    let shortage = affordable - delivered;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width().max(1.), 18.), Sense::hover());
    let mut x = rect.left();
    for (quantity, color) in [
        (delivered, Color32::from_rgb(45, 180, 140)),
        (poverty, Color32::from_rgb(220, 170, 65)),
        (shortage, Color32::from_rgb(220, 90, 85)),
    ] {
        let width = rect.width() * quantity as f32 / need.max(1) as f32;
        ui.painter().rect_filled(
            Rect::from_min_size(egui::pos2(x, rect.top()), Vec2::new(width, rect.height())),
            0.,
            color,
        );
        x += width;
    }
    response.on_hover_text(format!(
        "Need {need} = delivered {delivered} + affordability gap {poverty} + supply gap {shortage}."
    ));
    ui.label(format!("Food need {need} = delivered {delivered} (green) + affordability gap {poverty} (amber) + supply gap {shortage} (red)"));
    ui.small(format!(
        "Food quantity unit: {}",
        content
            .markets
            .as_ref()
            .expect("markets")
            .good(GoodKind::Food)
            .unit
    ));
    ui.label(format!("Desired spending {desired}m at this settlement"));
    ui.strong(format!(
        "Planned cash {planned}m = actual payments {paid}m + cash kept {kept}m"
    ));
    if poverty > 0 {
        ui.colored_label(Color32::LIGHT_YELLOW,"At today's quote, some food needs were outside household purchasing power. Compare household income, tax, welfare and prices.");
    }
    if poverty > 0 && supplied < need {
        ui.small("Today's supply is also below total need; funding all orders would still leave a physical shortfall at this supply.");
    }
    if shortage > 0 {
        ui.colored_label(Color32::LIGHT_YELLOW,"At today's quote, some affordable food orders lacked supply. Inspect domestic capacity, import limits and reserve releases.");
    }
    if poverty == 0 && shortage == 0 {
        ui.label("All modeled food needs were delivered.");
    }
    ui.small("Cash kept stays with the same household group; it is not extra income or welfare. Partial food charges round down within whole reporting units; a quote can also stay unspent when no whole food units are requested. The next day's plan uses the remaining cash; there is no second clearing batch today.");
}

#[cfg(test)]
mod tests {
    use super::*;
    fn render(
        ctx: &egui::Context,
        c: &Content,
        w: &World,
        id: &CountryId,
        events: Vec<egui::Event>,
    ) -> String {
        let output = ctx.run(
            egui::RawInput {
                focused: true,
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(1100., 1400.),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| body(ui, c, w, id));
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
    fn actual_shortage_receipts_and_private_projection_render_read_only_at_both_scales() {
        let c = Content::consumption_bundled().unwrap();
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
                text = render(&ctx, &c, &w, &w.player, vec![]);
            }
            for label in [
                "Desired m",
                "Plan m",
                "Paid m",
                "Kept m",
                "supply gap",
                "actual payments",
                "import limits",
            ] {
                assert!(text.contains(label), "{label}: {text}");
            }
            let foreign = render(&ctx, &c, &w, &CountryId("USA".into()), vec![]);
            assert!(foreign.contains("private to this government"));
            assert!(!foreign.contains("actual payments"));
            assert!(!foreign.contains("Working households"));
        }
        assert_eq!(persistence::encode(&w, &c).unwrap(), before);
    }
    #[test]
    fn opening_receipts_wait_for_a_settled_day_without_inventing_orders() {
        let c = Content::consumption_bundled().unwrap();
        let w = World::new(&c, CountryId("GBR".into()), 17).unwrap();
        let before = persistence::encode(&w, &c).unwrap();
        let ctx = egui::Context::default();
        let text = render(&ctx, &c, &w, &w.player, vec![]);
        assert!(text.contains("advance one day"));
        assert!(!text.contains("actual payments"));
        assert_eq!(persistence::encode(&w, &c).unwrap(), before);
    }
}
