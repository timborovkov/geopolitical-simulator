use eframe::egui::{self, Ui};
use sim_core::*;
use std::collections::BTreeMap;
#[derive(Default)]
pub struct Panel {
    settings: BTreeMap<RegionId, (i32, u32, i32)>,
}
pub fn tier(tier: &RegionalTier) -> &'static str {
    match tier {
        RegionalTier::Region => "Regional",
        RegionalTier::Municipality => "Municipal",
        RegionalTier::AutonomousJurisdiction => "Autonomous jurisdiction",
    }
}
impl Panel {
    pub fn show(&mut self, ui: &mut Ui, content: &Content, world: &World) -> Option<Action> {
        let rules = content.subnational.as_ref()?;
        let charter = rules.charter(&world.player)?;
        let c = world.player_country();
        let state = c.subnational.as_ref()?;
        let mut action = None;
        ui.separator();
        ui.heading("Subnational powers & public funds");
        ui.label(match charter.form {
            TerritorialForm::Federation => "Federal territorial charter",
            TerritorialForm::DecentralizedUnitary => "Decentralized unitary charter",
            TerritorialForm::DelegatedUnitary => "Unitary charter with delegated administration",
        });
        ui.small("These are authored budget jurisdictions, with no historical regional boundary claims. Their population shares partition this scenario's country; regional geography and separate regional household groups are not modeled.");
        ui.label(format!("Consolidated public cash {}m · protected local funds/escrow {}m · national uncommitted cash {}m.",c.accounts.treasury.0,c.regional_protected_cash().0,c.available_cash().0));
        ui.label("Tax entitlements stay with local authorities. National orders, fees and debt repayment cannot spend them. Base purchases are placed only when funded; grants pre-fund additional work for a fixed term.");
        ui.small("Service purchasing levels are annual GDP points weighted by the jurisdiction's population share. Payments reach public-service households; they do not add physical capacity or certify service quality/staffing. Public accounts consolidate both levels of government.");
        for mandate in &charter.regions {
            let local = state
                .regions
                .iter()
                .find(|r| r.id == mandate.id)
                .expect("region");
            let settings = self.settings.entry(mandate.id.clone()).or_insert((
                2,
                30,
                (local.service_level + 1).min(8),
            ));
            egui::CollapsingHeader::new(format!("{} · {}",mandate.name,if mandate.autonomous_services {"Autonomous services"} else {"Delegated services"})).id_salt(&mandate.id.0).show(ui,|ui| {
                ui.label(format!("{} authority · population share {:.1}% · entitlement {:.1}% of collected income tax · base purchasing level {}.",tier(&mandate.tier),mandate.population_share_bps as f64/100.0,mandate.tax_entitlement_bps as f64/100.0,local.service_level));
                ui.label(format!("Protected local base cash {}m · tax received {}m · base purchases paid {}m. Next national order after {}.",local.cash.0,local.tax_received.0,local.base_spent.0,date(local.next_order_day)));
                if let Some(last)=&local.last {ui.label(format!("{}: entitled receipts {}m · base requested/paid {}/{}m · funded supplement paid {}m.",date(last.day),last.tax_receipts.0,last.requested_base.0,last.paid_base.0,last.paid_supplement.0));}
                for order in state.orders.iter().filter(|o|o.region==mandate.id && matches!(o.status,RegionalOrderStatus::Pending|RegionalOrderStatus::Implementing)) {
                    ui.label(format!("#{} · {:?} · review {} · administration {}m · escrow {}m · local grant remaining {}m.",order.id.0,order.status,date(order.decision_day),order.paid_administration.0,order.escrow.0,order.remaining.0));
                    if order.status==RegionalOrderStatus::Pending && ui.button(format!("Withdraw pending regional order #{}",order.id.0)).clicked() {action=Some(Action::CancelRegionalOrder {order:order.id});}
                    if order.status==RegionalOrderStatus::Implementing {ui.label(format!("Accepted local grant: {}m/day through {}. National government cannot reclaim or redirect these funds.",order.daily_supplement.0,date(order.end_day.expect("term"))));}
                }
                ui.strong("Offer a funded service supplement");
                ui.horizontal_wrapped(|ui| {ui.label("Additional levels");ui.add(egui::DragValue::new(&mut settings.0).range(1..=3));ui.label("Days");ui.add(egui::DragValue::new(&mut settings.1).range(30..=180));});
                let kind=RegionalOrderKind::Grant {boost:settings.0,days:settings.1};
                match world.preview_regional_order(content,&world.player,&mandate.id,&kind) {
                    Ok(preview)=>{
                        ui.label(format!("Pay {}m administration now · protect {}m escrow · local review {} · if accepted, {}m/day through {}.",preview.fee.0,preview.principal.0,date(preview.decision_day),preview.daily_supplement.0,date(preview.end_day.expect("term"))));
                        ui.small("Review rechecks national backing, cabinet sponsorship and the local cash absorption mandate. Withdrawal/refusal releases escrow; administration and the order interval remain. Accepted grants survive national succession.");
                        if ui.button("Offer funded regional grant").clicked() {action=Some(Action::RegionalOrder {region:mandate.id.clone(),kind});}
                    }
                    Err(reason)=>{ui.label(format!("Cannot offer: {reason}"));}
                }
                if mandate.autonomous_services {ui.label("This authority controls its own base service level. Your government can offer a funded grant, but cannot order a different base level.");}
                else {
                    ui.strong("Delegated base service order");
                    ui.horizontal_wrapped(|ui| {ui.label("Base level");ui.add(egui::DragValue::new(&mut settings.2).range(0..=8));});
                    let kind=RegionalOrderKind::Directive {level:settings.2};
                    match world.preview_regional_order(content,&world.player,&mandate.id,&kind) {
                        Ok(preview)=>{ui.label(format!("Pay {}m administration · current national consent completes {}. Local base funds constrain actual purchases.",preview.fee.0,date(preview.decision_day)));if ui.button("Propose delegated service level").clicked() {action=Some(Action::RegionalOrder {region:mandate.id.clone(),kind});}}
                        Err(reason)=>{ui.label(format!("Cannot order: {reason}"));}
                    }
                }
            });
        }
        egui::CollapsingHeader::new("Regional decisions & funded grant terms").show(ui,|ui| {
            if state.orders.is_empty() {ui.label("No national regional orders in this campaign.");}
            for order in state.orders.iter().rev() {ui.label(format!("#{} · {} · {:?} · {:?} · review {} · administration {}m · grant principal/spent/remaining {}/{}/{}m · escrow {}m",order.id.0,order.region.0,order.kind,order.status,date(order.decision_day),order.paid_administration.0,order.principal.0,order.spent.0,order.remaining.0,order.escrow.0));}
        });
        action
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_local_grant_preview_requires_an_explicit_order_and_shows_its_separate_fee_and_escrow() {
        for scale in [1.0, 1.4] {
            let content = Content::from_json(include_str!(
                "../../../data/synthetic-subnational-test.json"
            ))
            .unwrap();
            let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
            let original = world.hash();
            let mut panel = Panel::default();
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            ctx.style_mut(|style| style.animation_time = 0.0);
            let input = egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1440.0 / scale, 940.0 / scale),
                )),
                ..Default::default()
            };
            let mut action = None;
            let draw = |panel: &mut Panel,
                        action: &mut Option<Action>,
                        world: &World,
                        input: egui::RawInput| {
                ctx.run(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        *action = panel.show(ui, &content, world);
                    });
                })
            };
            let position = |output: &egui::FullOutput, label: &str| {
                output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == label => {
                            let point = text.pos + text.galley.size() / 2.0;
                            assert!(shape.clip_rect.contains(point));
                            Some(point)
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "Missing {label}: {:?}",
                            output
                                .shapes
                                .iter()
                                .filter_map(|shape| match &shape.shape {
                                    egui::Shape::Text(text) => Some(text.galley.text()),
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                        )
                    })
            };
            let output = draw(&mut panel, &mut action, &world, input.clone());
            let header = position(
                &output,
                "Fictional AUR provincial authority · Autonomous services",
            );
            for pressed in [true, false] {
                let mut click = input.clone();
                click.events = vec![
                    egui::Event::PointerMoved(header),
                    egui::Event::PointerButton {
                        pos: header,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ];
                draw(&mut panel, &mut action, &world, click);
            }
            let output = draw(&mut panel, &mut action, &world, input.clone());
            assert!(action.is_none());
            assert_eq!(world.hash(), original);
            assert!(output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text().contains("Pay 20m administration now · protect 90m escrow"))));
            let button = position(&output, "Offer funded regional grant");
            for pressed in [true, false] {
                let mut click = input.clone();
                click.events = vec![
                    egui::Event::PointerMoved(button),
                    egui::Event::PointerButton {
                        pos: button,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ];
                draw(&mut panel, &mut action, &world, click);
            }
            assert_eq!(world.hash(), original);
            assert_eq!(
                action,
                Some(Action::RegionalOrder {
                    region: RegionId("region:AUR:province".into()),
                    kind: RegionalOrderKind::Grant { boost: 2, days: 30 }
                })
            );
            world.issue(&content, action.take().unwrap()).unwrap();
            world.advance(&content, 14).unwrap();
            let expected = world.hash();
            let output = draw(&mut panel, &mut action, &world, input);
            assert!(action.is_none());
            assert_eq!(world.hash(), expected);
            assert!(output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text().contains("Accepted local grant: 3m/day"))));
            assert!(!output.shapes.iter().any(|shape|matches!(&shape.shape,egui::Shape::Text(text) if text.galley.text().starts_with("Withdraw pending regional order"))));
        }
    }
}
