use eframe::egui;
use sim_core::*;

pub fn material_hint(ui: &mut egui::Ui, plan: &ConstructionMaterialPlan) {
    ui.label(format!(
        "{}: {} required; {} stored ({})",
        plan.name, plan.required.0, plan.available.0, plan.unit
    ));
    ui.small("Delivery dates are earliest dates. Packages are allocated when funded programmes operate; current stocks are not reserved for this draft.");
}
/// Show time and physical availability separately for an owned pending programme.
/// Current packages are observations, never reservations or delivery forecasts.
pub fn pending_progress(
    ui: &mut egui::Ui,
    content: &Content,
    world: &World,
    project: &PolicyInstance,
) {
    let material = world
        .construction_material_plan(
            content,
            Viewer::Government(world.player.clone()),
            &world.player,
            &project.policy,
        )
        .ok()
        .flatten();
    let date_label = if material.is_some() {
        "earliest operation"
    } else {
        "operating due"
    };
    ui.label(format!(
        "{} · consent {} · {date_label} {} · reserved {}m",
        project.stage.label(),
        date(project.consent_day),
        date(project.due_day),
        project.reserved.0
    ));
    let elapsed = world.day.units().saturating_sub(project.start_day.units()) as f32;
    let duration = (project.due_day.units() - project.start_day.units()).max(1) as f32;
    let progress = (elapsed / duration).clamp(0., 1.);
    if let Some(material) = material {
        let waiting = project.stage == PolicyStage::Implementing
            && world.day >= project.due_day
            && material.available.0 < material.required.0;
        if waiting {
            ui.strong("Waiting for construction packages");
            ui.label(
                "Minimum construction time has elapsed. Operation still needs the actual packages.",
            );
            let availability =
                (material.available.0 as f32 / material.required.0.max(1) as f32).clamp(0., 1.);
            ui.add(egui::ProgressBar::new(availability).text(format!(
                "{} / {} available now",
                material.available.0, material.required.0
            )));
        } else {
            ui.add(egui::ProgressBar::new(progress).text(format!(
                "Minimum construction time: {:.0}%",
                progress * 100.
            )));
        }
        material_hint(ui, &material);
    } else {
        ui.add(egui::ProgressBar::new(progress).show_percentage());
    }
}
pub fn show(ui: &mut egui::Ui, content: &Content, world: &World, country: &CountryId) {
    if content.production.is_none() {
        return;
    }
    egui::CollapsingHeader::new("Industry, inputs & construction")
        .id_salt(("production-owned", &country.0, world.content_hash))
        .show(ui, |ui| {
            if let Ok(Some(report)) =
                world.production_report(content, Viewer::Government(world.player.clone()), country)
            {
                inner(ui, &report);
            }
        });
}
pub fn show_public(ui: &mut egui::Ui, content: &Content, world: &World, country: &CountryId) {
    if content.production.is_none() {
        return;
    }
    egui::CollapsingHeader::new("Infrastructure production definitions")
        .id_salt(("production-public", &country.0, world.content_hash))
        .show(ui, |ui| {
            if let Ok(Some(report)) = world.production_report(content, Viewer::Observer, country) {
                inner(ui, &report);
            }
        });
}
fn inner(ui: &mut egui::Ui, report: &ProductionReport) {
    let Some(owned) = &report.owned else {
        ui.label("No current industrial stocks, staffing or project requirements are published for this country. Missing observations are not zero stocks.");
        for sector in &report.sectors {
            ui.label(&sector.name);
        }
        provenance(ui, report);
        return;
    };
    ui.small("Authored infrastructure inputs; national civilian food and energy remain separate. Stocks below use each row's own model unit.");
    if let Some(industry) = &owned.industry {
        ui.strong("Public industry capacity");
        if let Some(retirement) = &industry.retirement {
            ui.label(&retirement.description);
        }
        ui.label(format!(
            "Added equipment readiness next day {}% · upkeep next day {}m / next 365 days {}m",
            industry.maintenance_readiness,
            industry
                .upkeep
                .due(DaySpan::new(1))
                .expect("validated upkeep")
                .0,
            industry
                .upkeep
                .due(DaySpan::new(365))
                .expect("validated upkeep")
                .0
        ));
        ui.small("Nominal installed batches include paid operating investments. Effective batches use the last maintenance settlement. New equipment produces from the following day; inspection does not reserve materials or commission work.");
        for sector in &report.sectors {
            ui.label(format!("{}: investment levels {} / {} · nominal {} batches · effective next day {} batches",sector.name,industry.levels[&sector.id],industry.maximum_levels,industry.nominal.installed[&sector.id].0,industry.effective.installed[&sector.id].0));
        }
        ui.label("Commission a capacity investment under Policies & delivery. Ordinary authority, funding, consent, minimum time and actual assembly packages apply.");
    }
    if let Some(last) = &owned.last {
        let labor = &last.physical.production;
        ui.label(format!(
            "Settled {} · network worker-days used {} / {} funded",
            date(last.physical.day),
            labor.labor_available.0 - labor.labor_unused.0,
            labor.labor_available.0
        ));
        ui.label(format!("Staffing follows actual private income paid: {}m. Network pool: {:.1}% of employed residents; other workers serve the civilian economy.", last.funding.private_income_paid.0, owned.worker_share.units() as f64 / 100.));
        ui.label(format!("Public service inputs: {} / {} supplied · maintenance: {} / {} · expanded-capacity readiness next day: {}%", last.service_delivered.0,last.service_required.0,last.maintenance_delivered.0,last.maintenance_required.0,last.maintenance_percent));
    } else {
        ui.label("Opening stocks · awaiting the first production settlement.");
    }
    ui.strong(format!("Current stocks · {}", date(report.day)));
    egui::ScrollArea::horizontal()
        .id_salt(("production-stock-scroll", &report.country.0))
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
        .show(ui, |ui| {
            egui::Grid::new(("production-stocks", &report.country.0))
                .striped(true)
                .show(ui, |ui| {
                    for title in [
                        "Good",
                        "Stored",
                        "Capacity",
                        "Unit",
                        "Produced today",
                        "Industrial use",
                        "Final / capital use",
                        "Discarded",
                        "Spoiled",
                    ] {
                        ui.strong(title);
                    }
                    ui.end_row();
                    for (id, good) in &report.goods {
                        ui.label(&good.name);
                        ui.label(owned.inventory.positions[id].quantity.0.to_string());
                        ui.label(owned.storage[id].capacity.0.to_string());
                        ui.label(&good.unit);
                        if let Some(last) = &owned.last {
                            let production = &last.physical.production.goods[id];
                            let storage = &last.physical.storage[id];
                            // Post-storage capital use is the difference between the
                            // day's closing receipt and the current position.
                            let capital =
                                storage.closing.0 - owned.inventory.positions[id].quantity.0;
                            for quantity in [
                                production.produced.0,
                                production.consumed.0,
                                storage.final_use.0 + capital,
                                storage.discarded.0,
                                storage.spoiled.0,
                            ] {
                                ui.label(quantity.to_string());
                            }
                        } else {
                            for _ in 0..5 {
                                ui.label("—");
                            }
                        }
                        ui.end_row();
                    }
                });
        });
    if let Some(last) = &owned.last {
        ui.strong("Production requests & bottlenecks");
        egui::Grid::new(("production-limits", &report.country.0))
            .striped(true)
            .show(ui, |ui| {
                for title in [
                    "Sector",
                    "Requested batches",
                    "Produced batches",
                    "Limiting input",
                ] {
                    ui.strong(title);
                }
                ui.end_row();
                for sector in &last.physical.production.sectors {
                    let name = report
                        .sectors
                        .iter()
                        .find(|s| s.id == sector.sector)
                        .map_or(sector.sector.0.as_str(), |s| s.name.as_str());
                    ui.label(name);
                    ui.label(sector.requested.0.to_string());
                    ui.label(sector.produced.0.to_string());
                    let limits = sector
                        .limits
                        .iter()
                        .filter(|limit| limit.binding)
                        .map(|limit| match &limit.constraint {
                            ProductionConstraint::InstalledCapacity => "Installed capacity".into(),
                            ProductionConstraint::Labor => "Paid worker-days".into(),
                            ProductionConstraint::Capital(id) => {
                                format!("Capital services ({})", id.0.replace('_', " "))
                            }
                            ProductionConstraint::Input(id) => report.goods[id].name.clone(),
                        })
                        .collect::<Vec<_>>();
                    ui.label(if sector.requested.0 == 0 {
                        "No replenishment requested".into()
                    } else if limits.is_empty() {
                        "Request supplied".into()
                    } else {
                        limits.join(", ")
                    });
                    ui.end_row();
                }
            });
    }
    if !owned.pending.is_empty() {
        ui.strong("Pending construction packages");
        for project in &owned.pending {
            let status = if project.stage == PolicyStage::Implementing
                && project.earliest_day <= report.day
                && project.materials.available.0 < project.materials.required.0
            {
                "Waiting for packages"
            } else {
                "Pending"
            };
            ui.label(format!(
                "{} · #{} · {status} · earliest {} · {} required / {} currently stored",
                project.name,
                project.commitment,
                date(project.earliest_day),
                project.materials.required.0,
                project.materials.available.0
            ));
        }
        ui.small("Ready projects use packages in commitment-ID order. Shortages delay operation after funding; current stocks are not separate reservations for every project.");
    }
    provenance(ui, report);
}
fn provenance(ui: &mut egui::Ui, report: &ProductionReport) {
    egui::CollapsingHeader::new("Production assumptions & source").show(ui, |ui| {
        ui.label(&report.description);
        ui.small(format!("{} / {}", report.source, report.source_version));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn waiting_fixture() -> (Content, World) {
        let mut content = Content::from_json(include_str!(
            "../../../data/synthetic-sector-production-test.json"
        ))
        .unwrap();
        let id = CountryId("AUR".into());
        let mut probe = World::new(&content, id.clone(), 17).unwrap();
        probe.advance(&content, DaySpan::new(30)).unwrap();
        let delay = probe
            .preview(&content, &id, &"energy".into(), 1)
            .unwrap()
            .delay;
        let required = u64::from(probe.day.units() + delay.units() + 40) * 10;
        let production = content.production.as_mut().unwrap();
        for weight in production.construction_policy_weights.values_mut() {
            *weight = 1;
        }
        let opening = production.countries.get_mut(&id).unwrap();
        opening.construction_unit = ProductionQuantity(required);
        opening
            .installed
            .insert(SectorId("construction".into()), ProductionBatches(1));
        opening
            .storage
            .get_mut(&production.assembly_good)
            .unwrap()
            .capacity = ProductionQuantity(required);
        content.validate().unwrap();
        let mut world = World::new(&content, id, 17).unwrap();
        world.advance(&content, DaySpan::new(30)).unwrap();
        world
            .issue(
                &content,
                Action::Enact {
                    policy: "energy".into(),
                    value: 1,
                },
            )
            .unwrap();
        (content, world)
    }
    fn pending_text(content: &Content, world: &World, scale: f32) -> String {
        let ctx = egui::Context::default();
        ctx.set_zoom_factor(scale);
        let mut panel = crate::commitments::Panel::default();
        let mut text = String::new();
        for frame in 0..5 {
            let output = ctx.run(
                egui::RawInput {
                    focused: true,
                    time: Some(frame as f64 / 60.),
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900. / scale, 1000. / scale),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        assert!(panel.show(ui, content, world).is_none());
                    });
                },
            );
            text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.text()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
        }
        text
    }
    #[test]
    fn paid_material_shortage_replaces_complete_time_percentage_with_actual_availability() {
        let (content, mut world) = waiting_fixture();
        let project = world.player_country().projects[0].clone();
        world
            .advance(&content, project.due_day.elapsed_since(world.day).unwrap())
            .unwrap();
        world.validate(&content).unwrap();
        let record = world.player_country().commitment(&project.id).unwrap();
        assert_eq!(record.paid, project.cost);
        assert_eq!(record.stage(), CommitmentStage::Implementing);
        let plan = world
            .construction_material_plan(
                &content,
                Viewer::Government(world.player.clone()),
                &world.player,
                &project.policy,
            )
            .unwrap()
            .unwrap();
        assert!(plan.available.0 < plan.required.0);
        let before = sim_core::persistence::encode(&world, &content).unwrap();
        for scale in [1., 1.4] {
            let text = pending_text(&content, &world, scale);
            assert!(text.contains("Waiting for construction packages"), "{text}");
            assert!(text.contains("Minimum construction time has elapsed"));
            assert!(text.contains(&format!(
                "{} / {} available now",
                plan.available.0, plan.required.0
            )));
            assert!(text.contains("earliest operation"));
            assert!(!text.contains("operating due"));
            assert!(
                !text.contains("100%"),
                "elapsed time is not completed material delivery: {text}"
            );
            assert_eq!(
                sim_core::persistence::encode(&world, &content).unwrap(),
                before
            );
        }
    }
    #[test]
    fn material_programmes_show_conditional_time_before_consent_and_earlier_packs_keep_their_dates()
    {
        let (content, world) = waiting_fixture();
        assert_eq!(
            world.player_country().projects[0].stage,
            PolicyStage::AwaitingConsent
        );
        let legacy = Content::bundled().unwrap();
        let mut earlier = World::new(&legacy, CountryId("GBR".into()), 17).unwrap();
        earlier.advance(&legacy, DaySpan::new(31)).unwrap();
        earlier
            .issue(
                &legacy,
                Action::Enact {
                    policy: "energy".into(),
                    value: 1,
                },
            )
            .unwrap();
        for scale in [1., 1.4] {
            let before = sim_core::persistence::encode(&world, &content).unwrap();
            let text = pending_text(&content, &world, scale);
            assert!(text.contains("earliest operation"));
            assert!(text.contains("Minimum construction time: 0%"));
            assert!(text.contains("current stocks are not reserved"));
            assert!(!text.contains("Waiting for construction packages"));
            assert_eq!(
                sim_core::persistence::encode(&world, &content).unwrap(),
                before
            );
            let before = sim_core::persistence::encode(&earlier, &legacy).unwrap();
            let text = pending_text(&legacy, &earlier, scale);
            assert!(text.contains("operating due"));
            assert!(!text.contains("earliest operation"));
            assert!(!text.contains("Minimum construction time"));
            assert!(!text.contains("assembly packages"));
            assert_eq!(
                sim_core::persistence::encode(&earlier, &legacy).unwrap(),
                before
            );
        }
    }
    fn fixture() -> (Content, World) {
        let content = Content::from_json(include_str!(
            "../../../data/synthetic-sector-production-test.json"
        ))
        .unwrap();
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        world.advance(&content, DaySpan::new(2)).unwrap();
        (content, world)
    }
    fn frame(
        ctx: &egui::Context,
        report: &ProductionReport,
        scale: f32,
        time: f64,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                focused: true,
                time: Some(time),
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900. / scale, 1200. / scale),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| inner(ui, report));
            },
        )
    }
    fn visible(output: &egui::FullOutput, label: &str) -> Option<egui::Pos2> {
        output.shapes.iter().find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == label => {
                let pos = text.pos + text.galley.size() / 2.;
                shape.clip_rect.contains(pos).then_some(pos)
            }
            _ => None,
        })
    }
    #[test]
    fn stock_overflow_scroll_reaches_last_flow_columns_without_mutating_the_world_at_two_scales() {
        let (content, world) = fixture();
        let before = world.clone();
        let report = world
            .production_report(
                &content,
                Viewer::Government(world.player.clone()),
                &world.player,
            )
            .unwrap()
            .unwrap();
        for scale in [1., 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut output = frame(&ctx, &report, scale, 0., vec![]);
            for step in 1..5 {
                output = frame(&ctx, &report, scale, step as f64 / 60., vec![]);
            }
            let point = visible(&output, "Stored").expect("current stock column visible initially")
                + egui::vec2(0., 25.);
            assert!(visible(&output, "Spoiled").is_none());
            let _ = frame(
                &ctx,
                &report,
                scale,
                0.1,
                vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(-10000., 0.),
                        modifiers: Default::default(),
                    },
                ],
            );
            for step in 7..35 {
                output = frame(&ctx, &report, scale, step as f64 / 60., vec![]);
            }
            assert!(
                visible(&output, "Spoiled").is_some(),
                "last per-good flow column reachable at scale {scale}"
            );
            assert_eq!(world, before);
        }
    }
    #[test]
    fn switching_from_owner_to_foreign_definitions_removes_all_live_stock_and_staffing_views() {
        let (content, world) = fixture();
        let owner = world
            .production_report(
                &content,
                Viewer::Government(world.player.clone()),
                &world.player,
            )
            .unwrap()
            .unwrap();
        let public = world
            .production_report(
                &content,
                Viewer::Government(world.player.clone()),
                &CountryId("BOR".into()),
            )
            .unwrap()
            .unwrap();
        for scale in [1., 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let _ = frame(&ctx, &owner, scale, 0., vec![]);
            let output = frame(&ctx, &public, scale, 0.1, vec![]);
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) => Some(text.galley.text()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert!(text.contains("No current industrial stocks"));
            assert!(!text.contains("Current stocks ·"));
            assert!(!text.contains("network worker-days used"));
            assert!(!text.contains("Pending construction packages"));
        }
    }
}

#[cfg(test)]
mod industry_tests {
    use super::*;
    #[test]
    fn industry_capacity_and_upkeep_are_read_only_and_hidden_from_foreign_inspection_at_two_scales()
    {
        for content in [
            Content::industry_bundled().unwrap(),
            Content::industry_lifecycle_bundled().unwrap(),
        ] {
            let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
            let before = sim_core::persistence::encode(&world, &content).unwrap();
            for scale in [1., 1.4] {
                let ctx = egui::Context::default();
                ctx.set_zoom_factor(scale);
                for (viewer, owned) in [
                    (Viewer::Government(world.player.clone()), true),
                    (Viewer::Observer, false),
                ] {
                    let report = world
                        .production_report(&content, viewer, &world.player)
                        .unwrap()
                        .unwrap();
                    let output = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(1440. / scale, 940. / scale),
                            )),
                            ..Default::default()
                        },
                        |ctx| {
                            egui::CentralPanel::default().show(ctx, |ui| inner(ui, &report));
                        },
                    );
                    let texts = output
                        .shapes
                        .iter()
                        .filter_map(|s| match &s.shape {
                            egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    assert_eq!(texts.contains("Public industry capacity"), owned);
                    assert_eq!(
                        texts.contains("Opening equipment cannot be retired"),
                        owned && content.rules == 33
                    );
                    assert_eq!(texts.contains("investment levels"), owned);
                    if owned {
                        assert!(texts.contains("upkeep next day"));
                        assert!(texts.contains("effective next day"));
                    } else {
                        assert!(!texts.contains("nominal"));
                        assert!(!texts.contains("upkeep"));
                    }
                    assert_eq!(
                        sim_core::persistence::encode(&world, &content).unwrap(),
                        before
                    );
                }
            }
        }
    }
}
