use eframe::egui;
use sim_core::*;

pub fn show(ui: &mut egui::Ui, content: &Content, world: &World) -> Option<Action> {
    let rules = content.reserves.as_ref()?;
    let state = world.player_country().reserves.as_ref()?;
    let mut action = None;
    ui.collapsing("National food & energy reserves",|ui|{
        ui.label("These national buffers are separate from firms' working stocks. Paid import contracts fill gradually after consent, only when civilian physical needs are covered and import capacity remains. Fixed quoted prices and finite contract quantities are scenario rules, not a historical commodity exchange.");
        ui.label("Stored goods automatically cover physical shortfalls left after normal imports. Food still has to reach households through the existing market; insufficient household cash can still limit delivery. Food stocks spoil at the ordinary storage rate. Energy quantities are model equivalents, not a claim about electricity storage technology.");
        ui.small(format!("Storage capacity: {} days of baseline need per commodity · contract duration {} days after consent · {}@{}",rules.capacity_days,rules.order_days.units(),rules.source,rules.source_version));
        for good in [GoodKind::Energy,GoodKind::Food] {
            ui.strong(format!("{good:?} · national inventory {} units",state.stored(good)));
            match world.reserve_supply(content,&world.player,good) {
                Ok(Some(s))=>{
                    ui.label(format!("Observed {}: civilian physical supply {} / need {}; civilian imports {} / capacity {}; spare capacity {} units/day.",date(s.day),s.civilian_supply,s.physical_need,s.civilian_imports,s.import_capacity,s.spare_import_capacity));
                    ui.label(s.explanation());
                    ui.small("This is the last settlement, not a forecast or guaranteed delivery. Future supply, imports and contract consent can change.");
                },
                Ok(None)=>{ui.small("No settled supply observation yet. Advance one day to inspect civilian needs and spare import capacity; procurement cannot deliver on proposal.");},
                Err(reason)=>{ui.small(reason);},
            }
            match good {
                GoodKind::Energy=>{if let Some(f)=&state.energy.last {ui.label(format!("{}: opening {} · purchased {} · released {} · loss {} · closing {}",date(f.day),f.opening,f.purchased,f.released,f.loss,f.closing));}},
                GoodKind::Food=>{if let Some(f)=&state.food.last {ui.label(format!("{}: opening {} · purchased {} · released {} · loss {} · closing {}",date(f.day),f.opening,f.purchased,f.released,f.loss,f.closing));}},
            }
            for count in 1..=rules.capacity_days.units() {
                let days = NeedDays::new(count);
                match world.preview_reserve(content,&world.player,good,days) {
                    Ok(p)=>{
                        ui.label(format!("{days} day(s), {} units: pay {}m administration now; reserve {}m purchase cash. Review {}; earliest delivery {}; expires {}. Quoted price index {:.4}. {}",p.quantity,p.fee.0,p.cost.0,date(p.review_day),date(p.earliest_delivery_day),date(p.expiry_day),f64::from(p.price_bps.units())/10000.0,p.route));
                        if ui.button(format!("Authorize {good:?} contract · {days} day(s)")).clicked() {action=Some(Action::ProcureReserve{good,days});}
                    }
                    Err(reason)=>{ui.small(format!("{days} day(s): {reason}"));}
                }
            }
        }
        ui.small("Orders buy a fixed quantity, not an endless refill target. Cancellation/expiry returns only unspent reservations; delivered goods and administration remain paid. Procurement is recorded in actual national payments; forecasts do not create purchasing power.");
        let open_count=state.orders.iter().filter(|o|o.status.open()).count();
        ui.strong(format!("Open reserve contracts ({open_count})"));
        for order in state.orders.iter().rev().filter(|o|o.status.open()) {
            if let Some(next)=show_order(ui,content,world,order) {action=Some(next);}
        }
        let closed_count=state.orders.len()-open_count;
        if closed_count>0 {
            ui.collapsing(format!("Closed retained contracts ({closed_count})"),|ui|{
                ui.small("Up to 128 contracts are retained. Paid goods and administration remain recorded after closure; lifetime inventory flows remain in the stock accounts.");
                for order in state.orders.iter().rev().filter(|o|!o.status.open()) {
                    let _=show_order(ui,content,world,order);
                }
            });
        }

    });
    action
}

fn show_order(
    ui: &mut egui::Ui,
    content: &Content,
    world: &World,
    order: &ReserveOrder,
) -> Option<Action> {
    let status = match &order.status {
        ReserveStatus::Pending => "Pending".into(),
        ReserveStatus::Active => "Active".into(),
        ReserveStatus::Completed => "Completed".into(),
        ReserveStatus::Cancelled => "Cancelled".into(),
        ReserveStatus::Expired => "Expired".into(),
        ReserveStatus::Refused { reason } => format!("Refused: {reason}"),
    };
    ui.separator();
    ui.label(format!(
        "#{} {:?}: {status} · filled {}/{} units · paid {}m of {}m · reserved {}m",
        order.id.0,
        order.good,
        order.filled,
        order.quantity,
        order.paid.0,
        order.quoted_cost.0,
        order.reserved.0
    ));
    ui.label(format!(
        "Proposed {} · consent review {} · earliest delivery {} · final delivery date {}",
        date(order.proposed_day),
        date(order.review_day),
        date(
            order
                .review_day
                .max(sim_core::CampaignDay::new((order.proposed_day).units() + 1))
        ),
        date(order.expiry_day)
    ));
    ui.small(format!(
        "Administration paid {}m · fixed quoted price index {:.4} · unfilled quantity {} units",
        order.fee.0,
        f64::from(order.price_bps.units()) / 10000.0,
        order.quantity - order.filled
    ));
    if let Some(day) = order.last_fill_day {
        ui.label(format!(
            "Last purchase {}: {} units",
            date(day),
            order.last_fill_quantity
        ));
    }
    if let Some(day) = order.resolved_day {
        ui.label(format!("Closed {}. Unspent purchase reservations were released; paid goods and administration were not refunded.",date(day)));
    }
    if order.status.open() {
        let next = order
            .review_day
            .max(sim_core::CampaignDay::new((order.proposed_day).units() + 1))
            .max(sim_core::CampaignDay::new(world.day.units() + 1));
        let remaining = if next <= order.expiry_day {
            (order.expiry_day).units() - (next).units() + 1
        } else {
            0
        };
        ui.label(format!("Remaining delivery settlements: {remaining}. Consent, civilian needs and spare import capacity still govern actual fills."));
        match world.preview_cancel_reserve(content, &world.player, order.id) {
            Ok(unspent) => {
                if ui
                    .button(format!(
                        "Cancel reserve contract #{} · release {}m reservation",
                        order.id.0, unspent.0
                    ))
                    .clicked()
                {
                    return Some(Action::CancelReserve { request: order.id });
                }
            }
            Err(reason) => {
                ui.small(format!("Cancellation unavailable: {reason}"));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_older_open_contract_stays_visible_and_all_closed_records_can_be_inspected_read_only() {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-national-reserves-test.json"
        ))
        .unwrap();
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        world
            .issue(
                &content,
                Action::ProcureReserve {
                    good: GoodKind::Energy,
                    days: sim_core::NeedDays::new(1),
                },
            )
            .unwrap();
        let mut closed = Vec::new();
        for _ in 0..9 {
            let request = ReserveOrderId(world.next_command.0);
            world
                .issue(
                    &content,
                    Action::ProcureReserve {
                        good: GoodKind::Food,
                        days: sim_core::NeedDays::new(1),
                    },
                )
                .unwrap();
            world
                .issue(&content, Action::CancelReserve { request })
                .unwrap();
            closed.push(request);
        }
        world.validate(&content).unwrap();
        let before = world.hash();
        let ctx = egui::Context::default();
        ctx.style_mut(|s| s.animation_time = 0.0);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 8000.0),
            )),
            ..Default::default()
        };
        let frame = |input: egui::RawInput| {
            ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    assert!(show(ui, &content, &world).is_none());
                });
            })
        };
        let text = |output: &egui::FullOutput| {
            output
                .shapes
                .iter()
                .filter_map(|s| {
                    if let egui::Shape::Text(t) = &s.shape {
                        Some(t.galley.text())
                    } else {
                        None
                    }
                })
                .collect::<String>()
        };
        let click = |output: &egui::FullOutput, label: &str| {
            let pos = output
                .shapes
                .iter()
                .find_map(|s| {
                    if let egui::Shape::Text(t) = &s.shape {
                        t.galley
                            .text()
                            .contains(label)
                            .then_some(t.pos + egui::vec2(5.0, 5.0))
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| panic!("Missing clickable label {label}"));
            let mut clicked = input.clone();
            clicked.events = vec![
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
            let _ = frame(clicked);
            frame(input.clone())
        };
        let closed_panel = frame(input.clone());
        let open_panel = click(&closed_panel, "National food & energy reserves");
        let visible = text(&open_panel);
        assert!(
            visible.contains("#1 Energy: Pending"),
            "An open contract must not disappear behind eight newer closed records"
        );
        assert!(visible.contains("Remaining delivery settlements: 90"));
        assert!(visible.contains("consent review 2000-01-15"));
        assert!(visible.contains("final delivery date 2000-04-13"));
        let history = click(&open_panel, "Closed retained contracts (9)");
        let visible = text(&history);
        assert!(visible.contains(&format!("#{} Food: Cancelled", closed.first().unwrap().0)));
        assert!(visible.contains(&format!("#{} Food: Cancelled", closed.last().unwrap().0)));
        assert_eq!(world.hash(), before);
    }
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
            assert!(text.contains("No settled supply observation yet"));
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
