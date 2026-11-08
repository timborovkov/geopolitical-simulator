use sim_core::*;
use std::collections::{BTreeMap, BTreeSet};
fn tags(names: &[&str]) -> BTreeSet<SocialGroupId> {
    names.iter().map(|name| (*name).into()).collect()
}
fn fixture() -> (
    SocialMembershipProfile,
    BTreeMap<SocialGroupId, SocialGroupDefinition>,
) {
    let catalogue = ["parents", "union", "faith"]
        .into_iter()
        .map(|id| {
            (
                id.into(),
                SocialGroupDefinition {
                    id: id.into(),
                    name: format!("Authored {id}"),
                    description: "Explicit fictional test membership; not national prevalence."
                        .into(),
                },
            )
        })
        .collect();
    let cells = [
        ("none", &[][..], 2000),
        ("parents", &["parents"][..], 2000),
        ("union", &["union"][..], 1000),
        ("shared", &["parents", "union", "faith"][..], 3000),
        ("faith", &["faith"][..], 2000),
    ]
    .into_iter()
    .map(|(id, members, share)| SocialCellDefinition {
        id: id.into(),
        memberships: tags(members),
        share: BasisPoints::new(share),
    })
    .collect();
    (SocialMembershipProfile { cells }, catalogue)
}
#[test]
fn overlapping_counts_can_exceed_residents_while_unions_intersections_and_accounts_count_people_once()
 {
    let (profile, catalogue) = fixture();
    let counts = profile.project(Population(1000), &catalogue).unwrap();
    assert_eq!(
        counts
            .cells
            .iter()
            .map(|cell| cell.population.0)
            .sum::<u64>(),
        1000
    );
    assert_eq!(counts.members(&"parents".into()).unwrap(), Population(500));
    assert_eq!(counts.members(&"union".into()).unwrap(), Population(400));
    assert_eq!(counts.members(&"faith".into()).unwrap(), Population(500));
    assert_eq!(
        catalogue
            .keys()
            .map(|group| counts.members(group).unwrap().0)
            .sum::<u64>(),
        1400
    );
    assert_eq!(
        counts.count_any(&tags(&["parents", "union"])).unwrap(),
        Population(600)
    );
    assert_eq!(
        counts.count_all(&tags(&["parents", "union"])).unwrap(),
        Population(300)
    );
    assert_eq!(
        counts
            .count_any(&tags(&["parents", "union", "faith"]))
            .unwrap(),
        Population(800)
    );
    assert_eq!(
        counts
            .count_none(&tags(&["parents", "union", "faith"]))
            .unwrap(),
        Population(200)
    );
    assert_eq!(counts.count_any(&tags(&[])).unwrap(), Population(0));
    assert_eq!(counts.count_all(&tags(&[])).unwrap(), Population(1000));
    assert!(counts.members(&"unknown".into()).is_err());
    counts.validate(&profile, &catalogue).unwrap();
}
#[test]
fn stable_cell_ids_set_rounding_ties_regardless_of_profile_order_or_group_names() {
    let (mut profile, mut catalogue) = fixture();
    profile.cells = vec![
        SocialCellDefinition {
            id: "z-last".into(),
            memberships: tags(&["parents"]),
            share: BasisPoints::new(5000),
        },
        SocialCellDefinition {
            id: "a-first".into(),
            memberships: tags(&[]),
            share: BasisPoints::new(5000),
        },
    ];
    let expected = profile.project(Population(1), &catalogue).unwrap();
    assert_eq!(expected.cells[0].id, "a-first".into());
    assert_eq!(expected.cells[0].population, Population(1));
    profile.cells.reverse();
    for group in catalogue.values_mut() {
        group.name = "A renamed display label".into();
    }
    assert_eq!(
        profile.project(Population(1), &catalogue).unwrap(),
        expected
    );
    for total in [0, 1, 7, 101, 1_000_003, u64::MAX] {
        let projected = profile.project(Population(total), &catalogue).unwrap();
        assert_eq!(
            projected
                .cells
                .iter()
                .map(|c| u128::from(c.population.0))
                .sum::<u128>(),
            u128::from(total)
        );
        assert!(projected.count_all(&tags(&[])).unwrap().0 == total);
    }
}
#[test]
fn missing_share_mass_duplicate_joint_cells_unknown_groups_and_forged_counts_are_rejected() {
    let (original, catalogue) = fixture();
    for variant in 0..5 {
        let mut profile = original.clone();
        match variant {
            0 => profile.cells[0].share = BasisPoints::new(1999),
            1 => profile.cells[0].id = profile.cells[1].id.clone(),
            2 => profile.cells[0].memberships = profile.cells[1].memberships.clone(),
            3 => {
                profile.cells[0].memberships.insert("missing".into());
            }
            4 => profile.cells[0].id = "Display Name".into(),
            _ => unreachable!(),
        }
        assert!(
            profile.project(Population(1000), &catalogue).is_err(),
            "variant {variant}"
        );
    }
    let valid = original.project(Population(1000), &catalogue).unwrap();
    let mut forged = valid.clone();
    forged.cells[0].population.0 += 1;
    assert!(forged.validate(&original, &catalogue).is_err());
    let restored: OverlappingMemberships =
        serde_json::from_slice(&serde_json::to_vec(&valid).unwrap()).unwrap();
    assert_eq!(restored, valid);
    restored.validate(&original, &catalogue).unwrap();
}
#[test]
fn projection_over_live_disjoint_cohorts_changes_no_resident_cash_or_government_state() {
    let content = Content::demographic_bundled().unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
    world.advance(&content, DaySpan::new(90)).unwrap();
    let before = world.clone();
    let (profile, catalogue) = fixture();
    for country in world.countries.values() {
        let projections: Vec<_> = country
            .households
            .iter()
            .map(|household| CohortSocialMemberships {
                cohort: household.id.clone(),
                memberships: profile.project(household.population, &catalogue).unwrap(),
            })
            .collect();
        assert_eq!(
            projections
                .iter()
                .map(|p| p.memberships.population.0)
                .sum::<u64>(),
            country.population.0
        );
        let memberships: u128 = projections
            .iter()
            .flat_map(|p| {
                catalogue
                    .keys()
                    .map(|id| u128::from(p.memberships.members(id).unwrap().0))
            })
            .sum();
        assert!(memberships > u128::from(country.population.0));
    }
    assert_eq!(world, before);
}
