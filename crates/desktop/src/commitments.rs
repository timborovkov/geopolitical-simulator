use eframe::egui::{self, Ui};
use sim_core::*;
use std::collections::BTreeMap;
#[derive(Default)]
pub struct Panel {
    levels: BTreeMap<CommitmentId, i32>,
    focused: Option<CommitmentId>,
    scroll_to_focus: bool,
}
impl Panel {
    pub fn focus(&mut self, request: CommitmentId) {
        self.focused = Some(request);
        self.scroll_to_focus = true;
    }
    /// Owner workspace only; read-only rendering returns an explicit command.
    pub fn show(&mut self, ui: &mut Ui, content: &Content, world: &World) -> Option<Action> {
        let records = world.player_country().commitments.as_ref()?;
        let mut action = None;
        let heading = ui.heading("Commitments & outcomes");
        let reveal_history = self.scroll_to_focus;
        if self.scroll_to_focus {
            ui.scroll_to_rect_animation(
                heading.rect,
                Some(egui::Align::TOP),
                egui::style::ScrollAnimation::none(),
            );
            self.scroll_to_focus = false;
        }
        if let Some(request) = self.focused {
            ui.horizontal_wrapped(|ui| {
                ui.strong(format!("Inspecting programme #{request}"));
                if ui.button("Show all programmes").clicked() {
                    self.focused = None;
                }
            });
            if world.player_country().commitment(&request).is_none() {
                ui.label("Programme is unavailable in your government's view.");
                return None;
            }
        }
        ui.small("Amendment restarts consent and delivery with a paid administration fee. Withdrawal releases only money still reserved; contractors retain payments for work already commissioned.");
        for p in world
            .player_country()
            .projects
            .iter()
            .filter(|p| self.focused.is_none_or(|id| p.id == id))
        {
            ui.push_id(p.id,|ui| {egui::Frame::group(ui.style()).show(ui,|ui| {
                ui.strong(format!("#{} · {} · level {}",p.id,content.policy(&p.policy).expect("policy").name,p.value));
                ui.label(format!("{} · consent {} · operating due {} · reserved {}m",p.stage.label(),date((p.consent_day).units()),date((p.due_day).units()),p.reserved.0));
                let progress=world.day.units().saturating_sub((p.start_day).units()) as f32/((p.due_day).units()-(p.start_day).units()).max(1) as f32;
                ui.add(egui::ProgressBar::new(progress).show_percentage());
                if p.stage==PolicyStage::AwaitingConsent && matches!(p.policy.as_str(),"income_tax"|"services"|"welfare") {
                    let value=self.levels.entry(p.id).or_insert(p.value);
                    let definition=content.policy(&p.policy).expect("policy");
                    ui.add(egui::Slider::new(value,definition.min..=definition.max).text("Amended level (%)"));
                    match world.preview_amendment(content,&world.player,p.id,*value) {
                        Ok(preview)=>{
                            ui.label(format!("Pay {}m administration now; consent restarts to {}, delivery {}. {}",preview.fee.0,date((preview.consent_day).units()),date((preview.due_day).units()),preview.route));
                            if preview.preparation_days.units() > 0 { ui.label(format!("Portfolio preparation adds {} days to the restarted delivery process.",preview.preparation_days.units())); }
                            ui.label(format!("After delivery versus current operating levels: annual consolidated net expenditure change {:+}m (forecast).",preview.annual_budget_change.0));
                            for impact in &preview.household_impacts {ui.label(format!("{}: {:+}m/day disposable income",impact.group,impact.daily_disposable_change.0));}
                            ui.small("Forecast holds opening GDP, collection and funded wages constant; shocks, prices and other pending changes can alter outcomes.");
                            if ui.button("Submit paid amendment").clicked() {action=Some(Action::AmendCommitment{request:p.id,value:*value});}
                        }
                        Err(reason)=>{ui.small(format!("Amendment: {reason}"));}
                    }
                }
                match world.preview_cancellation(content,&world.player,p.id) {
                    Ok(preview)=>{
                        ui.label(format!("Withdrawal releases {}m reserved cash; {}m already paid stays spent. No refund or completed capacity is granted.",preview.released.0,preview.sunk.0));
                        if ui.button("Withdraw this pending commitment").clicked() {action=Some(Action::CancelCommitment{request:p.id});}
                    }
                    Err(reason)=>{ui.small(format!("Cannot withdraw: {reason}"));}
                }
            });});
        }
        if world
            .player_country()
            .projects
            .iter()
            .all(|p| self.focused.is_some_and(|id| p.id != id))
        {
            ui.label(if self.focused.is_some() {
                "No pending work for this programme."
            } else {
                "No pending commitments. Draft a programme above."
            });
        }
        let count = if self.focused.is_some() {
            1
        } else {
            records.len()
        };
        egui::CollapsingHeader::new(format!("Programme history · {count} record{}",if count == 1 { "" } else { "s" })).default_open(true).open(reveal_history.then_some(true)).show(ui,|ui| {
            for r in records.iter().rev().filter(|r| self.focused.is_none_or(|id| r.id == id)) {
                egui::CollapsingHeader::new(format!("#{} · {} · {:?} · paid {}m",r.id.0,content.policy(&r.project.policy).expect("policy").name,r.stage(),r.paid.0)).id_salt(r.id.0).open(self.focused.map(|id| id == r.id)).show(ui,|ui| {
                    for m in &r.milestones {ui.label(format!("{} · {:?} · level {}",date(m.day.units()),m.stage,m.value));}
                    for e in &r.direct_effects {ui.label(format!("Direct delivered change: {} from {} to {}",effect_label(&e.field),e.before,e.after));}
                    if let Some(day)=r.review_due {ui.label(format!("Observed outcome review due {}",date((day).units())));}
                    if let Some(e)=&r.evaluation {
                        ui.strong(format!("Observed review · {}",date(e.day.units())));
                        if let Some(before)=&e.before {ui.label(format!("Before delivery: revenue {}m/day · spending {}m/day · food {}% · energy {}% · external disruption {} capacity points",before.revenue.0,before.spending.0,before.food_coverage,before.energy_coverage,before.shock));}
                        else {ui.label("No pre-delivery settlement was available at this bookmark.");}
                        ui.label(format!("Review day: revenue {}m/day · spending {}m/day · food {}% · energy {}% · external disruption {} capacity points",e.after.revenue.0,e.after.spending.0,e.after.food_coverage,e.after.energy_coverage,e.after.shock));
                        ui.label(format!("Approval {}% to {}%; {} other programme deliveries during this window.",e.approval_before,e.approval_after,e.other_deliveries));
                        ui.small("Observed differences also reflect prices, other policies and external shocks. They are not an isolated causal effect or a forecast guarantee.");
                    }
                });
            }
        });
        action
    }
}

pub(crate) fn effect_label(field: &str) -> &str {
    match field {
        "income_tax" => "Income-tax policy setting (%)",
        "services" => "Services (% of GDP)",
        "welfare" => "Welfare (% of GDP)",
        "energy_capacity" => "Domestic energy capacity (%)",
        "food_capacity" => "Domestic food capacity (%)",
        "administration" => "Administration (%)",
        "legislative_support" => "Legislative support",
        "elite_support" => "Elite backing",
        "council_support" => "Council backing",
        _ => field,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn withdrawal_shows_sunk_cost_and_returns_an_explicit_order_without_mutating_the_world() {
        for scale in [1.0, 1.4] {
            let content =
                Content::from_json(include_str!("../../../data/synthetic-commitment-test.json"))
                    .unwrap();
            let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
            world
                .issue(
                    &content,
                    Action::Enact {
                        policy: "income_tax".into(),
                        value: 30,
                    },
                )
                .unwrap();
            let expected = world.hash();
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut panel = Panel::default();
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1440.0 / scale, 940.0 / scale),
                )),
                focused: true,
                ..Default::default()
            };
            let mut action = None;
            let output = ctx.run(input.clone(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    action = panel.show(ui, &content, &world);
                });
            });
            assert!(action.is_none());
            assert_eq!(world.hash(), expected);
            assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().contains("2m already paid stays spent"))));
            let position = output
                .shapes
                .iter()
                .find_map(|s| {
                    if let egui::Shape::Text(t) = &s.shape
                        && t.galley.text() == "Withdraw this pending commitment"
                    {
                        let center = t.pos + t.galley.size() / 2.0;
                        assert!(s.clip_rect.contains(center));
                        Some(center)
                    } else {
                        None
                    }
                })
                .unwrap();
            for pressed in [true, false] {
                let mut click = input.clone();
                click.events = vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ];
                let _ = ctx.run(click, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        action = panel.show(ui, &content, &world);
                    });
                });
            }
            assert_eq!(
                action,
                Some(Action::CancelCommitment {
                    request: CommitmentId(1)
                })
            );
            assert_eq!(world.hash(), expected);
        }
    }
}
