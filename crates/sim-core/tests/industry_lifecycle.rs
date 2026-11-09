use sim_core::*;
fn content() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-industry-lifecycle-test.json"
    ))
    .unwrap()
}
fn level(world: &World) -> u32 {
    world.player_country().industry.as_ref().unwrap().levels[&SectorId("construction".into())]
}
fn enact(world: &mut World, content: &Content, policy: &str) -> PolicyInstance {
    world
        .issue(
            content,
            Action::Enact {
                policy: policy.into(),
                value: 1,
            },
        )
        .unwrap();
    world
        .player_country()
        .projects
        .iter()
        .find(|p| p.policy == policy)
        .unwrap()
        .clone()
}
fn operate(world: &mut World, content: &Content, policy: &str, expected: u32) -> PolicyInstance {
    for _ in 0..3 {
        match world.preview(content, &world.player, &policy.into(), 1) {
            Err(reason) if reason.contains("Negotiate governing support") => {
                let quote = world
                    .preview(content, &world.player, &"coalition".into(), 1)
                    .unwrap();
                enact(world, content, "coalition");
                world.advance(content, quote.delay).unwrap();
            }
            _ => break,
        }
    }
    let project = enact(world, content, policy);
    for _ in 0..300 {
        world.advance(content, DaySpan::new(1)).unwrap();
        if !world
            .player_country()
            .projects
            .iter()
            .any(|p| p.id == project.id)
        {
            break;
        }
    }
    assert_eq!(level(world), expected, "{policy} must actually operate");
    world.validate(content).unwrap();
    project
}
#[test]
fn paid_retirement_protects_opening_assets_settles_after_production_and_survives_replay() {
    let content = content();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let id = world.player.clone();
    let sector = SectorId("construction".into());
    assert!(
        world
            .preview(&content, &id, &"retire_industry_construction".into(), 1)
            .unwrap_err()
            .contains("opening equipment")
    );
    operate(&mut world, &content, "industry_construction", 1);
    world.advance(&content, DaySpan::new(1)).unwrap();
    let nominal = world
        .production_report(&content, Viewer::Government(id.clone()), &id)
        .unwrap()
        .unwrap()
        .owned
        .unwrap()
        .industry
        .unwrap()
        .nominal
        .installed[&sector];
    let quote = world
        .preview(&content, &id, &"retire_industry_construction".into(), 1)
        .unwrap();
    assert!(quote.cost.0 > 0 && quote.annual_budget_change.0 < 0);
    assert!(quote.explanation.contains("No cash proceeds"));
    assert!(
        world
            .construction_material_plan(
                &content,
                Viewer::Government(id.clone()),
                &id,
                &"retire_industry_construction".into()
            )
            .unwrap()
            .is_none()
    );
    let before_cash = world.player_country().available_cash();
    let project = enact(&mut world, &content, "retire_industry_construction");
    assert_eq!(
        world.player_country().available_cash(),
        Money(before_cash.0 - quote.cost.0)
    );
    assert!(
        world
            .preview(&content, &id, &"industry_construction".into(), 1)
            .unwrap_err()
            .contains("pending")
    );
    world.advance(&content, DaySpan::new(1)).unwrap();
    assert_eq!(level(&world), 1);
    let snapshot = persistence::encode(&world, &content).unwrap();
    let mut resumed = persistence::decode(&snapshot, &content).unwrap();
    for _ in 0..120 {
        world.advance(&content, DaySpan::new(1)).unwrap();
        resumed.advance(&content, DaySpan::new(1)).unwrap();
        assert_eq!(world, resumed);
        if level(&world) == 0 {
            break;
        }
    }
    assert_eq!(level(&world), 0);
    assert!(world.day >= project.due_day);
    let last = world.production.as_ref().unwrap().countries[&id]
        .last
        .as_ref()
        .unwrap();
    assert_eq!(last.industry_opening.as_ref().unwrap().levels[&sector], 1);
    assert!(last.request.capacities[&sector].installed_batches.0 <= nominal.0);
    let carry = world
        .player_country()
        .industry
        .as_ref()
        .unwrap()
        .upkeep_remainder;
    world.advance(&content, DaySpan::new(1)).unwrap();
    assert_eq!(
        world
            .player_country()
            .industry
            .as_ref()
            .unwrap()
            .upkeep_remainder,
        carry
    );
    let last = world.production.as_ref().unwrap().countries[&id]
        .last
        .as_ref()
        .unwrap();
    assert_eq!(last.industry_opening.as_ref().unwrap().levels[&sector], 0);
    assert_eq!(
        last.request.capacities[&sector].installed_batches,
        content.production.as_ref().unwrap().countries[&id].installed[&sector]
    );
    let report = world
        .production_report(&content, Viewer::Government(id.clone()), &id)
        .unwrap()
        .unwrap()
        .owned
        .unwrap()
        .industry
        .unwrap();
    assert_eq!(report.upkeep.due(DaySpan::new(365)).unwrap(), Money(0));
    assert!(report.retirement.is_some());
    assert!(
        world
            .production_report(&content, Viewer::Observer, &id)
            .unwrap()
            .unwrap()
            .owned
            .is_none()
    );
    world.validate(&content).unwrap();
    assert_eq!(persistence::snapshot_schema(&world), 34);
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}
#[test]
fn cancelling_retirement_preserves_capacity_and_after_consent_keeps_its_sunk_fee() {
    let content = content();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    operate(&mut world, &content, "industry_construction", 1);
    let project = enact(&mut world, &content, "retire_industry_construction");
    let mut early = world.clone();
    early
        .issue(
            &content,
            Action::CancelCommitment {
                request: project.id,
            },
        )
        .unwrap();
    assert_eq!(level(&early), 1);
    assert!(early.player_country().available_cash().0 > world.player_country().available_cash().0);
    let consent = project.consent_day.elapsed_since(world.day).unwrap();
    world.advance(&content, consent).unwrap();
    let cash = world.player_country().accounts.treasury;
    world
        .issue(
            &content,
            Action::CancelCommitment {
                request: project.id,
            },
        )
        .unwrap();
    assert_eq!(world.player_country().accounts.treasury, cash);
    assert_eq!(level(&world), 1);
    world.validate(&content).unwrap();
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}
#[test]
fn retired_levels_can_be_replaced_and_frozen_budget_reviews_keep_their_original_assets() {
    let mut content = content();
    // A funded test calibration exercises lifetime replacement without a fiscal crisis.
    let opening = content
        .countries
        .iter_mut()
        .find(|c| c.id.0.as_str() == "AUR")
        .unwrap();
    opening.tax = Percent::new(45);
    opening.services = Percent::new(4);
    opening.welfare = Percent::new(0);
    content.validate().unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    // Four lifetime additions are legal with never more than one operating at a time.
    for cycle in 0..4 {
        operate(&mut world, &content, "industry_construction", 1);
        if cycle == 0 {
            // Freeze a fiscal forecast while the addition is still operating.
            let boundary = world.player_country().budget.end_day;
            world
                .advance(&content, boundary.elapsed_since(world.day).unwrap())
                .unwrap();
            assert_eq!(
                world
                    .player_country()
                    .budget
                    .forecast
                    .industry_upkeep
                    .as_ref()
                    .unwrap()
                    .levels,
                1
            );
        }
        operate(&mut world, &content, "retire_industry_construction", 0);
    }
    assert!(
        world.player_country().budget_reviews.iter().any(|r| r
            .period
            .forecast
            .industry_upkeep
            .as_ref()
            .unwrap()
            .levels
            > 0)
    );
    let carry = world
        .player_country()
        .industry
        .as_ref()
        .unwrap()
        .upkeep_remainder;
    world.advance(&content, DaySpan::new(3)).unwrap();
    assert_eq!(
        world
            .player_country()
            .industry
            .as_ref()
            .unwrap()
            .upkeep_remainder,
        carry
    );
    world.validate(&content).unwrap();
    let mut forged = world.clone();
    forged
        .countries
        .get_mut(&world.player)
        .unwrap()
        .budget_reviews[0]
        .period
        .forecast
        .industry_upkeep
        .as_mut()
        .unwrap()
        .levels = 2;
    assert!(forged.validate(&content).is_err());
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}
#[test]
fn forged_retirement_history_and_wrong_save_format_are_rejected_transactionally() {
    let content = content();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    operate(&mut world, &content, "industry_construction", 1);
    operate(&mut world, &content, "retire_industry_construction", 0);
    let mut raw: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&world, &content).unwrap()).unwrap();
    raw["schema"] = serde_json::json!(33);
    assert!(
        persistence::decode(&serde_json::to_vec(&raw).unwrap(), &content)
            .unwrap_err()
            .contains("34")
    );
    let id = world.player.clone();
    let record = world
        .countries
        .get_mut(&id)
        .unwrap()
        .commitments
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r.project.policy == "retire_industry_construction")
        .unwrap();
    record.direct_effects[0].before = 0;
    record.direct_effects[0].after = -1;
    assert!(world.validate(&content).is_err());
    let before = world.clone();
    assert!(world.advance(&content, DaySpan::new(1)).is_err());
    assert_eq!(world, before);
}
#[test]
fn retirement_definition_requires_paid_attributed_exact_coverage_and_preserves_old_opening() {
    let new = Content::industry_lifecycle_bundled().unwrap();
    let old = Content::industry_bundled().unwrap();
    let expected =
        serde_json::to_value(World::new(&old, CountryId("GBR".into()), 17).unwrap()).unwrap();
    let mut actual =
        serde_json::to_value(World::new(&new, CountryId("GBR".into()), 17).unwrap()).unwrap();
    actual["rules"] = expected["rules"].clone();
    actual["content_hash"] = expected["content_hash"].clone();
    assert_eq!(actual, expected);
    for mutation in [0, 1, 2, 3] {
        let mut bad = new.clone();
        match mutation {
            0 => {
                bad.industry
                    .as_mut()
                    .unwrap()
                    .retirement
                    .as_mut()
                    .unwrap()
                    .source = "missing".into()
            }
            1 => {
                bad.industry
                    .as_mut()
                    .unwrap()
                    .retirement
                    .as_mut()
                    .unwrap()
                    .policies
                    .remove(&"retire_industry_construction".into());
            }
            2 => {
                bad.policies
                    .iter_mut()
                    .find(|p| p.id == "retire_industry_construction")
                    .unwrap()
                    .cost_bps = BasisPoints::new(0);
            }
            _ => {
                bad.production
                    .as_mut()
                    .unwrap()
                    .construction_policy_weights
                    .insert("retire_industry_construction".into(), 1);
            }
        }
        assert!(bad.validate().is_err());
    }
    let mut old = old;
    old.industry.as_mut().unwrap().retirement = new.industry.unwrap().retirement;
    assert!(old.validate().is_err());
}

#[test]
fn ai_retains_productive_paid_service_additions() {
    let mut content = content();
    let id = CountryId("BOR".into());
    let sector = SectorId("services".into());
    content
        .production
        .as_mut()
        .unwrap()
        .countries
        .get_mut(&id)
        .unwrap()
        .installed
        .insert(sector.clone(), ProductionBatches(0));
    content.validate().unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.advance(&content, DaySpan::new(450)).unwrap();
    assert!(world.countries[&id].industry.as_ref().unwrap().levels[&sector] > 0);
    assert!(!world.journal.iter().any(|e| e.ai && e.command.issuer == id && matches!(&e.command.action, Action::Enact {policy, ..} if policy.as_str().starts_with("retire_industry_"))));
    world.validate(&content).unwrap();
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}

#[test]
fn actual_retirement_returns_no_cash_or_packages_and_does_not_erase_unpaid_claims() {
    let content = content();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    operate(&mut world, &content, "industry_construction", 1);
    let project = enact(&mut world, &content, "retire_industry_construction");
    world
        .advance(
            &content,
            project.consent_day.elapsed_since(world.day).unwrap(),
        )
        .unwrap();
    let mut cancelled = world.clone();
    cancelled
        .issue(
            &content,
            Action::CancelCommitment {
                request: project.id,
            },
        )
        .unwrap();
    world
        .advance(&content, project.due_day.elapsed_since(world.day).unwrap())
        .unwrap();
    cancelled
        .advance(
            &content,
            project.due_day.elapsed_since(cancelled.day).unwrap(),
        )
        .unwrap();
    assert_eq!(level(&world), 0);
    assert_eq!(level(&cancelled), 1);
    // Both routes already paid the same fee. All settlement through the actual
    // retirement day still uses identical opening assets and creditor claims.
    assert_eq!(
        world.player_country().accounts,
        cancelled.player_country().accounts
    );
    assert_eq!(
        world.player_country().arrears,
        cancelled.player_country().arrears
    );
    assert_eq!(world.production, cancelled.production);
    assert_eq!(
        world
            .player_country()
            .industry
            .as_ref()
            .unwrap()
            .upkeep_remainder,
        cancelled
            .player_country()
            .industry
            .as_ref()
            .unwrap()
            .upkeep_remainder
    );
    world.validate(&content).unwrap();
    cancelled.validate(&content).unwrap();
}

#[test]
fn ai_retires_an_unmaintained_addition_through_ordinary_paid_delivery() {
    let mut content = content();
    let id = CountryId("BOR".into());
    let opening = content
        .production
        .as_mut()
        .unwrap()
        .countries
        .get_mut(&id)
        .unwrap();
    opening
        .installed
        .insert(SectorId("services".into()), ProductionBatches(0));
    // Deliberate physical shortage: even commissioned service additions cannot
    // meet this authored maintenance requirement and lose next-day readiness.
    opening.maintenance_units_per_step = ProductionQuantity(1000);
    content.validate().unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    for _ in 0..450 {
        world.advance(&content, DaySpan::new(1)).unwrap();
        if world.countries[&id]
            .commitments
            .as_ref()
            .unwrap()
            .iter()
            .any(|r| {
                r.project.policy == "retire_industry_services"
                    && r.milestones
                        .iter()
                        .any(|m| m.stage == CommitmentStage::Operating)
            })
        {
            break;
        }
    }
    let retirement = world.countries[&id]
        .commitments
        .as_ref()
        .unwrap()
        .iter()
        .find(|r| {
            r.project.policy == "retire_industry_services"
                && r.milestones
                    .iter()
                    .any(|m| m.stage == CommitmentStage::Operating)
        })
        .expect(
            "Observed unused, unmaintained equipment must take the ordinary paid retirement route",
        );
    assert!(retirement.paid.0 > 0);
    assert!(
        retirement
            .direct_effects
            .iter()
            .all(|e| e.before > 0 && e.after == e.before - 1)
    );
    assert!(world.journal.iter().any(|e| e.ai && e.command.issuer == id && matches!(&e.command.action, Action::Enact {policy, ..} if policy == "retire_industry_services")));
    let last = world.countries[&id]
        .markets
        .as_ref()
        .unwrap()
        .food
        .last
        .as_ref()
        .unwrap();
    assert!(last.physical_need.units() > content.country(&id).unwrap().population.0 as i64 / 1000);
    assert!(
        last.reserve_release.unwrap().units() > 0,
        "The growing-population release must validate against actual daily need"
    );
    world.validate(&content).unwrap();
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}
