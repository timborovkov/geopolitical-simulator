use sim_core::*;
use std::collections::BTreeSet;
fn fixture() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-overlapping-memberships-test.json"
    ))
    .unwrap()
}
fn selection() -> BTreeSet<SocialGroupId> {
    BTreeSet::from(["employment".into(), "parents".into()])
}
#[test]
fn owner_reports_reconcile_unique_people_and_actual_jobs_without_issuing_or_funding_orders() {
    for (content, id) in [
        (fixture(), "AUR"),
        (Content::social_memberships_bundled().unwrap(), "GBR"),
    ] {
        let mut world = World::new(&content, CountryId(id.into()), 17).unwrap();
        world.advance(&content, DaySpan::new(90)).unwrap();
        let before = world.clone();
        let report = world
            .social_membership_report(
                &content,
                Viewer::Government(world.player.clone()),
                &world.player,
                &selection(),
            )
            .unwrap()
            .unwrap();
        let counts = report.owned.unwrap();
        let country = world.player_country();
        assert_eq!(counts.population, country.population);
        assert_eq!(
            counts.members[&"employment".into()].0,
            country
                .households
                .iter()
                .map(|group| group.employed.0)
                .sum::<u64>()
        );
        assert!(
            counts
                .members
                .values()
                .map(|count| u128::from(count.0))
                .sum::<u128>()
                > u128::from(country.population.0)
        );
        assert_eq!(
            counts
                .cohorts
                .iter()
                .map(|cohort| cohort.population.0)
                .sum::<u64>(),
            country.population.0
        );
        assert!(
            counts.any_selected.0 >= counts.members[&"employment".into()].0
                && counts.any_selected.0 <= country.population.0
        );
        assert!(counts.all_selected.0 <= counts.members[&"parents".into()].0);
        assert_eq!(
            counts.any_selected.0 + counts.none_selected.0,
            country.population.0
        );
        assert_eq!(world, before);
    }
}
#[test]
fn observer_and_foreign_views_return_definitions_without_private_memberships_or_age_counts() {
    let content = fixture();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.advance(&content, DaySpan::new(30)).unwrap();
    for (viewer, id) in [
        (Viewer::Observer, world.player.clone()),
        (
            Viewer::Government(CountryId("BOR".into())),
            world.player.clone(),
        ),
        (
            Viewer::Government(world.player.clone()),
            CountryId("BOR".into()),
        ),
        (
            Viewer::Government(CountryId("BOR".into())),
            CountryId("BOR".into()),
        ),
    ] {
        let report = world
            .social_membership_report(&content, viewer, &id, &selection())
            .unwrap()
            .unwrap();
        assert!(report.owned.is_none());
        let encoded = serde_json::to_value(&report).unwrap();
        assert!(encoded.get("owned").is_none());
        assert_eq!(report.groups.len(), 4);
        assert!(!encoded.to_string().contains("\"any_selected\""));
    }
    assert!(
        world
            .social_membership_report(
                &content,
                Viewer::Government(world.player.clone()),
                &world.player,
                &BTreeSet::from(["missing".into()])
            )
            .is_err()
    );
    assert!(
        world
            .social_membership_report(
                &Content::bundled().unwrap(),
                Viewer::Observer,
                &world.player,
                &selection()
            )
            .is_err()
    );
}
#[test]
fn joint_profiles_preserve_every_opening_input_and_all_population_economic_and_governing_rules() {
    let content = Content::social_memberships_bundled().unwrap();
    let previous = Content::demographic_bundled().unwrap();
    assert_eq!(content.countries, previous.countries);
    assert_eq!(content.policies, previous.policies);
    assert_eq!(content.facts, previous.facts);
    assert_eq!(content.demographics, previous.demographics);
    assert_eq!(
        content.governing_institutions,
        previous.governing_institutions
    );
    assert_eq!(Content::bundled_for_hash(content.hash()).unwrap(), content);
    assert_eq!(
        Content::bundled_for_hash(previous.hash()).unwrap(),
        previous
    );
    let mut before = World::new(&previous, CountryId("GBR".into()), 17).unwrap();
    let mut after = World::new(&content, before.player.clone(), 17).unwrap();
    before.advance(&previous, DaySpan::new(90)).unwrap();
    after.advance(&content, DaySpan::new(90)).unwrap();
    assert_eq!(before.countries, after.countries);
    assert_eq!(before.demographics, after.demographics);
    assert_eq!(before.journal, after.journal);
    assert_ne!(before.content_hash, after.content_hash);
    assert!(
        before
            .social_membership_report(&previous, Viewer::Observer, &before.player, &selection())
            .unwrap()
            .is_none()
    );
}
#[test]
fn versioned_profiles_inspect_and_resume_replay_exactly_without_persisting_derived_membership_copies()
 {
    let content = fixture();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.advance(&content, DaySpan::new(90)).unwrap();
    let original = persistence::encode(&world, &content).unwrap();
    assert_eq!(persistence::snapshot_schema(&world), 31);
    let report = world
        .social_membership_report(
            &content,
            Viewer::Government(world.player.clone()),
            &world.player,
            &selection(),
        )
        .unwrap();
    assert!(report.unwrap().owned.is_some());
    assert_eq!(original, persistence::encode(&world, &content).unwrap());
    let mut resumed = persistence::decode(&original, &content).unwrap();
    assert_eq!(persistence::replay(&resumed, &content).unwrap(), world);
    world.advance(&content, DaySpan::new(40)).unwrap();
    resumed.advance(&content, DaySpan::new(40)).unwrap();
    assert_eq!(resumed, world);
    world.validate(&content).unwrap();
    let mut old_header: serde_json::Value = serde_json::from_slice(&original).unwrap();
    old_header["schema"] = 30.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&old_header).unwrap(), &content)
            .unwrap_err()
            .contains("format 31")
    );
    let mut old_content = serde_json::to_value(&content).unwrap();
    old_content["schema"] = 29.into();
    old_content["rules"] = 29.into();
    assert!(Content::from_json(&old_content.to_string()).is_err());
}
#[test]
fn joint_profile_order_and_translated_labels_do_not_control_membership_rounding_or_employment() {
    let content = fixture();
    let before = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let original = before
        .social_membership_report(
            &content,
            Viewer::Government(before.player.clone()),
            &before.player,
            &selection(),
        )
        .unwrap()
        .unwrap()
        .owned;
    let mut renamed = content.clone();
    let definition = renamed.social_memberships.as_mut().unwrap();
    for group in definition.catalogue.groups.values_mut() {
        group.name = "A renamed social role".into();
    }
    for country in definition.profiles.values_mut() {
        for scopes in country.values_mut() {
            for profile in scopes.values_mut() {
                profile.cells.reverse();
            }
        }
    }
    renamed.validate().unwrap();
    let after = World::new(&renamed, before.player.clone(), 17).unwrap();
    assert_eq!(
        after
            .social_membership_report(
                &renamed,
                Viewer::Government(after.player.clone()),
                &after.player,
                &selection()
            )
            .unwrap()
            .unwrap()
            .owned,
        original
    );
    let mut invalid = renamed.clone();
    let definitions = invalid.social_memberships.as_mut().unwrap();
    definitions
        .profiles
        .get_mut(&before.player)
        .unwrap()
        .values_mut()
        .next()
        .unwrap()
        .get_mut(&SocialPopulationScope::Young)
        .unwrap()
        .cells[0]
        .memberships
        .insert("union".into());
    assert!(invalid.validate().is_err());
}
