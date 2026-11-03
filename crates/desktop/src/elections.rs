use eframe::egui::{self, Ui};
use sim_core::*;
/// Owner-only workspace; published dates/results use the public report instead.
pub fn show(ui: &mut Ui, content: &Content, world: &World) -> Option<Action> {
    let rules = content.elections.as_ref()?;
    let country = world.player_country();
    let state = country.electoral_authority.as_ref()?;
    let mandate = rules.mandate(&world.player)?;
    let mut action = None;
    ui.separator();
    ui.heading("Independent electoral authority");
    ui.strong(&mandate.title);
    ui.small("Mandates and procedures are authored scenario rules. Polls use the simplified 100-seat model; turnout, districts and electoral-integrity certification are not modeled.");
    ui.label(format!(
        "Current scheduled poll: {}",
        country
            .next_election
            .filter(|day| *day <= MAX_DAY)
            .map_or_else(|| "None within the playable calendar".into(), date)
    ));
    ui.label(format!("Government early-poll powers apply in: {}. Current authority/backing must also permit the request.",mandate.snap_forms.iter().map(|form|format!("{form:?}")).collect::<Vec<_>>().join(", ")));
    ui.label(format!("Independent review {} days · campaign {} days · minimum request/poll interval {} days · regular term {} calendar years. Next request permitted after {}.",rules.decision_days,rules.campaign_days,rules.minimum_interval_days,rules.regular_term_years,date(state.next_request_day)));
    if let Some(case) = state.pending() {
        ui.label(format!("Pending request #{} · paid {}m administration · decision {} · proposed poll {}. The prior poll date remains in force until approval.",case.id.0,case.paid.0,date(case.decision_day),date(case.vote_day)));
        if ui
            .button("Withdraw unapproved election request · no refund")
            .clicked()
        {
            action = Some(Action::WithdrawElectionRequest { request: case.id });
        }
    }
    if let Some(case) = state.announced() {
        ui.strong(format!("Announced poll: {}", date(case.vote_day)));
        ui.label("The government cannot withdraw an announced poll. An earlier mandatory confidence poll takes precedence; an explicit constitutional change can supersede the mandate.");
    }
    match world.preview_election(content, &world.player) {
        Ok(preview) => {
            ui.label(format!(
                "Pay {}m administration now · review {} · if approved, poll {} instead of {}.",
                preview.fee.0,
                date(preview.decision_day),
                date(preview.vote_day),
                preview
                    .previous_vote_day
                    .map_or_else(|| "no poll within the playable calendar".into(), date)
            ));
            ui.small("Review rechecks jurisdiction, current backing and cabinet sponsorship. The authority cannot postpone an earlier poll. Withdrawal/refusal retains the fee and request interval; approval fixes the announced date.");
            if ui.button("Submit costed early-election request").clicked() {
                action = Some(Action::RequestElection);
            }
        }
        Err(reason) => {
            ui.label(format!("Cannot request: {reason}"));
        }
    }
    egui::CollapsingHeader::new("Electoral decisions and modeled polls").show(ui, |ui| {
        if state.cases.is_empty() {
            ui.label("No early-election requests in this campaign.");
        }
        for case in state.cases.iter().rev() {
            ui.label(format!(
                "#{} · {:?} · proposed {} · review {} · poll {} · paid {}m",
                case.id.0,
                case.status,
                date(case.proposed_day),
                date(case.decision_day),
                date(case.vote_day),
                case.paid.0
            ));
        }
        if state.polls.is_empty() {
            ui.label("No modeled poll has completed since this campaign began.");
        }
        for poll in state.polls.iter().rev() {
            ui.strong(format!(
                "{} · modeled allocation · governing party {}",
                date(poll.day),
                poll.governing_party
            ));
            if !poll.contested {
                ui.label("Single listed party: no alternative allocation was modeled.");
            }
            ui.label(format!(
                "Cabinet #{} to #{}. Country play continues.",
                poll.government_before, poll.government_after
            ));
            for (party, seats) in &poll.seats {
                ui.label(format!(
                    "{}: {seats} seats",
                    content
                        .country(&world.player)
                        .expect("country")
                        .parties
                        .iter()
                        .find(|p| &p.id == party)
                        .map_or(party.as_str(), |p| p.name.as_str())
                ));
            }
        }
    });
    action
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn costed_election_preview_requires_an_explicit_order_and_announced_polls_have_no_withdrawal_control()
     {
        for scale in [1.0, 1.4] {
            let content =
                Content::from_json(include_str!("../../../data/synthetic-election-test.json"))
                    .unwrap();
            let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
            world.advance(&content, 365).unwrap();
            let expected = world.hash();
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
            let mut action = None;
            let output = ctx.run(input.clone(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    action = show(ui, &content, &world);
                });
            });
            assert!(action.is_none());
            assert_eq!(world.hash(), expected);
            assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().contains("Pay 50m administration now"))));
            let position = output
                .shapes
                .iter()
                .find_map(|s| {
                    if let egui::Shape::Text(t) = &s.shape
                        && t.galley.text() == "Submit costed early-election request"
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
                let click = egui::RawInput {
                    events: vec![
                        egui::Event::PointerMoved(position),
                        egui::Event::PointerButton {
                            pos: position,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                    ..input.clone()
                };
                let _ = ctx.run(click, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        action = show(ui, &content, &world);
                    });
                });
            }
            assert_eq!(action, Some(Action::RequestElection));
            assert_eq!(world.hash(), expected);
            world.issue(&content, action.take().unwrap()).unwrap();
            world.advance(&content, 7).unwrap();
            let expected = world.hash();
            let output = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    action = show(ui, &content, &world);
                });
            });
            assert!(action.is_none());
            assert_eq!(world.hash(), expected);
            assert!(!output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().contains("Withdraw unapproved"))));
        }
    }
}
