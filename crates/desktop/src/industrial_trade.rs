use eframe::egui;
use sim_core::*;

/// Drafts are scoped to exact content and country, so loading another pack cannot
/// leave an unknown good selected. Rendering submits no simulation commands.
pub fn show(
    ui: &mut egui::Ui,
    content: &Content,
    world: &World,
    country: &CountryId,
) -> Option<Action> {
    let definition = content.industrial_trade.as_ref()?;
    let report = world
        .industrial_trade_report(content, Viewer::Government(world.player.clone()), country)
        .ok()??;
    let mut action = None;
    egui::CollapsingHeader::new("Industrial imports & exports").default_open(true).show(ui,|ui| {
        ui.label("Buy scarce industrial inputs or sell surplus stocks. Purchases reserve cash; exports reserve no stocks. Ordinary consent, daily quotas, storage and buyer cash limit actual deliveries.");
        ui.small("Inputs arrive after production and can feed recipes from the following day. Due funded construction receives packages before exports. Fees are nonrefundable; cancellation keeps completed deliveries and payments.");
        let Some(owned)=&report.owned else {ui.label("Current contracts and stocks are not published for this government.");return;};
        let key=egui::Id::new(("industrial_trade_draft",content.hash(),country.0.as_str()));
        let first=definition.goods.keys().next().expect("validated market").clone();
        let (mut good,mut direction,mut lots)=ui.data_mut(|data|data.get_temp::<(GoodId,IndustrialTradeDirection,u64)>(key).unwrap_or((first,IndustrialTradeDirection::Import,1)));
        if !definition.goods.contains_key(&good) {good=definition.goods.keys().next().unwrap().clone();}
        egui::ComboBox::from_id_salt(key.with("good")).selected_text(&content.production.as_ref().unwrap().framework.goods[&good].name).show_ui(ui,|ui| {
            for id in definition.goods.keys() {ui.selectable_value(&mut good,id.clone(),&content.production.as_ref().unwrap().framework.goods[id].name);}
        });
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut direction,IndustrialTradeDirection::Import,"Import");
            ui.selectable_value(&mut direction,IndustrialTradeDirection::Export,"Export");
            ui.add(egui::DragValue::new(&mut lots).range(1..=100).suffix(" country lots"));
        });
        ui.data_mut(|data|data.insert_temp(key,(good.clone(),direction,lots)));
        let unit=content.production.as_ref().unwrap().countries[country].construction_unit.0;
        let quantity=ProductionQuantity(unit.saturating_mul(lots));
        let stock=world.production.as_ref().unwrap().countries[country].inventory.positions[&good].quantity.0;
        ui.label(format!("Observed stock: {stock} units. Order: {} units ({unit} per country lot).",quantity.0));
        match world.preview_industrial_trade(content,country,&good,direction,quantity) {
            Ok(quote)=> {
                if direction==IndustrialTradeDirection::Export && quote.quoted_cash.0<=quote.fee.0 {ui.label("Quoted sale proceeds do not cover the administration fee.");}
                ui.label(format!("Quote {}m · fee {}m · quota {} units/day · consent no earlier than {} · expires {}. Route: {}",quote.quoted_cash.0,quote.fee.0,quote.daily_limit.0,date(quote.review_day),date(quote.expiry_day),quote.route));
                if ui.button("Submit industrial trade order").clicked(){action=Some(Action::OrderIndustrialTrade{good:good.clone(),direction,quantity});}
            }
            Err(reason)=>{ui.label(reason);}
        }
        let c=&world.countries[country];
        ui.label(format!("Reserved purchase cash {}m · export receipts this fiscal period {}m (separate from taxes)",owned.reserved_cash().0,c.budget.industrial_exports.unwrap_or(Money(0)).0));
        ui.label(format!("{} contracts retained in campaign history; all open and latest 20 closed shown.",owned.orders.len()));
        for order in owned.orders.iter().rev().filter(|o|o.status.open()).chain(owned.orders.iter().rev().filter(|o|!o.status.open()).take(20)) {
            ui.group(|ui| {
                let status=match &order.status {IndustrialTradeStatus::Pending=>"Awaiting consent".into(),IndustrialTradeStatus::Active=>"Filling".into(),IndustrialTradeStatus::Completed=>"Completed".into(),IndustrialTradeStatus::Cancelled=>"Cancelled".into(),IndustrialTradeStatus::Expired=>"Expired".into(),IndustrialTradeStatus::Refused{reason}=>format!("Refused: {reason}")};
                ui.label(format!("#{} · {:?} {} · {} · {} / {} units · {}m settled · {}m reserved",order.id.0,order.direction,content.production.as_ref().unwrap().framework.goods[&order.good].name,status,order.filled().0,order.quantity.0,order.cash().0,order.reserved.0));
                ui.small(format!("Fee {}m · review {} · expiry {}",order.fee.0,date(order.review_day),date(order.expiry_day)));
                if order.status.open() && ui.button(format!("Cancel order #{}",order.id.0)).clicked() {action=Some(Action::CancelIndustrialTrade{request:order.id});}
            });
        }
        egui::CollapsingHeader::new("Trade prices & provenance").show(ui,|ui| {
            ui.label(format!("{} @ {}",definition.source,definition.source_version));
            ui.label(&definition.description);
            for (good,price) in &definition.goods {ui.label(format!("{} · ask {} / bid {} basis points of opening annual GDP per {} country lots",good.0,price.ask_bps.units(),price.bid_bps.units(),definition.price_lots));}
        });
    });
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    fn render(
        ctx: &egui::Context,
        content: &Content,
        w: &World,
        id: &CountryId,
        events: Vec<egui::Event>,
    ) -> (String, Option<egui::Pos2>, Option<Action>) {
        let mut action = None;
        let output = ctx.run(
            egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1100., 1200.),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    action = show(ui, content, w, id);
                });
            },
        );
        let mut text = String::new();
        let mut button = None;
        for shape in &output.shapes {
            if let egui::Shape::Text(t) = &shape.shape {
                text.push_str(t.galley.text());
                text.push('\n');
                if t.galley.text() == "Submit industrial trade order"
                    || t.galley.text().starts_with("Cancel order #")
                {
                    button = Some(t.pos + t.galley.size() / 2.);
                }
            }
        }
        (text, button, action)
    }
    fn click(
        ctx: &egui::Context,
        content: &Content,
        w: &World,
        id: &CountryId,
        pos: egui::Pos2,
    ) -> Action {
        render(ctx, content, w, id, vec![egui::Event::PointerMoved(pos)]);
        render(
            ctx,
            content,
            w,
            id,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            }],
        );
        render(
            ctx,
            content,
            w,
            id,
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        )
        .2
        .expect("explicit click submits command")
    }
    #[test]
    fn owner_can_submit_and_cancel_using_real_controls_at_both_scales() {
        let content = Content::industrial_trade_bundled().unwrap();
        for scale in [1., 1.4] {
            let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
            let id = world.player.clone();
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let key = egui::Id::new(("industrial_trade_draft", content.hash(), id.0.as_str()));
            ctx.data_mut(|data| {
                data.insert_temp(
                    key,
                    (
                        GoodId("tools".into()),
                        IndustrialTradeDirection::Import,
                        1_u64,
                    ),
                )
            });
            let before = persistence::encode(&world, &content).unwrap();
            let mut submit = None;
            for _ in 0..5 {
                let (text, button, action) = render(&ctx, &content, &world, &id, vec![]);
                assert!(action.is_none());
                assert!(text.contains("nonrefundable"));
                submit = button;
            }
            assert_eq!(before, persistence::encode(&world, &content).unwrap());
            let action = click(&ctx, &content, &world, &id, submit.unwrap());
            assert!(matches!(action, Action::OrderIndustrialTrade { .. }));
            assert_eq!(before, persistence::encode(&world, &content).unwrap());
            world.issue(&content, action).unwrap();
            let mut cancel = None;
            for _ in 0..5 {
                cancel = render(&ctx, &content, &world, &id, vec![]).1;
            }
            let action = click(&ctx, &content, &world, &id, cancel.unwrap());
            assert!(matches!(action, Action::CancelIndustrialTrade { .. }));
            world.issue(&content, action).unwrap();
            world.validate(&content).unwrap();
            assert_eq!(
                world
                    .player_country()
                    .industrial_trade
                    .as_ref()
                    .unwrap()
                    .orders[0]
                    .status,
                IndustrialTradeStatus::Cancelled
            );
            ctx.data_mut(|data| {
                data.insert_temp(
                    key,
                    (
                        GoodId("biomass".into()),
                        IndustrialTradeDirection::Export,
                        1_u64,
                    ),
                )
            });
            let before = persistence::encode(&world, &content).unwrap();
            let (text, _, action) = render(&ctx, &content, &world, &id, vec![]);
            assert!(text.contains("Quoted sale proceeds do not cover"));
            assert!(action.is_none());
            assert_eq!(before, persistence::encode(&world, &content).unwrap());
        }
    }
    #[test]
    fn pack_switch_and_foreign_inspection_preserve_bytes_and_hide_private_trade() {
        let content = Content::industrial_trade_bundled().unwrap();
        let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        let foreign = content
            .countries
            .iter()
            .find(|c| c.id != world.player)
            .unwrap()
            .id
            .clone();
        let old = Content::production_bundled().unwrap();
        let old_world = World::new(&old, world.player.clone(), 17).unwrap();
        for scale in [1., 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let before = persistence::encode(&world, &content).unwrap();
            for _ in 0..5 {
                let (text, _, action) = render(&ctx, &content, &world, &foreign, vec![]);
                assert!(action.is_none());
                assert!(!text.contains("Reserved purchase cash"));
                assert!(!text.contains("Submit industrial trade order"));
                assert!(
                    render(&ctx, &old, &old_world, &old_world.player, vec![])
                        .2
                        .is_none()
                );
                render(&ctx, &content, &world, &world.player, vec![]);
            }
            assert_eq!(before, persistence::encode(&world, &content).unwrap());
        }
    }
}
