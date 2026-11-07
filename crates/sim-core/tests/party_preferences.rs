use sim_core::*;

fn fixture() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-party-preferences-test.json"
    ))
    .unwrap()
}
fn player() -> CountryId {
    CountryId("AUR".into())
}
fn current() -> FiscalTargets {
    FiscalTargets {
        income_tax: 24,
        services: 9,
        welfare: 4,
    }
}
fn hostile() -> FiscalTargets {
    FiscalTargets {
        income_tax: 45,
        services: 18,
        welfare: 12,
    }
}
fn neutralize(content: &mut Content, country: &str) {
    let r = content
        .party_preferences
        .as_mut()
        .unwrap()
        .influences
        .iter_mut()
        .find(|r| r.country.0 == country)
        .unwrap();
    r.legislative_max_penalty = 0;
    r.party_leadership_max_penalty = 0;
}
fn replay(world: &World, content: &Content) {
    world.validate(content).unwrap();
    assert_eq!(*world, persistence::replay(world, content).unwrap());
}
#[test]
fn positions_are_explicit_attributed_and_not_ideology_label_bonuses() {
    let content = fixture();
    assert_eq!(content.rules, 20);
    assert_eq!(
        content.party_preferences.as_ref().unwrap().profiles.len(),
        6
    );
    let boreal = content.country(&CountryId("BOR".into())).unwrap();
    assert_eq!(boreal.government.route, DecisionRoute::PartyCouncil);
    let world = World::new(&content, player(), 17).unwrap();
    let aligned = world
        .party_alignment_for(&content, &player(), &current())
        .unwrap()
        .unwrap();
    assert_eq!(aligned.metric, Some(PartyBackingMetric::Legislative));
    assert_eq!(aligned.max_penalty, 20);
    assert_eq!(aligned.penalty, 0);
    assert_eq!(aligned.known_positions_bps.units(), 10000);
    let opposed = world
        .party_alignment_for(&content, &player(), &hostile())
        .unwrap()
        .unwrap();
    assert_eq!(opposed.penalty, 9);
    assert_eq!(opposed.parties[0].weight, 60);
    assert_eq!(opposed.parties[1].weight, 0);
    let json = serde_json::to_value(opposed).unwrap();
    for key in [
        "treasury",
        "households",
        "elite_support",
        "projects",
        "seed",
        "journal",
    ] {
        assert!(json.get(key).is_none());
    }
    let mut renamed = content.clone();
    for country in &mut renamed.countries {
        for party in &mut country.parties {
            party.ideology = "Different descriptive label".into();
        }
    }
    let mut first = world;
    let mut second = World::new(&renamed, player(), 17).unwrap();
    first
        .advance(&content, sim_core::DaySpan::new(100))
        .unwrap();
    second
        .advance(&renamed, sim_core::DaySpan::new(100))
        .unwrap();
    first.content_hash = 0;
    second.content_hash = 0;
    assert_eq!(first, second);
}
#[test]
fn invalid_positions_references_limits_and_implicit_unknowns_are_rejected() {
    let original: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/synthetic-party-preferences-test.json"
    ))
    .unwrap();
    for kind in [
        "missing_profile",
        "duplicate_profile",
        "party",
        "country",
        "range",
        "importance",
        "duplicate_axis",
        "missing_axis",
        "revision",
        "source",
        "unknown_quality",
        "missing_preferred",
        "missing_influence",
        "limit",
        "old_rules",
    ] {
        let mut value = original.clone();
        let prefs = &mut value["party_preferences"];
        match kind {
            "missing_profile" => {
                prefs["profiles"].as_array_mut().unwrap().pop();
            }
            "duplicate_profile" => {
                let duplicate = prefs["profiles"][0].clone();
                prefs["profiles"].as_array_mut().unwrap().push(duplicate);
            }
            "party" => prefs["profiles"][0]["party"] = "absent".into(),
            "country" => prefs["profiles"][0]["country"] = "absent".into(),
            "range" => prefs["profiles"][0]["positions"][0]["preferred"]["max"] = 99.into(),
            "importance" => prefs["profiles"][0]["positions"][0]["importance"] = 0.into(),
            "duplicate_axis" => {
                prefs["profiles"][0]["positions"][1]["policy"] = "income_tax".into()
            }
            "missing_axis" => {
                prefs["profiles"][0]["positions"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            "revision" => prefs["profiles"][0]["source_version"] = "".into(),
            "source" => prefs["source"] = "absent".into(),
            "unknown_quality" => prefs["profiles"][0]["quality"] = "unknown".into(),
            "missing_preferred" => {
                prefs["profiles"][0]["positions"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("preferred");
            }
            "missing_influence" => {
                prefs["influences"].as_array_mut().unwrap().pop();
            }
            "limit" => prefs["influences"][0]["legislative_max_penalty"] = 21.into(),
            "old_rules" => {
                value["schema"] = 19.into();
                value["rules"] = 19.into();
            }
            _ => unreachable!(),
        }
        assert!(
            Content::from_json(&value.to_string()).is_err(),
            "accepted {kind}"
        );
    }
    let mut missing = original;
    missing.as_object_mut().unwrap().remove("party_preferences");
    assert!(Content::from_json(&missing.to_string()).is_err());
}
#[test]
fn coalition_seats_weight_known_positions_and_unknowns_create_no_opposition() {
    let mut content = fixture();
    content
        .countries
        .iter_mut()
        .find(|c| c.id == player())
        .unwrap()
        .coalition
        .push("AUR-2".into());
    for profile in content
        .party_preferences
        .as_mut()
        .unwrap()
        .profiles
        .iter_mut()
        .filter(|p| p.country == player())
    {
        for p in &mut profile.positions {
            let def = content.policies.iter().find(|d| d.id == p.policy).unwrap();
            let ideal = if profile.party.0 == "AUR-1" {
                def.min
            } else {
                def.max
            };
            p.preferred = Some(PreferredPolicyRange {
                min: ideal,
                max: ideal,
            });
        }
    }
    let low = FiscalTargets {
        income_tax: 10,
        services: 4,
        welfare: 0,
    };
    let world = World::new(&content, player(), 17).unwrap();
    let r = world
        .party_alignment_for(&content, &player(), &low)
        .unwrap()
        .unwrap();
    assert_eq!(r.penalty, 8); // 40 opposition-coalition seats × 20-point maximum.
    let unknown = content
        .party_preferences
        .as_mut()
        .unwrap()
        .profiles
        .iter_mut()
        .find(|p| p.party.0 == "AUR-2")
        .unwrap();
    unknown.quality = RecordQuality::Unknown;
    for p in &mut unknown.positions {
        p.preferred = None;
    }
    let world = World::new(&content, player(), 17).unwrap();
    let r = world
        .party_alignment_for(&content, &player(), &low)
        .unwrap()
        .unwrap();
    assert_eq!(r.penalty, 0);
    assert_eq!(r.known_positions_bps.units(), 6000);
    assert!(r.parties[1].distance_bps.is_none());
}
#[test]
fn policy_feedback_and_legislative_pressure_follow_operation_not_declaration() {
    let content = fixture();
    let mut neutral = content.clone();
    neutralize(&mut neutral, "AUR");
    let mut first = World::new(&content, player(), 17).unwrap();
    let mut second = World::new(&neutral, player(), 17).unwrap();
    let before = first.hash();
    let preview = first
        .preview(&content, &player(), &"income_tax".into(), 45)
        .unwrap();
    assert!(preview.explanation.contains("Party alignment"));
    assert_eq!(first.hash(), before);
    for (world, pack) in [(&mut first, &content), (&mut second, &neutral)] {
        world
            .issue(
                pack,
                Action::Enact {
                    policy: "income_tax".into(),
                    value: 45,
                },
            )
            .unwrap();
    }
    let due = first.player_country().projects[0].due_day;
    first
        .advance(&content, sim_core::DaySpan::new(due - 1))
        .unwrap();
    second
        .advance(&neutral, sim_core::DaySpan::new(due - 1))
        .unwrap();
    assert_eq!(first.player_country(), second.player_country());
    assert_eq!(first.player_country().tax, 24);
    first.advance(&content, sim_core::DaySpan::new(91)).unwrap();
    second
        .advance(&neutral, sim_core::DaySpan::new(91))
        .unwrap();
    assert_eq!(first.player_country().tax, 45);
    assert!(
        first.player_country().legislative_support < second.player_country().legislative_support
    );
    assert_eq!(
        first.player_country().accounts,
        second.player_country().accounts
    );
    assert_eq!(
        first.player_country().households,
        second.player_country().households
    );
    assert_eq!(
        first.player_country().approval,
        second.player_country().approval
    );
    replay(&first, &content);
    replay(&second, &neutral);
}
#[test]
fn party_leadership_pressure_is_separate_from_council_backing() {
    let content = fixture();
    let mut neutral = content.clone();
    neutralize(&mut neutral, "BOR");
    let id = CountryId("BOR".into());
    let mut first = World::new(&content, id.clone(), 17).unwrap();
    let mut second = World::new(&neutral, id.clone(), 17).unwrap();
    for (world, pack) in [(&mut first, &content), (&mut second, &neutral)] {
        world
            .issue(
                pack,
                Action::Enact {
                    policy: "income_tax".into(),
                    value: 45,
                },
            )
            .unwrap();
    }
    let due = first.player_country().projects[0].due_day;
    first
        .advance(&content, sim_core::DaySpan::new(due + 720))
        .unwrap();
    second
        .advance(&neutral, sim_core::DaySpan::new(due + 720))
        .unwrap();
    assert!(first.player_country().elite_support < second.player_country().elite_support);
    assert_eq!(
        first.player_country().council_support,
        second.player_country().council_support
    );
    assert_eq!(
        first.player_country().households,
        second.player_country().households
    );
    assert_eq!(
        first
            .party_alignment_for(&content, &id, &hostile())
            .unwrap()
            .unwrap()
            .metric,
        Some(PartyBackingMetric::PartyLeadership)
    );
    replay(&first, &content);
}
#[test]
fn royal_decisions_are_independent_of_party_positions_and_old_rules_remain_exact() {
    let mut content = fixture();
    for r in &mut content.party_preferences.as_mut().unwrap().influences {
        r.legislative_max_penalty = 0;
        r.party_leadership_max_penalty = 0;
    }
    let mut old = content.clone();
    old.party_preferences = None;
    old.rules = 19;
    old.schema = 19;
    old.validate().unwrap();
    for id in ["AUR", "BOR", "COR"] {
        let mut first = World::new(&content, CountryId(id.into()), 17).unwrap();
        let mut second = World::new(&old, CountryId(id.into()), 17).unwrap();
        first.advance(&content, sim_core::DaySpan::new(30)).unwrap();
        second.advance(&old, sim_core::DaySpan::new(30)).unwrap();
        first
            .issue(
                &content,
                Action::Enact {
                    policy: "energy".into(),
                    value: 1,
                },
            )
            .unwrap();
        second
            .issue(
                &old,
                Action::Enact {
                    policy: "energy".into(),
                    value: 1,
                },
            )
            .unwrap();
        first
            .advance(&content, sim_core::DaySpan::new(370))
            .unwrap();
        second.advance(&old, sim_core::DaySpan::new(370)).unwrap();
        first.rules = 0;
        second.rules = 0;
        first.content_hash = 0;
        second.content_hash = 0;
        assert_eq!(first, second, "disabled influence changed {id}");
    }
    let content = fixture();
    let world = World::new(&content, CountryId("COR".into()), 17).unwrap();
    let r = world
        .party_alignment_for(&content, &world.player, &hostile())
        .unwrap()
        .unwrap();
    assert_eq!(r.metric, None);
    assert_eq!(r.penalty, 0);
}
#[test]
fn alignment_queries_reject_wrong_content_and_invalid_hypothetical_targets() {
    let content = fixture();
    let world = World::new(&content, player(), 17).unwrap();
    let before = world.hash();
    let mut other = content.clone();
    other.party_preferences.as_mut().unwrap().influences[0].legislative_max_penalty = 10;
    assert!(
        world
            .party_alignment_for(&other, &player(), &current())
            .is_err()
    );
    assert!(
        world
            .party_alignment_for(&content, &CountryId("absent".into()), &current())
            .is_err()
    );
    assert!(
        world
            .party_alignment_for(
                &content,
                &player(),
                &FiscalTargets {
                    income_tax: 99,
                    ..current()
                }
            )
            .is_err()
    );
    assert_eq!(world.hash(), before);
}
#[test]
fn snapshot_21_preserves_pending_policy_and_exact_continuation() {
    let content = fixture();
    let mut world = World::new(&content, player(), 17).unwrap();
    world
        .issue(
            &content,
            Action::Enact {
                policy: "income_tax".into(),
                value: 45,
            },
        )
        .unwrap();
    world.advance(&content, sim_core::DaySpan::new(7)).unwrap();
    let bytes = persistence::encode(&world, &content).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["schema"], 21);
    json["schema"] = 20.into();
    assert!(persistence::decode(&serde_json::to_vec(&json).unwrap(), &content).is_err());
    let mut restored = persistence::decode(&bytes, &content).unwrap();
    world
        .advance(&content, sim_core::DaySpan::new(120))
        .unwrap();
    restored
        .advance(&content, sim_core::DaySpan::new(120))
        .unwrap();
    assert_eq!(restored, world);
    replay(&world, &content);
    assert_eq!(Content::technology_bundled().unwrap().rules, 18);
    assert!(Content::bundled_for_hash(content.hash()).is_err());
}

fn unsupported_platform() -> Content {
    let mut content = fixture();
    let d = content
        .countries
        .iter_mut()
        .find(|c| c.id == player())
        .unwrap();
    d.tax = 10;
    d.services = 4;
    d.welfare = 0;
    let profile = content
        .party_preferences
        .as_mut()
        .unwrap()
        .profiles
        .iter_mut()
        .find(|p| p.party.0 == "AUR-1")
        .unwrap();
    for position in &mut profile.positions {
        let maximum = content
            .policies
            .iter()
            .find(|p| p.id == position.policy)
            .unwrap()
            .max;
        position.preferred = Some(PreferredPolicyRange {
            min: maximum,
            max: maximum,
        });
        position.importance = if position.policy == "income_tax" {
            1
        } else {
            100
        };
    }
    content.validate().unwrap();
    content
}
#[test]
fn misalignment_can_block_authority_and_costed_support_restores_it_without_free_votes() {
    let content = unsupported_platform();
    let mut world = World::new(&content, player(), 17).unwrap();
    assert_eq!(
        world
            .party_alignment_for(
                &content,
                &player(),
                &FiscalTargets::current(world.player_country())
            )
            .unwrap()
            .unwrap()
            .penalty,
        20
    );
    while world.player_country().legislative_support >= 51 && world.day.units() < 360 {
        world.advance(&content, sim_core::DaySpan::new(30)).unwrap();
    }
    assert!(world.player_country().legislative_support < 51);
    let before = world.hash();
    assert!(
        world
            .preview(&content, &player(), &"energy".into(), 1)
            .is_err()
    );
    assert_eq!(world.hash(), before);
    let cash = world.player_country().accounts.treasury;
    let support = world.player_country().legislative_support;
    world
        .issue(
            &content,
            Action::Enact {
                policy: "coalition".into(),
                value: 1,
            },
        )
        .unwrap();
    assert!(world.player_country().accounts.treasury.0 < cash.0);
    assert_eq!(world.player_country().legislative_support, support);
    let due = world
        .player_country()
        .projects
        .iter()
        .find(|p| p.policy == "coalition")
        .unwrap()
        .due_day;
    world
        .advance(&content, sim_core::DaySpan::new(due - world.day.units()))
        .unwrap();
    assert!(world.player_country().legislative_support >= 51);
    assert!(
        world
            .preview(&content, &player(), &"energy".into(), 1)
            .is_ok()
    );
    assert!(!world.is_developer_modified());
    replay(&world, &content);
}
#[test]
fn shared_ai_pays_for_governing_support_when_party_pressure_erodes_backing() {
    let content = unsupported_platform();
    let mut world = World::new(&content, CountryId("COR".into()), 17).unwrap();
    world
        .advance(&content, sim_core::DaySpan::new(720))
        .unwrap();
    assert!(world.journal.iter().any(|entry| entry.command.issuer == player() && matches!(&entry.command.action, Action::Enact { policy, .. } if policy == "coalition")));
    assert!(
        world.countries[&player()].budget.capital.0 > 0
            || world
                .events
                .iter()
                .any(|event| event.country == player() && event.text.contains("coalition"))
    );
    assert!(!world.is_developer_modified());
    replay(&world, &content);
}

#[test]
fn later_party_rules_can_leave_reference_geography_unknown_and_budget_feedback_separate() {
    let mut content = fixture();
    content.sovereignty = None;
    content.validate().unwrap();
    let world = World::new(&content, player(), 17).unwrap();
    let before = world.hash();
    assert!(
        world
            .public_report(&content, &player())
            .unwrap()
            .bookmark_sovereignty
            .is_none()
    );
    let preview = world
        .preview_budget(&content, &player(), 2000, &hostile())
        .unwrap();
    assert_eq!(
        preview.route,
        world.governing_rules(&content, &player()).unwrap().label()
    );
    assert!(
        preview
            .party_preference_feedback
            .contains("Party alignment")
    );
    assert!(
        preview
            .party_preference_feedback
            .contains("monthly political reviews")
    );
    assert_eq!(world.hash(), before);
    let mut old = content;
    old.party_preferences = None;
    old.schema = 19;
    old.rules = 19;
    assert!(old.validate().is_err());
}
#[test]
fn leadership_uses_the_ruling_party_id_and_royal_routes_ignore_party_influence_caps() {
    let mut content = fixture();
    let boreal = content
        .countries
        .iter_mut()
        .find(|c| c.id.0 == "BOR")
        .unwrap();
    boreal.ruling_party = "BOR-2".into();
    boreal.coalition = vec!["BOR-2".into()];
    boreal.executive = boreal
        .parties
        .iter()
        .find(|p| p.id == "BOR-2")
        .unwrap()
        .leader
        .clone();
    let rule = content
        .party_preferences
        .as_mut()
        .unwrap()
        .influences
        .iter_mut()
        .find(|r| r.country.0 == "COR")
        .unwrap();
    rule.legislative_max_penalty = 20;
    rule.party_leadership_max_penalty = 20;
    content.validate().unwrap();
    let boreal = World::new(&content, CountryId("BOR".into()), 17).unwrap();
    let report = boreal
        .party_alignment_for(&content, &boreal.player, &hostile())
        .unwrap()
        .unwrap();
    assert_eq!(report.penalty, 0);
    assert_eq!(report.known_positions_bps.units(), 0);
    assert_eq!(
        report
            .parties
            .iter()
            .find(|p| p.party.0 == "BOR-1")
            .unwrap()
            .weight,
        0
    );
    assert_eq!(
        report
            .parties
            .iter()
            .find(|p| p.party.0 == "BOR-2")
            .unwrap()
            .weight,
        1
    );
    let royal = World::new(&content, CountryId("COR".into()), 17).unwrap();
    let report = royal
        .party_alignment_for(&content, &royal.player, &hostile())
        .unwrap()
        .unwrap();
    assert_eq!(report.metric, None);
    assert_eq!(report.max_penalty, 0);
    assert_eq!(report.penalty, 0);
}
