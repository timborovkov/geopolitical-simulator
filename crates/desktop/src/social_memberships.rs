use eframe::egui;
use sim_core::*;
use std::collections::BTreeSet;

#[derive(Default)]
pub struct Panel {
    selected: BTreeSet<SocialGroupId>,
}
impl Panel {
    pub fn show(&mut self, ui: &mut egui::Ui, content: &Content, world: &World, id: &CountryId) {
        let Some(definition) = &content.social_memberships else {
            return;
        };
        self.selected
            .retain(|id| definition.catalogue.groups.contains_key(id));
        egui::CollapsingHeader::new("Social groups & overlap")
            .id_salt(("social-overlap", &id.0))
            .show(ui, |ui| self.inner(ui, content, world, id));
    }
    fn inner(&mut self, ui: &mut egui::Ui, content: &Content, world: &World, id: &CountryId) {
        let viewer = Viewer::Government(world.player.clone());
        let Ok(Some(mut report)) =
            world.social_membership_report(content, viewer.clone(), id, &self.selected)
        else {
            return;
        };
        if report.owned.is_none() {
            public_inner(ui, &report);
            return;
        }
        let mut changed = false;
        ui.label("These are live scenario projections from explicit joint profiles. A resident can have several memberships; adding group counts would double-count people.");
        egui::Grid::new(("social-membership-counts", &id.0))
            .num_columns(3)
            .show(ui, |ui| {
                ui.strong("Group · select to inspect overlap");
                ui.strong("Projected members");
                ui.strong("Share of unique residents");
                ui.end_row();
                let counts = report.owned.as_ref().expect("owner");
                for group in &report.groups {
                    let mut selected = self.selected.contains(&group.id);
                    let response = ui
                        .checkbox(&mut selected, &group.name)
                        .on_hover_text(&group.description);
                    if response.clicked() {
                        response.request_focus();
                    }
                    if response.changed() {
                        changed = true;
                        if selected {
                            self.selected.insert(group.id.clone());
                        } else {
                            self.selected.remove(&group.id);
                        }
                    }
                    let count = counts.members[&group.id];
                    ui.label(count.0.to_string());
                    if counts.population.0 > 0 {
                        ui.label(format!(
                            "{:.1}%",
                            count.0 as f64 * 100. / counts.population.0 as f64
                        ));
                    } else {
                        ui.label("No residents");
                    }
                    ui.end_row();
                }
            });
        if changed {
            let Ok(Some(next)) =
                world.social_membership_report(content, viewer, id, &self.selected)
            else {
                return;
            };
            report = next;
        }
        let counts = report.owned.as_ref().expect("owner");
        ui.strong(format!(
            "Unique modeled residents: {} · {}",
            counts.population.0,
            date(report.day)
        ));
        if self.selected.is_empty() {
            ui.label("Select at least one group to inspect overlap.");
        } else {
            for (label, count) in [
                ("Members of any selected group", counts.any_selected),
                ("Members of every selected group", counts.all_selected),
                (
                    "Members of none of the selected groups",
                    counts.none_selected,
                ),
            ] {
                ui.label(format!("{label}: {}", count.0));
            }
            egui::CollapsingHeader::new("Overlap within disjoint household cohorts").show(
                ui,
                |ui| {
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        egui::Grid::new(("social-cohort-counts", &id.0)).show(ui, |ui| {
                            for title in [
                                "Household cohort",
                                "Unique residents",
                                "Any selected",
                                "Every selected",
                                "None selected",
                            ] {
                                ui.strong(title);
                            }
                            ui.end_row();
                            for cohort in &counts.cohorts {
                                let name = content
                                    .country(id)
                                    .and_then(|country| country.household(&cohort.cohort))
                                    .map_or(cohort.cohort.0.as_str(), |group| group.name.as_str());
                                ui.label(name);
                                for count in [
                                    cohort.population,
                                    cohort.any_selected,
                                    cohort.all_selected,
                                    cohort.none_selected,
                                ] {
                                    ui.label(count.0.to_string());
                                }
                                ui.end_row();
                            }
                        })
                    });
                },
            );
        }
        ui.small("The employment group matches existing modeled jobs. Membership projections create no residents, cash, votes, powers or programme exemptions.");
        provenance(ui, &report);
    }
}
fn provenance(ui: &mut egui::Ui, report: &SocialMembershipReport) {
    egui::CollapsingHeader::new("Joint-profile assumptions & source").show(ui, |ui| {
        ui.label(&report.description);
        ui.small(format!("{} / {}", report.source, report.source_version));
    });
}
fn public_inner(ui: &mut egui::Ui, report: &SocialMembershipReport) {
    ui.label("No current membership counts are published for this country. These definitions are scenario assumptions; unknown memberships are not zero people.");
    for group in &report.groups {
        ui.label(&group.name).on_hover_text(&group.description);
    }
    provenance(ui, report);
}
pub fn show_public(ui: &mut egui::Ui, content: &Content, world: &World, id: &CountryId) {
    if content.social_memberships.is_none() {
        return;
    }
    egui::CollapsingHeader::new("Authored social-group definitions")
        .id_salt(("social-public-definitions", &id.0))
        .show(ui, |ui| {
            if let Ok(Some(report)) =
                world.social_membership_report(content, Viewer::Observer, id, &BTreeSet::new())
            {
                public_inner(ui, &report);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Content, World) {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-overlapping-memberships-test.json"
        ))
        .unwrap();
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        world.advance(&content, DaySpan::new(1)).unwrap();
        (content, world)
    }
    fn text(output: &egui::FullOutput) -> String {
        output
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::Shape::Text(t) => Some(t.galley.text()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    #[test]
    fn checkbox_overlap_selection_is_read_only_and_retains_ids_at_two_scales() {
        let (content, world) = fixture();
        let before = world.clone();
        for scale in [1., 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut panel = Panel::default();
            let mut frame = |events| {
                ctx.run(
                    egui::RawInput {
                        focused: true,
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1440. / scale, 1000. / scale),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default()
                            .show(ctx, |ui| panel.inner(ui, &content, &world, &world.player));
                    },
                )
            };
            for _ in 0..3 {
                let _ = frame(vec![]);
            }
            for label in ["Employed residents", "Parents and carers (15+ model)"] {
                let output = frame(vec![]);
                let pos = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(t) if t.galley.text() == label => {
                            let pos = t.pos + t.galley.size() / 2.;
                            shape.clip_rect.contains(pos).then_some(pos)
                        }
                        _ => None,
                    })
                    .expect("visible membership checkbox");
                for pressed in [true, false] {
                    let _ = frame(vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: Default::default(),
                        },
                    ]);
                }
            }
            let output = frame(vec![]);
            let labels = text(&output);
            assert!(labels.contains("Members of any selected group"));
            assert!(labels.contains("Members of every selected group"));
            assert_eq!(
                panel.selected,
                BTreeSet::from(["employment".into(), "parents".into()])
            );
            assert_eq!(world, before);
        }
    }
    #[test]
    fn foreign_panel_never_renders_current_private_role_counts_or_replaces_unknown_with_zero() {
        let (content, world) = fixture();
        let foreign = CountryId("BOR".into());
        for scale in [1., 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut panel = Panel {
                selected: BTreeSet::from(["employment".into()]),
            };
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1440. / scale, 1000. / scale),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| panel.inner(ui, &content, &world, &foreign));
                },
            );
            let labels = text(&output);
            assert!(labels.contains("No current membership counts are published"));
            assert!(!labels.contains("Unique modeled residents"));
            assert!(!labels.contains("Members of any selected group"));
        }
        let old = Content::bundled().unwrap();
        let old_world = World::new(&old, CountryId("GBR".into()), 17).unwrap();
        let ctx = egui::Context::default();
        let mut panel = Panel::default();
        let output = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                panel.show(ui, &old, &old_world, &old_world.player)
            });
        });
        assert!(!text(&output).contains("Social groups & overlap"));
    }
}
