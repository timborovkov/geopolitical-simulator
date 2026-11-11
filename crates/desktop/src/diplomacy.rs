use eframe::egui::{self, Color32};
use sim_core::*;
use std::collections::BTreeMap;

pub struct Panel {
    campaign: Option<(u64, CountryId)>,
    target: CountryId,
    duration: u32,
    notice: u32,
    counter_terms: BTreeMap<(TreatyId, TreatyRevision), (u32, u32)>,
    review: Option<TreatyAction>,
}

impl Default for Panel {
    fn default() -> Self {
        Self {
            campaign: None,
            target: CountryId(String::new()),
            duration: 180,
            notice: 30,
            counter_terms: BTreeMap::new(),
            review: None,
        }
    }
}

fn country_name<'a>(content: &'a Content, id: &'a CountryId) -> &'a str {
    content
        .country(id)
        .map_or(id.0.as_str(), |country| country.name.as_str())
}
fn terms_controls(
    ui: &mut egui::Ui,
    duration: &mut u32,
    notice: &mut u32,
    rules: &DiplomacyDefinition,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Duration from activation, days");
        ui.add(
            egui::DragValue::new(duration)
                .range(rules.minimum_duration.units()..=rules.maximum_duration.units()),
        );
        *notice = (*notice).min(*duration);
        ui.label("Termination notice, days");
        ui.add(egui::DragValue::new(notice).range(0..=*duration));
    });
}
fn review_button(
    ui: &mut egui::Ui,
    label: &str,
    action: TreatyAction,
    requested: &mut Option<TreatyAction>,
) {
    if ui.button(label).clicked() {
        *requested = Some(action);
    }
}

impl Panel {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        content: &Content,
        world: Option<&World>,
        selected: &CountryId,
    ) -> Option<Action> {
        ui.heading("Diplomacy · bilateral agreements");
        let Some(world) = world else {
            ui.label("Begin or load a campaign to inspect agreements.");
            return None;
        };
        let key = (world.content_hash, world.player.clone());
        if self.campaign.as_ref() != Some(&key) {
            *self = Self {
                campaign: Some(key),
                ..Self::default()
            };
        }
        let Some(rules) = &content.diplomacy else {
            ui.label("Agreement negotiations are available in the fictional diplomacy exercise. Choose it under New campaign → World pack.");
            return None;
        };
        ui.label("Non-aggression pacts record mutual promises between two countries. This exercise has no military actions.");
        let own = selected == &world.player;
        let viewer = if own {
            Viewer::Government(world.player.clone())
        } else {
            Viewer::Observer
        };
        let records = match world.treaties_view(content, viewer, selected) {
            Ok(records) => records,
            Err(error) => {
                ui.colored_label(Color32::LIGHT_YELLOW, error);
                return None;
            }
        };
        let mut requested = None;
        if own {
            ui.label("An offer records your government's signature. The other party may sign, counteroffer or refuse. Both domestic ratifications must pass before scheduled activation.");
            let candidates: Vec<_> = content
                .countries
                .iter()
                .filter(|country| country.id != world.player)
                .collect();
            if !candidates.iter().any(|country| country.id == self.target)
                && let Some(country) = candidates.first()
            {
                self.target = country.id.clone();
            }
            if !candidates.is_empty() {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.strong("Draft a non-aggression offer");
                    egui::ComboBox::from_label("Other country")
                        .selected_text(country_name(content, &self.target))
                        .show_ui(ui, |ui| {
                            for country in &candidates {
                                ui.selectable_value(
                                    &mut self.target,
                                    country.id.clone(),
                                    &country.name,
                                );
                            }
                        });
                    terms_controls(ui, &mut self.duration, &mut self.notice, rules);
                    review_button(
                        ui,
                        "Review new offer",
                        TreatyAction::Offer {
                            with: self.target.clone(),
                            terms: NonAggressionTerms {
                                duration: DaySpan::new(self.duration),
                                notice: DaySpan::new(self.notice),
                            },
                        },
                        &mut requested,
                    );
                });
            }
        } else {
            ui.label(format!("{} · public signed-agreement inspector. Unaccepted negotiations and government finances are private.", country_name(content, selected)));
        }
        if records.is_empty() {
            ui.label("No agreements visible in this view.");
        }
        for record in records.iter().rev() {
            ui.push_id(("pact", record.id.0), |ui| {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.heading(format!("Pact #{} · revision {}", record.id.0, record.revision.0));
                    ui.strong(format!("{} / {} · {}", country_name(content, &record.parties[0]), country_name(content, &record.parties[1]), record.stage.label()));
                    ui.label(format!("Duration {} days from activation · termination notice {} days.", record.terms.duration.units(), record.terms.notice.units()));
                    if let Some(deadline) = record.offer_deadline { ui.label(format!("Respond before {} · offering party {}", date(deadline), country_name(content, &record.offered_by))); }
                    if let Some(day) = record.signed_on { ui.label(format!("Both signatures recorded {}. Signing alone does not activate the pact.", date(day))); }
                    if record.stage == TreatyStage::AwaitingRatification && let Some(day) = record.ratification_submission_deadline {
                        ui.label(format!("Each party must submit domestic ratification before {}. Already submitted reviews may finish later.", date(day)));
                    }
                    if let Some(day) = record.activation_day {
                        ui.label(format!("Scheduled activation {} · actual activation {}", date(day), record.activated_on.map(date).unwrap_or_else(|| "not activated".into())));
                    }
                    if let Some(day) = record.expires_at { ui.label(format!("Natural expiry {} · expiry day excluded from operation.", date(day))); }
                    if let Some(notice) = &record.notice { ui.label(format!("Notice by {} on {} · operative until {}", country_name(content, &notice.by), date(notice.day), date(notice.until))); }
                    if let Some(day) = record.closed_on { ui.label(format!("Closed {} · earlier administration remains paid.", date(day))); }
                    if record.signed_on.is_some() {
                        for request in &record.ratifications {
                            let status = if let Some(day) = request.passed { format!("passed {}", date(day)) }
                                else if record.stage.closed() { "not ratified before closure".into() }
                                else if let Some(day) = request.review { format!("submitted · review {}", date(day)) }
                                else { "not submitted".into() };
                            ui.label(format!("{} domestic ratification: {}", country_name(content, &request.country), status));
                        }
                    }
                    if let Some(detail) = &record.owned {
                        ui.small(format!("Your paid administration {}m · each new paid submission {}m · available national cash {}m. {}", detail.administration_paid.0, detail.service_charge.0, detail.available_national_cash.0, detail.authority));
                        match record.stage {
                            TreatyStage::Negotiating => {
                                if record.offered_by != world.player {
                                    ui.horizontal_wrapped(|ui| {
                                        review_button(ui, "Review signature", TreatyAction::Sign { agreement: record.id, revision: record.revision }, &mut requested);
                                        review_button(ui, "Review refusal", TreatyAction::Reject { agreement: record.id, revision: record.revision }, &mut requested);
                                    });
                                    let draft = self.counter_terms.entry((record.id, record.revision)).or_insert((record.terms.duration.units(), record.terms.notice.units()));
                                    ui.strong("Draft counteroffer");
                                    terms_controls(ui, &mut draft.0, &mut draft.1, rules);
                                    review_button(ui, "Review counteroffer", TreatyAction::Counteroffer { agreement: record.id, revision: record.revision,
                                        terms: NonAggressionTerms { duration: DaySpan::new(draft.0), notice: DaySpan::new(draft.1) } }, &mut requested);
                                } else { ui.label("Awaiting the other government's response."); }
                            }
                            TreatyStage::AwaitingRatification if detail.own_ratification.is_none() => {
                                review_button(ui, "Review domestic ratification", TreatyAction::Ratify { agreement: record.id, revision: record.revision }, &mut requested);
                            }
                            TreatyStage::Active => {
                                review_button(ui, "Review termination notice", TreatyAction::Terminate { agreement: record.id, revision: record.revision }, &mut requested);
                            }
                            _ => {}
                        }
                        if matches!(record.stage, TreatyStage::Negotiating | TreatyStage::AwaitingRatification | TreatyStage::Scheduled) {
                            review_button(ui, "Review withdrawal", TreatyAction::Withdraw { agreement: record.id, revision: record.revision }, &mut requested);
                        }
                    }
                    egui::CollapsingHeader::new("Agreement history").show(ui, |ui| {
                        for milestone in &record.milestones {
                            ui.label(format!("{} · revision {} · {}", date(milestone.day), milestone.revision.0, milestone.outcome.label()));
                        }
                    });
                });
            });
        }
        if let Some(action) = requested {
            self.review = Some(action);
        }
        if !own {
            return None;
        }
        let action = self.review.clone()?;
        let mut open = true;
        let mut cancel = false;
        let mut submit = false;
        egui::Window::new("Review diplomatic order")
            .collapsible(false)
            .default_width(520.0)
            .open(&mut open)
            .show(ui.ctx(), |ui| {
                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| {
                        ui.strong(action.label());
                        match world.preview_treaty(content, &action) {
                            Ok(plan) => {
                                ui.label(format!(
                                    "{} / {} · pact #{} revision {}",
                                    country_name(content, &plan.parties[0]),
                                    country_name(content, &plan.parties[1]),
                                    plan.agreement.0,
                                    plan.revision.0
                                ));
                                ui.label(format!(
                                    "Duration {} days · notice {} days",
                                    plan.terms.duration.units(),
                                    plan.terms.notice.units()
                                ));
                                ui.strong(format!(
                                    "Immediate administration: {}m",
                                    plan.immediate_cost.0
                                ));
                                ui.label(&plan.authority);
                                ui.label(format!(
                                    "After submission: {}",
                                    plan.stage_after_command.label()
                                ));
                                if let Some(day) = plan.ratification_review {
                                    ui.label(format!("Conditional domestic review {}", date(day)));
                                }
                                ui.label(&plan.explanation);
                                if ui.button("Submit this diplomatic order").clicked() {
                                    submit = true;
                                }
                            }
                            Err(error) => {
                                ui.colored_label(
                                    Color32::LIGHT_YELLOW,
                                    format!("Cannot submit: {error}"),
                                );
                            }
                        }
                        if ui.button("Cancel review").clicked() {
                            cancel = true;
                        }
                    });
            });
        if !open || cancel || submit {
            self.review = None;
        }
        submit.then_some(Action::Diplomacy { action })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};

    fn frame(
        ctx: &egui::Context,
        panel: &mut Panel,
        content: &Content,
        world: &World,
        selected: &CountryId,
        scale: f32,
        events: Vec<Event>,
    ) -> (egui::FullOutput, Option<Action>) {
        let mut order = None;
        let output = ctx.run(
            RawInput {
                focused: true,
                events,
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    Vec2::new(1280.0 / scale, 800.0 / scale),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    if let Some(action) = panel.show(ui, content, Some(world), selected) {
                        order = Some(action);
                    }
                });
            },
        );
        (output, order)
    }
    fn point(output: &egui::FullOutput, label: &str) -> Pos2 {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == label => {
                    Some(text.pos + Vec2::new(10.0, 5.0))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("Missing actual control: {label}"))
    }
    fn pointer(pos: Pos2, pressed: bool) -> Vec<Event> {
        vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                pressed,
                button: PointerButton::Primary,
                modifiers: Modifiers::NONE,
            },
        ]
    }
    fn labels(output: &egui::FullOutput) -> String {
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(text) => Some(text.galley.text()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn actual_review_and_submit_controls_require_two_clicks_and_foreign_inspection_is_public_at_both_scales()
     {
        for scale in [1.0, 1.4] {
            let content = Content::diplomacy_bundled().unwrap();
            let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            ctx.style_mut(|style| style.animation_time = 0.0);
            let mut panel = Panel::default();
            let selected = world.player.clone();
            let _ = frame(&ctx, &mut panel, &content, &world, &selected, scale, vec![]);
            let (output, _) = frame(&ctx, &mut panel, &content, &world, &selected, scale, vec![]);
            let review = point(&output, "Review new offer");
            let before = persistence::encode(&world, &content).unwrap();
            for pressed in [true, false] {
                let (_, order) = frame(
                    &ctx,
                    &mut panel,
                    &content,
                    &world,
                    &selected,
                    scale,
                    pointer(review, pressed),
                );
                assert!(order.is_none(), "Review must not submit");
            }
            assert_eq!(persistence::encode(&world, &content).unwrap(), before);
            let _ = frame(&ctx, &mut panel, &content, &world, &selected, scale, vec![]);
            let (output, order) =
                frame(&ctx, &mut panel, &content, &world, &selected, scale, vec![]);
            assert!(order.is_none());
            assert!(labels(&output).contains("Immediate administration: 10m"));
            let submit = point(&output, "Submit this diplomatic order");
            let (_, down) = frame(
                &ctx,
                &mut panel,
                &content,
                &world,
                &selected,
                scale,
                pointer(submit, true),
            );
            assert!(down.is_none());
            let (_, action) = frame(
                &ctx,
                &mut panel,
                &content,
                &world,
                &selected,
                scale,
                pointer(submit, false),
            );
            let action = action.expect("Explicit submit issues the reviewed order");
            assert!(
                matches!(&action, Action::Diplomacy { action: TreatyAction::Offer { with, .. } } if with == &CountryId("BOR".into()))
            );
            world.issue(&content, action).unwrap();
            assert_eq!(world.diplomacy.as_ref().unwrap().agreements.len(), 1);
            assert_eq!(
                world.diplomacy.as_ref().unwrap().agreements[&TreatyId(1)].administration_paid
                    [&world.player],
                Money(10)
            );
            assert!(
                frame(&ctx, &mut panel, &content, &world, &selected, scale, vec![])
                    .1
                    .is_none()
            );
            world.advance(&content, DaySpan::new(3)).unwrap();
            let before = persistence::encode(&world, &content).unwrap();
            let foreign = CountryId("BOR".into());
            let _ = frame(&ctx, &mut panel, &content, &world, &foreign, scale, vec![]);
            let (public, order) =
                frame(&ctx, &mut panel, &content, &world, &foreign, scale, vec![]);
            let public = labels(&public);
            assert!(public.contains("Pact #1"));
            assert!(public.contains("public signed-agreement inspector"));
            assert!(!public.contains("Draft a non-aggression offer"));
            assert!(!public.contains("Your paid administration"));
            assert!(!public.contains("Review domestic ratification"));
            assert!(order.is_none());
            assert_eq!(persistence::encode(&world, &content).unwrap(), before);
        }
    }
}
