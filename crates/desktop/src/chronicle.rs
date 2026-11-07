use eframe::egui;
use sim_core::*;

#[derive(Default)]
pub struct Panel {
    command_reference: String,
    command_selected: Option<CommandId>,
    command_error: Option<String>,
    reference: String,
    selected: Option<EventId>,
    error: Option<String>,
}
impl Panel {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        content: &Content,
        world: Option<&World>,
    ) -> Option<CommitmentId> {
        let mut inspect = None;
        ui.heading("Chronicle & decisions");
        ui.label("Accepted outcomes, delivery dates and AI explanations. Most recent first; filtered to the selected country.");
        let Some(world) = world else {
            ui.label("Your chronicle begins with a mandate.");
            return None;
        };
        if content.governing_institutions.is_some() {
            egui::CollapsingHeader::new("Command governing context").show(ui, |ui| {
                ui.label("Inspect an accepted command from your government. This records context when submitted; consent or approval may happen later.");
                ui.horizontal_wrapped(|ui| {
                    ui.label("Command ID");
                    ui.add(egui::TextEdit::singleline(&mut self.command_reference).desired_width(100.0).char_limit(20));
                    if ui.button("Find command").clicked() {
                        match self.command_reference.trim().parse::<u64>().ok().filter(|id| *id > 0) {
                            Some(id) => { self.command_selected = Some(CommandId(id)); self.command_error = None; },
                            None => { self.command_selected = None; self.command_error = Some("Enter a positive command ID".into()); },
                        }
                    }
                });
                if let Some(error) = &self.command_error { ui.colored_label(egui::Color32::LIGHT_YELLOW, error); }
                if let Some(command) = self.command_selected {
                    match world.command_governing_context(content, Viewer::Government(world.player.clone()), command) {
                        Ok(Some(context)) => {
                            ui.label(format!("Command #{} · government #{} · {:?}", command.0, context.government, context.route));
                            if let Some(definition) = &content.governing_institutions {
                                if let Some(primary) = definition.institution(&context.primary) { ui.strong(&primary.name); }
                                ui.small(&context.primary.0);
                                if let Some(council) = context.council.and_then(|id| definition.institution(&id)) { ui.label(format!("Consultative context: {}", council.name)); }
                            }
                        },
                        Ok(None) => { ui.label("No governing context available for this command."); },
                        Err(reason) => { ui.colored_label(egui::Color32::LIGHT_YELLOW, reason); },
                    }
                }
                ui.small("Recent accepted command IDs");
                ui.horizontal_wrapped(|ui| {
                    for entry in world.journal.iter().rev().filter(|e| e.command.issuer == world.player && e.governing_context.is_some()).take(8) {
                        if ui.button(format!("#{}", entry.command.id.0)).clicked() { self.command_selected = Some(entry.command.id); self.command_reference = entry.command.id.0.to_string(); self.command_error = None; }
                    }
                });
            });
        }
        if world.event_sequences.is_some() {
            ui.label("References belong to this campaign. Only your government's retained entries are available; older entries may have expired.");
            ui.horizontal_wrapped(|ui| {
                ui.label("Event reference");
                ui.add(
                    egui::TextEdit::singleline(&mut self.reference)
                        .desired_width(220.0)
                        .char_limit(192)
                        .hint_text(format!("{}#12", world.player.0)),
                );
                if ui.button("Find event").clicked() {
                    match self.reference.parse::<EventId>() {
                        Ok(id) => {
                            self.selected = Some(id);
                            self.error = None;
                        }
                        Err(reason) => {
                            self.selected = None;
                            self.error = Some(reason);
                        }
                    }
                }
            });
            if let Some(reason) = &self.error {
                ui.colored_label(egui::Color32::LIGHT_YELLOW, reason);
            }
            if let Some(id) = &self.selected {
                ui.push_id("event_reference_result", |ui| {
                    match world.event_by_reference(
                        content,
                        Viewer::Government(world.player.clone()),
                        id,
                    ) {
                        Ok(Some(event)) => {
                            ui.strong("Referenced event");
                            inspect = show_event(ui, event).or(inspect.take());
                        }
                        Ok(None) => {
                            ui.label(
                                "No retained event with that reference in your government's view.",
                            );
                        }
                        Err(reason) => {
                            ui.colored_label(egui::Color32::LIGHT_YELLOW, reason);
                        }
                    }
                });
                ui.separator();
            }
        }
        ui.push_id("event_reference_history", |ui| {
            for event in world
                .events
                .iter()
                .rev()
                .filter(|e| e.country == world.player)
            {
                inspect = show_event(ui, event).or(inspect.take());
            }
        });
        if let Some(id) = inspect {
            match world.event_commitment(content, Viewer::Government(world.player.clone()), &id) {
                Ok(Some(record)) => return Some(record.id),
                Ok(None) => {
                    self.error = Some("Programme is unavailable in your government's view.".into())
                }
                Err(reason) => self.error = Some(reason),
            }
        }
        None
    }
}
fn show_event(ui: &mut egui::Ui, event: &Event) -> Option<EventId> {
    let row = |ui: &mut egui::Ui| {
        let mut inspect = None;
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    if event.important {
                        super::MINT
                    } else {
                        super::MUTED
                    },
                    format!(
                        "{} · {}",
                        date(event.day.units()),
                        if event.important {
                            "DECISION / ALERT"
                        } else {
                            "REPORT"
                        }
                    ),
                );
                if let Some(id) = &event.id {
                    ui.label(format!("Reference {id}"));
                    if ui.button("Copy reference").clicked() {
                        ui.ctx().copy_text(id.to_string());
                    }
                }
            });
            ui.label(&event.text);
            if let Some(EventContext::Commitment { request, stage }) = event.context {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("Recorded stage: {stage:?}"));
                    if ui.button(format!("Inspect programme #{request}")).clicked() {
                        inspect = event.id.clone();
                    }
                });
            }
        });
        inspect
    };
    if let Some(id) = &event.id {
        ui.push_id(id.to_string(), row).inner
    } else {
        row(ui)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lookup_and_copy_at_two_scales_preserve_the_world_and_hide_foreign_entries() {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-event-references-test.json"
        ))
        .unwrap();
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        world.advance(&content, sim_core::DaySpan::new(30)).unwrap();
        let foreign = world
            .events
            .iter()
            .find(|e| e.country.0 == "BOR")
            .unwrap()
            .id
            .as_ref()
            .unwrap()
            .to_string();
        let before = world.hash();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_pixels_per_point(scale);
            let mut panel = Panel {
                reference: "AUR#1".into(),
                ..Default::default()
            };
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0 / scale, 3500.0 / scale),
                )),
                ..Default::default()
            };
            let frame = |panel: &mut Panel, input: egui::RawInput| {
                ctx.run(input, |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| panel.show(ui, &content, Some(&world)));
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
            let click = |panel: &mut Panel, output: &egui::FullOutput, label: &str| {
                let pos = output
                    .shapes
                    .iter()
                    .find_map(|s| {
                        if let egui::Shape::Text(t) = &s.shape {
                            (t.galley.text() == label).then_some(t.pos + egui::vec2(4.0, 4.0))
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| panic!("Missing button {label}"));
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
                frame(panel, clicked)
            };
            let initial = frame(&mut panel, input.clone());
            let _ = click(&mut panel, &initial, "Find event");
            let found = frame(&mut panel, input.clone());
            assert!(text(&found).contains("Referenced event"));
            let copied = click(&mut panel, &found, "Copy reference");
            assert!(
                copied
                    .platform_output
                    .commands
                    .iter()
                    .any(|c| matches!(c,egui::OutputCommand::CopyText(value) if value=="AUR#1"))
            );
            panel.reference = foreign.clone();
            let current = frame(&mut panel, input.clone());
            let _ = click(&mut panel, &current, "Find event");
            let missing = frame(&mut panel, input.clone());
            let visible = text(&missing);
            assert!(
                visible.contains("No retained event with that reference in your government's view")
            );
            assert!(!visible.contains("Referenced event"));
            assert!(!visible.contains(&format!("Reference {foreign}")));
            assert_eq!(world.hash(), before);
        }
    }
}

#[cfg(test)]
mod context_tests {
    use super::*;
    #[test]
    fn owner_can_open_and_inspect_recent_command_context_without_issuing_orders() {
        let content = Content::governing_institutions_bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        world
            .issue(&content, Action::Repay { amount: Money(1) })
            .unwrap();
        let before = world.clone();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            ctx.style_mut(|style| style.animation_time = 0.0);
            let mut panel = Panel::default();
            let mut frame = |events| {
                ctx.run(
                    egui::RawInput {
                        focused: true,
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1440.0 / scale, 940.0 / scale),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            panel.show(ui, &content, Some(&world));
                        });
                    },
                )
            };
            let locate = |output: &egui::FullOutput, label: &str| {
                output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == label => {
                            let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
                            shape.clip_rect.contains_rect(rect).then_some(rect.center())
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("Visible control missing: {label}"))
            };
            frame(vec![]);
            for label in ["Command governing context", "#1"] {
                let point = locate(&frame(vec![]), label);
                for pressed in [true, false] {
                    frame(vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ]);
                }
            }
            let output = frame(vec![]);
            let labels = output
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(t) => Some(t.galley.text()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                labels.contains("Command #1 · government #1 · Legislature"),
                "{labels}"
            );
            assert!(labels.contains("Modeled national legislature"), "{labels}");
            assert_eq!(world, before);
        }
    }
}
