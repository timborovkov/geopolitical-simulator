use eframe::egui::{self, Ui};
use sim_core::*;
use std::collections::BTreeMap;
#[derive(Default)]
pub struct Panel {
    projects: BTreeMap<String, Option<CommitmentId>>,
}
pub fn goal_label(content: &Content, goal: &PolicyDeliveryGoal) -> String {
    goal.description(content)
}

fn effect_label(effect: &PoliticalEffect) -> String {
    format!(
        "approval {:+} · legislature {:+} · elite {:+} · council {:+}",
        effect.approval, effect.legislative, effect.elite, effect.council
    )
}
impl Panel {
    pub fn show(&mut self, ui: &mut Ui, content: &Content, world: &World) -> Option<Action> {
        let rules = content.undertakings.as_ref()?;
        let state = world.player_country().undertakings.as_ref()?;
        let mut action = None;
        ui.heading("Delivery undertakings");
        ui.label(format!("Credibility {} / 100 · {} open of {}. Fulfilled undertakings add 5; breaches subtract 10; withdrawing an accepted undertaking subtracts 8; unfulfilled accepted undertakings closed by succession subtract 3. This score gates new undertakings; it does not increase production or capacity.",state.credibility,state.records.iter().filter(|r|matches!(r.status,UndertakingStatus::Pending|UndertakingStatus::Active)).count(),rules.maximum_open));
        ui.label("A declaration pays administration and promises a delivery target. Order and fund its policy separately. Acceptance may grant political backing; missing or withdrawing an accepted target incurs the published costs. Deadlines stay fixed when a policy is amended or withdrawn.");
        if let Some(latest) = state.records.last() {
            ui.label(format!(
                "Latest undertaking #{}: {} · review {} · fixed deadline {}",
                latest.id.0,
                latest.status.label(),
                date(latest.review_day),
                date(latest.deadline_day)
            ));
        }
        for template in &rules.templates {
            egui::CollapsingHeader::new(&template.name).id_salt(("undertaking",&template.id)).show(ui,|ui| {
                ui.label(format!("Review {} days · deliver within {} days after review · minimum credibility {} · elite {}, council {}. Declaration interval {} days for this kind.",template.review_days,template.deadline_days,template.minimum_credibility,template.elite_threshold,template.council_threshold,rules.cooldown_days));
                ui.label(format!("On acceptance: {}. On breach or accepted withdrawal: {}.",effect_label(&template.acceptance),effect_label(&template.breach)));
                let project=self.projects.entry(template.id.clone()).or_default();
                if template.kind==UndertakingKind::ProjectDeadline {
                    egui::ComboBox::from_id_salt(("deadline-project",&template.id)).selected_text(project.map_or("Choose an existing policy project".into(),|id|format!("Policy commitment #{}",id.0))).show_ui(ui,|ui| {
                        ui.selectable_value(project,None,"Choose an existing policy project");
                        for p in &world.player_country().projects {if p.policy!="coalition" {ui.selectable_value(project,Some(p.id),format!("#{} · {} · operating due {}",p.id,content.policy(&p.policy).expect("policy").name,date(p.due_day)));}}
                    });
                }
                match world.preview_undertaking(content,&world.player,&template.id,*project) {
                    Ok(p)=>{
                        ui.label(format!("Promise {} by {} · pay {}m administration now · review {}. No implementation funds are reserved by this declaration.",goal_label(content,&p.goal),date(p.deadline_day),p.fee.0,date(p.review_day)));
                        if let Some(due)=p.expected_delivery {ui.label(format!("Current conditional policy-delivery date: {}.",date(due)));if due>p.deadline_day {ui.colored_label(egui::Color32::LIGHT_YELLOW,"The current delivery date is after the promised deadline; this undertaking can fail.");}}
                        if let Some(reason)=&p.bottleneck {ui.small(reason);}
                        if ui.button("Submit undertaking").clicked(){action=Some(Action::DeclareUndertaking{template:template.id.clone(),project:*project});}
                    }
                    Err(reason)=>{ui.label(format!("Cannot declare: {reason}"));}
                }
            });
        }
        egui::CollapsingHeader::new("Undertaking history & delivery evidence").default_open(!state.records.is_empty()).show(ui,|ui| {
            if state.records.is_empty(){ui.label("No delivery undertakings declared.");}
            for record in state.records.iter().rev() {ui.push_id(record.id.0,|ui|{
                ui.strong(format!("#{} · {} · {}",record.id.0,goal_label(content,&record.goal),record.status.label()));
                ui.label(format!("Review {} · fixed deadline {} · paid {}m administration · attached policy {}",date(record.review_day),date(record.deadline_day),record.fee.0,record.linked_project.map_or("none".into(),|id|format!("#{}",id.0))));
                if matches!(record.status,UndertakingStatus::Pending|UndertakingStatus::Active)&&ui.button(format!("Withdraw undertaking #{}",record.id.0)).clicked(){action=Some(Action::WithdrawUndertaking{request:record.id});}
            });}
        });
        ui.separator();
        action
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn previews_show_separate_policy_delivery_and_require_explicit_submission_at_two_scales() {
        for scale in [1.0, 1.4] {
            let content = Content::from_json(include_str!(
                "../../../data/synthetic-undertakings-test.json"
            ))
            .unwrap();
            let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
            let hash = world.hash();
            let mut panel = Panel::default();
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            ctx.style_mut(|s| s.animation_time = 0.0);
            let input = egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1440.0 / scale, 940.0 / scale),
                )),
                ..Default::default()
            };
            let mut action = None;
            let draw = |panel: &mut Panel, action: &mut Option<Action>, input: egui::RawInput| {
                ctx.run(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        *action = panel.show(ui, &content, &world);
                    });
                })
            };
            let position = |o: &egui::FullOutput, label: &str| {
                o.shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Text(t) if t.galley.text() == label => {
                            let point = t.pos + t.galley.size() / 2.0;
                            assert!(s.clip_rect.contains(point));
                            Some(point)
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("Missing {label}"))
            };
            let output = draw(&mut panel, &mut action, input.clone());
            let header = position(&output, "Public energy delivery pledge");
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
                draw(&mut panel, &mut action, click);
            }
            let output = draw(&mut panel, &mut action, input.clone());
            assert!(action.is_none());
            assert_eq!(world.hash(), hash);
            assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().contains("pay 20m administration now")&&t.galley.text().contains("No implementation funds are reserved"))));
            let button = position(&output, "Submit undertaking");
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
                draw(&mut panel, &mut action, click);
            }
            assert_eq!(
                action,
                Some(Action::DeclareUndertaking {
                    template: "energy-pledge".into(),
                    project: None
                })
            );
            assert_eq!(world.hash(), hash);
        }
    }
}
