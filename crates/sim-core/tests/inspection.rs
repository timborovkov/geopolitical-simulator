use sim_core::*;

fn fixture() -> (Content, World) {
    let content = Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world
        .issue(&content, Action::Borrow { amount: Money(100) })
        .unwrap();
    world.advance(&content, sim_core::DaySpan::new(60)).unwrap();
    (content, world)
}

#[test]
fn malformed_journals_are_rejected_before_encoding_or_replay() {
    let (content, world) = fixture();
    for variant in 0..6 {
        let mut bad = world.clone();
        match variant {
            0 => bad.journal[0].command.id = CommandId(99),
            1 => bad.journal[0].command.day = sim_core::CampaignDay::new(world.day.units() + 1),
            2 => bad.journal[0].command.issuer = CountryId("absent".into()),
            3 => bad.journal[0].ai = true,
            4 => {
                bad.journal[0].command.action = Action::Enact {
                    policy: "unknown".into(),
                    value: 1,
                }
            }
            _ => bad.journal[0].command.action = Action::Borrow { amount: Money(-1) },
        }
        assert!(
            bad.validate(&content)
                .unwrap_err()
                .contains("journal entry 1")
        );
        assert!(persistence::encode(&bad, &content).is_err());
        assert!(persistence::replay(&bad, &content).is_err());
    }
}

#[test]
fn replay_identifies_the_first_changed_ai_command_without_mutating_the_snapshot() {
    let (content, mut world) = fixture();
    let entry = world.journal.iter_mut().find(|entry| entry.ai).unwrap();
    let index = entry.command.id;
    entry.command.action = Action::Borrow { amount: Money(1) };
    world.validate(&content).unwrap();
    let before = world.hash();
    let error = persistence::replay(&world, &content).unwrap_err();
    assert!(
        error.contains(&format!("first command divergence at entry {index}")),
        "{error}"
    );
    assert_eq!(world.hash(), before);
}

#[test]
fn final_snapshot_difference_names_the_country_field_and_values() {
    let (content, mut world) = fixture();
    let expected = world.clone();
    world.countries.get_mut(&world.player).unwrap().tax += 1;
    world.validate(&content).unwrap();
    let difference = persistence::first_state_difference(&world, &expected).unwrap();
    assert_eq!(difference.path, "/countries/AUR/tax");
    assert_eq!(difference.expected, "25");
    assert_eq!(difference.actual, "24");
    assert!(persistence::first_state_difference(&world, &world).is_none());
    let error = persistence::replay(&world, &content).unwrap_err();
    assert!(
        error.contains("first differing final-state field /countries/AUR/tax"),
        "{error}"
    );
}

#[test]
fn next_event_reaches_a_leap_year_fiscal_boundary_after_365_quiet_days() {
    let mut content =
        Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
    // A solvent authored fixture and seed isolate the next annual review.
    for country in &mut content.countries {
        country.tax = Percent::new(30);
        country.services = Percent::new(6);
        country.welfare = Percent::new(4);
    }
    content.validate().unwrap();
    let opening = World::new(&content, CountryId("AUR".into()), 0).unwrap();
    let mut world = opening.clone();
    assert_eq!(world.advance_to_event(&content).unwrap(), DaySpan::new(366));
    assert_eq!(date(world.day), "2001-01-01");
    assert!(world.events.iter().any(|e| e.country == world.player
        && e.day == sim_core::CampaignDay::new(366)
        && e.important
        && e.text.starts_with("Fiscal year closed")));
    let mut direct = opening;
    direct
        .advance(&content, sim_core::DaySpan::new(366))
        .unwrap();
    assert_eq!(world, direct);
    assert_eq!(world, persistence::replay(&world, &content).unwrap());
}

#[test]
fn next_event_stops_cleanly_at_the_campaign_limit() {
    let (content, mut world) = fixture();
    // A boundary fixture isolates clock behavior without a century-long soak.
    world.day = sim_core::CampaignDay::new(MAX_DAY - 1);
    assert_eq!(world.advance_to_event(&content).unwrap(), DaySpan::new(1));
    assert_eq!(world.day.units(), MAX_DAY);
    let before = world.hash();
    assert_eq!(world.advance_to_event(&content).unwrap(), DaySpan::new(0));
    assert_eq!(world.hash(), before);
    world.day = sim_core::CampaignDay::new(MAX_DAY + 1);
    assert!(world.advance_to_event(&content).is_err());
}
