use eframe::egui::{self, Ui};
use sim_core::*;
use std::collections::BTreeMap;
#[derive(Default)]
pub struct Panel {
    candidates: BTreeMap<OfficeId, PersonId>,
}
impl Panel {
    /// Only the controlled government may open its appointment workspace.
    pub fn show(&mut self, ui: &mut Ui, content: &Content, world: &World) -> Option<Action> {
        let rules = content.cabinet.as_ref()?;
        let state = world.player_country().cabinet.as_ref()?;
        let mut action = None;
        ui.separator();
        ui.heading("Cabinet & ministerial portfolios");
        ui.small("Nominees and portfolio rules are authored scenario content. Generated candidates are not historical ministers. Permanent civil service prepares proposals while a portfolio is unassigned.");
        ui.label("Ministerial skill changes preparation time for new proposals and amendments. Construction already commissioned keeps its delivery date; appointments do not create output or administrative capacity.");
        for office in rules
            .portfolios
            .iter()
            .filter(|p| p.country == world.player)
        {
            let holder = state
                .minister(&office.id)
                .and_then(|t| rules.candidate(&t.person));
            let pending_label = state
                .pending
                .iter()
                .find(|p| p.office == office.id)
                .map_or_else(String::new, |p| {
                    format!(" · appointment pending {}", date(p.decision_day))
                });
            egui::CollapsingHeader::new(format!("{} · {}{}",office.title,holder.map_or("Permanent civil service",|p|p.name.as_str()),pending_label)).id_salt(&office.id.0).show(ui,|ui| {
                ui.label(format!("Jurisdiction: {}. Current portfolio preparation: {} days, in addition to consent and construction.",office.policies.iter().filter_map(|p|content.policy(p)).map(|p|p.name.as_str()).collect::<Vec<_>>().join(", "),world.policy_preparation_days(content,&world.player,&office.policies[0])));
                if let Some(person)=holder {ui.label(format!("Incumbent skill {} · political acceptability {} (authored profiles)",person.skill,person.acceptability));}
                if let Some(pending)=state.pending.iter().find(|p|p.office==office.id) {
                    ui.label(format!("Appointment #{} pending to {} · paid {}m administration · sponsor cabinet #{}",pending.id.0,date(pending.decision_day),pending.paid.0,pending.sponsor));
                    if ui.button("Withdraw pending appointment · no refund").clicked() {action=Some(Action::CancelAppointment{request:pending.id});}
                }
                let candidates:Vec<_>=rules.candidates.iter().filter(|p|p.office==office.id).collect();
                let selected=self.candidates.entry(office.id.clone()).or_insert_with(||candidates[0].person.clone());
                if !candidates.iter().any(|p|p.person==*selected) {*selected=candidates[0].person.clone();}
                egui::ComboBox::from_id_salt((&office.id.0,"nominee")).selected_text(&rules.candidate(selected).expect("candidate").name).show_ui(ui,|ui| {
                    for person in &candidates {ui.selectable_value(selected,person.person.clone(),format!("{} · skill {} · acceptability {}",person.name,person.skill,person.acceptability));}
                });
                match world.preview_appointment(content,&world.player,&office.id,selected) {
                    Ok(preview)=>{
                        ui.label(format!("Pay {}m now · decision {} · {}",preview.fee.0,date(preview.decision_day),preview.route));
                        ui.label(format!("If approved: portfolio preparation {} days; elite backing -{}, council backing -{} (floor zero). Cooldown {} days from proposal.",preview.preparation_days.units(),preview.elite_cost,preview.council_cost,rules.cooldown_days.units()));
                        ui.small("Authority and sponsoring cabinet are rechecked on the decision date. Administration and cooldown remain after withdrawal or refusal.");
                        if ui.button("Propose this ministerial appointment").clicked() {action=Some(Action::Appoint{office:office.id.clone(),person:selected.clone()});}
                    }
                    Err(reason)=>{ui.label(format!("Cannot appoint: {reason}"));}
                }
                if holder.is_some() {
                    match world.preview_dismissal(content,&world.player,&office.id) {
                        Ok(fee)=>{
                            ui.label(format!("Dismissal costs {}m; civil-service preparation returns to {} days and a {}-day appointment cooldown starts.",fee.0,rules.vacancy_preparation_days.units(),rules.cooldown_days.units()));
                            if ui.button("Dismiss incumbent with these costs").clicked() {action=Some(Action::DismissMinister{office:office.id.clone()});}
                        }
                        Err(reason)=>{ui.label(format!("Cannot dismiss: {reason}"));}
                    }
                }
            });
        }
        egui::CollapsingHeader::new("Ministerial term history").show(ui, |ui| {
            for term in state.terms.iter().rev() {
                ui.label(format!(
                    "{} · {} · {} to {} · cabinet #{}",
                    rules.office(&term.office).expect("office").title,
                    rules.candidate(&term.person).expect("candidate").name,
                    date(term.since),
                    term.until.map_or_else(|| "present".into(), date),
                    term.government
                ));
            }
            if state.terms.is_empty() {
                ui.label("No approved ministerial appointments in this campaign.");
            }
        });
        action
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appointment_requires_a_costed_explicit_order_and_inspection_does_not_mutate_the_world() {
        for scale in [1.0, 1.4] {
            let content =
                Content::from_json(include_str!("../../../data/synthetic-cabinet-test.json"))
                    .unwrap();
            let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
            let expected = world.hash();
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            ctx.style_mut(|style| style.animation_time = 0.0);
            let mut panel = Panel::default();
            let input = egui::RawInput {
                focused: true,
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1440.0 / scale, 940.0 / scale),
                )),
                ..Default::default()
            };
            let mut action = None;
            let mut render = |input| {
                ctx.run(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        action = panel.show(ui, &content, &world);
                    });
                })
            };
            let position = |output: &egui::FullOutput, label: &str| {
                output
                    .shapes
                    .iter()
                    .find_map(|s| {
                        if let egui::Shape::Text(t) = &s.shape
                            && t.galley.text() == label
                        {
                            let center = t.pos + t.galley.size() / 2.0;
                            assert!(s.clip_rect.contains(center));
                            Some(center)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "Missing {label}: {:?}",
                            output
                                .shapes
                                .iter()
                                .filter_map(|s| if let egui::Shape::Text(t) = &s.shape {
                                    Some(t.galley.text().to_string())
                                } else {
                                    None
                                })
                                .collect::<Vec<_>>()
                        )
                    })
            };
            let click = |pos: egui::Pos2, pressed| egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..input.clone()
            };
            let output = render(input.clone());
            let pos = position(&output, "Finance ministry · Permanent civil service");
            render(click(pos, true));
            render(click(pos, false));
            let output = render(input.clone());
            assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().contains("Pay 20m now"))));
            assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().contains("elite backing -6, council backing -6"))));
            let pos = position(&output, "Propose this ministerial appointment");
            render(click(pos, true));
            render(click(pos, false));
            assert_eq!(
                action,
                Some(Action::Appoint {
                    office: OfficeId("portfolio:AUR:finance".into()),
                    person: PersonId("generated:AUR:finance:professional".into())
                })
            );
            assert_eq!(world.hash(), expected);
        }
    }
}
