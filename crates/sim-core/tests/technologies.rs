use sim_core::*;
fn json() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../data/synthetic-technology-eligibility-test.json"
    ))
    .unwrap()
}
fn fixture() -> Content {
    Content::from_json(&json().to_string()).unwrap()
}
fn replay(w: &World, c: &Content) {
    w.validate(c).unwrap();
    assert_eq!(persistence::replay(w, c).unwrap(), *w);
}

#[test]
fn date_restriction_rejects_without_payment_then_commissions_through_normal_funding_and_delivery() {
    let c = fixture();
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    let hash = w.hash();
    let rows = w
        .policy_technology_eligibility(&c, &w.player, &"energy".into())
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].country_eligible);
    assert!(!rows[0].date_eligible);
    assert!(!rows[0].available());
    assert!(
        w.preview(&c, &w.player, &"energy".into(), 1)
            .unwrap_err()
            .contains("2000-01-31")
    );
    assert!(w.policy_funding_plan(&c, &"energy".into(), 1).is_err());
    assert!(
        w.issue(
            &c,
            Action::Enact {
                policy: "energy".into(),
                value: 1
            }
        )
        .unwrap_err()
        .contains("2000-01-31")
    );
    assert_eq!(
        w.hash(),
        hash,
        "query and rejected order must not pay fees or reserve cash"
    );
    w.advance(&c, 29).unwrap();
    assert!(w.preview(&c, &w.player, &"energy".into(), 1).is_err());
    let capacity = w.player_country().energy_capacity;
    assert_eq!(w.advance_to_event(&c).unwrap(), 1);
    assert_eq!(w.day, 30);
    assert_eq!(w.player_country().energy_capacity, capacity);
    let p = w.preview(&c, &w.player, &"energy".into(), 1).unwrap();
    let treasury = w.player_country().accounts.treasury;
    w.issue(
        &c,
        Action::Enact {
            policy: "energy".into(),
            value: 1,
        },
    )
    .unwrap();
    assert_eq!(
        w.player_country().accounts.treasury.0,
        treasury.0 - p.upfront_payment.0
    );
    assert_eq!(
        w.player_country().committed_cash.0,
        p.cost.0 - p.upfront_payment.0
    );
    w.advance(&c, p.delay - 1).unwrap();
    assert_eq!(w.player_country().energy_capacity, capacity);
    w.advance(&c, 1).unwrap();
    assert_eq!(w.player_country().energy_capacity, capacity + 15);
    replay(&w, &c);
    let mut incompatible = c.clone();
    incompatible.technologies.as_mut().unwrap().definitions[0].available_from =
        "2000-03-01".parse().unwrap();
    assert!(w.day >= 60);
    let mut forged = w.clone();
    forged.content_hash = incompatible.hash();
    assert!(
        forged.validate(&incompatible).is_err(),
        "accepted historical commands must satisfy their own dated prerequisites"
    );
}
#[test]
fn all_prerequisites_and_country_eligibility_apply_and_queries_use_exact_public_scenario_data() {
    let mut data = json();
    data["technologies"]["definitions"][0]["eligible_countries"] =
        serde_json::json!(["BOR", "COR"]);
    data["technologies"]["policy_requirements"][0]["technologies"] =
        serde_json::json!(["grid_planning", "agronomy"]);
    let c = Content::from_json(&data.to_string()).unwrap();
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    w.advance(&c, 30).unwrap();
    let hash = w.hash();
    let rows = w
        .policy_technology_eligibility(&c, &w.player, &"energy".into())
        .unwrap();
    assert!(!rows[0].country_eligible);
    assert!(rows[0].date_eligible);
    assert!(rows[1].available());
    assert!(
        w.preview(&c, &w.player, &"energy".into(), 1)
            .unwrap_err()
            .contains("not eligible for this country")
    );
    assert!(
        w.policy_technology_eligibility(&c, &CountryId("BOR".into()), &"energy".into())
            .unwrap()
            .iter()
            .all(TechnologyEligibility::available)
    );
    assert!(
        w.policy_technology_eligibility(&c, &CountryId("MISSING".into()), &"energy".into())
            .is_err()
    );
    let mut wrong = c.clone();
    wrong.name.push('!');
    assert!(
        w.policy_technology_eligibility(&wrong, &w.player, &"energy".into())
            .is_err()
    );
    assert_eq!(w.hash(), hash);
    data["technologies"]["definitions"][0]["eligible_countries"] =
        serde_json::json!(["AUR", "BOR", "COR"]);
    data["technologies"]["definitions"][1]["available_from"] = "2000-03-01".into();
    let c = Content::from_json(&data.to_string()).unwrap();
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    w.advance(&c, 30).unwrap();
    assert!(
        w.preview(&c, &w.player, &"energy".into(), 1)
            .unwrap_err()
            .contains("Agronomic planning")
    );
    w.advance(&c, 30).unwrap();
    assert!(w.preview(&c, &w.player, &"energy".into(), 1).is_ok());
    replay(&w, &c);
}
#[test]
fn malformed_definitions_references_dates_and_unversioned_gates_are_rejected() {
    for case in 0..10 {
        let mut d = json();
        match case {
            0 => d["technologies"]["definitions"][1]["id"] = "grid_planning".into(),
            1 => {
                d["technologies"]["definitions"][0]["eligible_countries"] =
                    serde_json::json!(["UNKNOWN"])
            }
            2 => {
                d["technologies"]["definitions"][0]["eligible_countries"] =
                    serde_json::json!(["AUR", "AUR"])
            }
            3 => d["technologies"]["definitions"][0]["source"] = "missing".into(),
            4 => d["technologies"]["definitions"][0]["available_from"] = "2100-01-02".into(),
            5 => d["technologies"]["definitions"][0]["available_from"] = "2000-02-30".into(),
            6 => d["technologies"]["policy_requirements"][0]["policy"] = "missing".into(),
            7 => {
                d["technologies"]["policy_requirements"][0]["technologies"] =
                    serde_json::json!(["missing"])
            }
            8 => {
                d["technologies"]["policy_requirements"][0]["technologies"] =
                    serde_json::json!(["grid_planning", "grid_planning"])
            }
            _ => d["technologies"] = serde_json::Value::Null,
        }
        assert!(Content::from_json(&d.to_string()).is_err(), "case {case}");
    }
    let mut old = Content::interest_bundled().unwrap();
    old.technologies = fixture().technologies;
    assert!(old.validate().is_err());
    let c = Content::interest_bundled().unwrap();
    let w = World::new(&c, CountryId("GBR".into()), 17).unwrap();
    assert!(
        w.policy_technology_eligibility(&c, &w.player, &"energy".into())
            .unwrap()
            .is_empty()
    );
}
#[test]
fn snapshot_19_replays_one_date_notice_and_later_bookmarks_initialize_eligibility() {
    let c = fixture();
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    w.advance(&c, 29).unwrap();
    let bytes = persistence::encode(&w, &c).unwrap();
    let mut data: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(data["schema"], 19);
    let mut resumed = persistence::decode(&bytes, &c).unwrap();
    assert_eq!(w.advance_to_event(&c).unwrap(), 1);
    resumed.advance(&c, 1).unwrap();
    assert_eq!(w, resumed);
    w.advance(&c, 1).unwrap();
    assert_eq!(
        w.events
            .iter()
            .filter(|e| e.country == w.player
                && e.text
                    .contains("Technology eligibility date reached: Grid planning"))
            .count(),
        1
    );
    data["schema"] = 18.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&data).unwrap(), &c)
            .unwrap_err()
            .contains("format 19")
    );
    replay(&w, &c);
    let later = c.at_bookmark("2001-07-15").unwrap();
    let later_world = World::new(&later, CountryId("AUR".into()), 17).unwrap();
    assert!(
        later_world
            .policy_technology_eligibility(&later, &later_world.player, &"energy".into())
            .unwrap()[0]
            .available()
    );
    assert!(
        w.policy_technology_eligibility(&c, &w.player, &"agriculture".into())
            .unwrap()[0]
            .available()
    );
}
#[test]
fn fiscal_packages_cannot_precommission_a_programme_before_its_prerequisite_is_available() {
    let mut data = json();
    data["technologies"]["policy_requirements"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"policy":"income_tax","technologies":["grid_planning"]}));
    let c = Content::from_json(&data.to_string()).unwrap();
    let mut w = World::new(&c, CountryId("COR".into()), 17).unwrap();
    let mut targets = FiscalTargets::current(w.player_country());
    targets.income_tax = 28;
    let hash = w.hash();
    assert!(
        w.preview_budget(&c, &w.player, 2000, &targets)
            .unwrap_err()
            .contains("Grid planning")
    );
    assert!(
        w.issue(
            &c,
            Action::ProposeBudget {
                year: 2000,
                targets: targets.clone()
            }
        )
        .is_err()
    );
    assert_eq!(w.hash(), hash);
    w.advance(&c, 30).unwrap();
    assert!(w.preview_budget(&c, &w.player, 2000, &targets).is_ok());
    w.issue(
        &c,
        Action::ProposeBudget {
            year: 2000,
            targets,
        },
    )
    .unwrap();
    replay(&w, &c);
}
#[test]
fn nonplayer_supply_orders_obey_the_same_date_gate_and_have_no_free_capacity() {
    let mut data = json();
    data["technologies"]["definitions"][0]["available_from"] = "2000-03-01".into();
    for country in data["countries"].as_array_mut().unwrap() {
        country["energy"] = 20.into();
    }
    let c = Content::from_json(&data.to_string()).unwrap();
    let mut w = World::new(&c, CountryId("COR".into()), 17).unwrap();
    w.advance(&c, 59).unwrap();
    let energy_order = |entry: &JournalEntry| matches!(&entry.command.action,Action::Enact {policy,..} if policy=="energy");
    assert!(!w.journal.iter().any(energy_order));
    assert!(
        w.countries
            .values()
            .all(|country| country.energy_capacity == 20)
    );
    w.advance(&c, 91).unwrap();
    let orders: Vec<_> = w
        .journal
        .iter()
        .filter(|entry| entry.ai && energy_order(entry))
        .collect();
    assert!(!orders.is_empty());
    assert!(orders.iter().all(|entry| entry.command.day >= 60));
    replay(&w, &c);
}
