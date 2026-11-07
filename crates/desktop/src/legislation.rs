use eframe::egui;
use sim_core::*;

pub fn show(ui: &mut egui::Ui, content: &Content, world: &World) -> Option<Action> {
    let rules = content.legislation.as_ref()?;
    let state = world.player_country().legislation.as_ref()?;
    let mut action = None;
    ui.heading("Statutory authority");
    ui.label("Fund drafting and implementation, obtain governing consent, then commission the permitted programme through its ordinary funding and delivery process.");
    for law in &rules.definitions {
        egui::CollapsingHeader::new(&law.name).id_salt((&law.id.0,"statute")).default_open(true).show(ui, |ui| {
            ui.label(&law.description);
            ui.small(format!("Proposal availability: {} · scenario source {} / {}",law.available_from,law.source,law.source_version));
            for requirement in rules.policy_requirements.iter().filter(|r|r.laws.contains(&law.id)) {
                ui.label(format!("Required for new {} targets from {}.",content.policy(&requirement.policy).expect("policy").name,requirement.minimum_value));
            }
            if let Some(enacted) = state.enacted.get(&law.id) {
                ui.colored_label(super::MINT,format!("Enacted {} · bill #{}. Permission is in force; programme funding and implementation remain separate.",date(enacted.day.units()),enacted.bill.0));
            } else {
                match world.preview_law(content,&world.player,&law.id) {
                    Ok(p) => {
                        ui.label(format!("Total {}m: pay {}m drafting now and protect {}m actual implementation cash. Consent {}; earliest enactment {}.",p.total_cost.0,p.upfront.0,p.reserved.0,date(p.review_day.units()),date(p.enactment_day.units())));
                        ui.label(p.authority.label());
                        ui.label("Consent rechecks current governing backing. Withdrawal, refusal or a sponsoring-government change releases unspent reservations; paid work remains spent and the drafting cooldown remains.");
                        if ui.button("Propose funded statutory bill").clicked() { action=Some(Action::ProposeLaw{law:law.id.clone()}); }
                    }
                    Err(reason) => { ui.label(format!("Proposal condition: {reason}")); }
                }
            }
        });
    }
    for bill in state.bills.iter().filter(|bill| bill.status.pending()) {
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.strong(format!(
                "Bill #{} · {} · {}",
                bill.id.0,
                rules.law(&bill.law).expect("law").name,
                bill.status.label()
            ));
            ui.label(format!(
                "Paid {}m · protected {}m · consent {} · earliest enactment {}",
                bill.paid.0,
                bill.reserved.0,
                date(bill.review_day.units()),
                date(bill.enactment_day.units())
            ));
            ui.label(&bill.explanation);
            if ui
                .add_enabled(
                    bill.sponsor == world.player_country().government_number,
                    egui::Button::new("Withdraw bill · release unspent reservation"),
                )
                .clicked()
            {
                action = Some(Action::WithdrawLaw { request: bill.id });
            }
        });
    }
    let closed: Vec<_> = state
        .bills
        .iter()
        .rev()
        .filter(|bill| !bill.status.pending())
        .collect();
    if !closed.is_empty() {
        egui::CollapsingHeader::new(format!("Retained statutory outcomes ({})", closed.len()))
            .show(ui, |ui| {
                for bill in closed {
                    ui.label(format!(
                        "#{} · {} · {} · paid {}m · reserved {}m · closed {}",
                        bill.id.0,
                        rules.law(&bill.law).expect("law").name,
                        bill.status.label(),
                        bill.paid.0,
                        bill.reserved.0,
                        bill.closed_day
                            .map(|d| date(d.units()))
                            .unwrap_or_else(|| "Unknown".into())
                    ));
                    ui.label(&bill.explanation);
                }
            });
    }
    ui.separator();
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn funded_bill_controls_and_enacted_permission_render_at_two_scales_without_mutating_world() {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-legislation-test.json"
        ))
        .unwrap();
        for scale in [1.0, 1.4] {
            let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let input = egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1440.0 / scale, 940.0 / scale),
                )),
                ..Default::default()
            };
            let render = |world: &World, input: egui::RawInput| {
                let mut action = None;
                let output = ctx.run(input, |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| action = show(ui, &content, world));
                });
                (output, action)
            };
            let labels = |output: &egui::FullOutput| {
                output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(t) => Some(t.galley.text()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let click = |world: &World, label: &str| {
                let (output, action) = render(world, input.clone());
                assert!(action.is_none());
                let pos = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(t) if t.galley.text() == label => {
                            Some(t.pos + t.galley.size() / 2.0)
                        }
                        _ => None,
                    })
                    .expect("visible bill control");
                let mut result = None;
                for pressed in [true, false] {
                    let mut next = input.clone();
                    next.events = vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ];
                    result = render(world, next).1.or(result);
                }
                result.expect("bill control returns a normal command")
            };
            let hash = world.hash();
            let (output, _) = render(&world, input.clone());
            assert!(labels(&output).contains("Total 20m: pay 5m drafting now and protect 15m"));
            let propose = click(&world, "Propose funded statutory bill");
            assert!(matches!(propose, Action::ProposeLaw { .. }));
            assert_eq!(world.hash(), hash);
            world.issue(&content, propose).unwrap();
            let pending_hash = world.hash();
            let (output, _) = render(&world, input.clone());
            assert!(labels(&output).contains("Paid 5m · protected 15m"));
            let withdraw = click(&world, "Withdraw bill · release unspent reservation");
            assert!(matches!(
                withdraw,
                Action::WithdrawLaw {
                    request: LawBillId(1)
                }
            ));
            assert_eq!(world.hash(), pending_hash);
            let mut withdrawn = world.clone();
            withdrawn.issue(&content, withdraw).unwrap();
            assert_eq!(withdrawn.player_country().committed_cash, Money(0));
            world.advance(&content, DaySpan::new(21)).unwrap();
            let enacted_hash = world.hash();
            let (output, action) = render(&world, input.clone());
            assert!(action.is_none());
            assert!(labels(&output).contains(
                "Permission is in force; programme funding and implementation remain separate"
            ));
            assert!(!labels(&output).contains("Propose funded statutory bill"));
            assert_eq!(world.hash(), enacted_hash);
        }
    }
}
