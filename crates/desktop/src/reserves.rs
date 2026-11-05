use eframe::egui;
use sim_core::*;

pub fn show(ui: &mut egui::Ui, content: &Content, world: &World) -> Option<Action> {
    let rules = content.reserves.as_ref()?;
    let state = world.player_country().reserves.as_ref()?;
    let mut action = None;
    ui.collapsing("National food & energy reserves",|ui|{
        ui.label("These national buffers are separate from firms' working stocks. Paid import contracts fill gradually after consent, only when civilian physical needs are covered and import capacity remains. Fixed quoted prices and finite contract quantities are scenario rules, not a historical commodity exchange.");
        ui.label("Stored goods automatically cover physical shortfalls left after normal imports. Food still has to reach households through the existing market; insufficient household cash can still limit delivery. Food stocks spoil at the ordinary storage rate. Energy quantities are model equivalents, not a claim about electricity storage technology.");
        ui.small(format!("Storage capacity: {} days of baseline need per commodity · contract duration {} days after consent · {}@{}",rules.capacity_days,rules.order_days,rules.source,rules.source_version));
        for good in [GoodKind::Energy,GoodKind::Food] {
            ui.strong(format!("{good:?} · national inventory {} units",state.stored(good)));
            match good {
                GoodKind::Energy=>{if let Some(f)=&state.energy.last {ui.label(format!("{}: opening {} · purchased {} · released {} · loss {} · closing {}",date(f.day),f.opening,f.purchased,f.released,f.loss,f.closing));}},
                GoodKind::Food=>{if let Some(f)=&state.food.last {ui.label(format!("{}: opening {} · purchased {} · released {} · loss {} · closing {}",date(f.day),f.opening,f.purchased,f.released,f.loss,f.closing));}},
            }
            for days in 1..=rules.capacity_days {
                match world.preview_reserve(content,&world.player,good,days) {
                    Ok(p)=>{
                        ui.label(format!("{days} day(s), {} units: pay {}m administration now; reserve {}m purchase cash. Review {}; earliest delivery {}; expires {}. Quoted price index {:.4}. {}",p.quantity,p.fee.0,p.cost.0,date(p.review_day),date(p.earliest_delivery_day),date(p.expiry_day),f64::from(p.price_bps)/10000.0,p.route));
                        if ui.button(format!("Authorize {good:?} contract · {days} day(s)")).clicked() {action=Some(Action::ProcureReserve{good,days});}
                    }
                    Err(reason)=>{ui.small(format!("{days} day(s): {reason}"));}
                }
            }
        }
        ui.small("Orders buy a fixed quantity, not an endless refill target. Cancellation/expiry returns only unspent reservations; delivered goods and administration remain paid. Procurement is recorded in actual national payments; forecasts do not create purchasing power.");
        for order in state.orders.iter().rev().take(8) {
            ui.label(format!("#{} {:?}: {:?} · filled {}/{} units · paid {}m of {}m · reserved {}m",order.id.0,order.good,order.status,order.filled,order.quantity,order.paid.0,order.quoted_cost.0,order.reserved.0));
            if order.status.open() {
                match world.preview_cancel_reserve(content,&world.player,order.id) {
                    Ok(unspent)=>{ if ui.button(format!("Cancel reserve contract #{} · release {}m reservation",order.id.0,unspent.0)).clicked() {action=Some(Action::CancelReserve{request:order.id});} },
                    Err(reason)=>{ui.small(format!("Cancellation unavailable: {reason}"));}
                }
            }
        }
    });
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reserve_previews_open_at_two_scales_without_issuing_orders() {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-national-reserves-test.json"
        ))
        .unwrap();
        let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        let before = world.hash();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(scale);
            ctx.style_mut(|s| s.animation_time = 0.0);
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1280.0 / scale, 2500.0 / scale),
                )),
                ..Default::default()
            };
            let _ = ctx.run(input.clone(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    assert!(show(ui, &content, &world).is_none());
                });
            });
            let pos = egui::pos2(50.0, 15.0);
            let mut click = input.clone();
            click.events = vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ];
            let _ = ctx.run(click, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    assert!(show(ui, &content, &world).is_none());
                });
            });
            let output = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    assert!(show(ui, &content, &world).is_none());
                });
            });
            let text: String = output
                .shapes
                .iter()
                .filter_map(|s| {
                    if let egui::Shape::Text(t) = &s.shape {
                        Some(t.galley.text())
                    } else {
                        None
                    }
                })
                .collect();
            assert!(text.contains("national inventory 0 units"));
            assert!(text.contains("administration now; reserve"));
            assert!(text.contains("earliest delivery"));
        }
        assert_eq!(world.hash(), before);
        assert!(
            world
                .player_country()
                .reserves
                .as_ref()
                .unwrap()
                .orders
                .is_empty()
        );
    }
}
