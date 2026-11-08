use sim_core::*;
use std::collections::{BTreeMap, BTreeSet};
fn tags(names: &[&str]) -> BTreeSet<SocialGroupId> {
    names.iter().map(|name| (*name).into()).collect()
}
fn profile(first: &[&str], second: &[&str], share: u32) -> SocialMembershipProfile {
    SocialMembershipProfile {
        cells: vec![
            SocialCellDefinition {
                id: "first".into(),
                memberships: tags(first),
                share: BasisPoints::new(share),
            },
            SocialCellDefinition {
                id: "second".into(),
                memberships: tags(second),
                share: BasisPoints::new(10000 - share),
            },
        ],
    }
}
fn fixture() -> (
    ScopedSocialCatalogue,
    BTreeMap<SocialPopulationScope, SocialMembershipProfile>,
) {
    let groups = ["employment", "parents", "union", "faith"]
        .into_iter()
        .map(|id| {
            (
                id.into(),
                SocialGroupDefinition {
                    id: id.into(),
                    name: format!("Authored {id}"),
                    description: "Fictional joint-profile test, not observed national prevalence."
                        .into(),
                },
            )
        })
        .collect();
    let eligibility = BTreeMap::from([
        ("employment".into(), SocialEligibility::EmployedResident),
        ("parents".into(), SocialEligibility::AgeFifteenPlus),
        ("union".into(), SocialEligibility::EmployedResident),
        ("faith".into(), SocialEligibility::AnyResident),
    ]);
    let profiles = BTreeMap::from([
        (SocialPopulationScope::Young, profile(&[], &["faith"], 5000)),
        (
            SocialPopulationScope::WorkingEmployed,
            profile(
                &["employment"],
                &["employment", "parents", "union", "faith"],
                4000,
            ),
        ),
        (
            SocialPopulationScope::WorkingOther,
            profile(&[], &["parents", "faith"], 5000),
        ),
        (
            SocialPopulationScope::Older,
            profile(&[], &["parents", "faith"], 5000),
        ),
    ]);
    (
        ScopedSocialCatalogue {
            groups,
            eligibility,
            employment_group: "employment".into(),
        },
        profiles,
    )
}
fn ages() -> AgeStocks {
    AgeStocks {
        young: Population(300),
        working_age: Population(600),
        older: Population(100),
    }
}
#[test]
fn unique_age_scopes_preserve_actual_jobs_and_restrict_age_or_employed_memberships() {
    let (catalogue, profiles) = fixture();
    let result = catalogue
        .project(ages(), Population(400), &profiles)
        .unwrap();
    assert_eq!(result.population, Population(1000));
    assert_eq!(
        result.members(&"employment".into()).unwrap(),
        Population(400)
    );
    assert_eq!(result.members(&"parents".into()).unwrap(), Population(390));
    assert_eq!(result.members(&"union".into()).unwrap(), Population(240));
    assert_eq!(result.members(&"faith".into()).unwrap(), Population(540));
    assert_eq!(
        result
            .count_all(&tags(&["parents", "union", "faith", "employment"]))
            .unwrap(),
        Population(240)
    );
    assert_eq!(
        result
            .count_none(&tags(&["parents", "union", "faith", "employment"]))
            .unwrap(),
        Population(300)
    );
    assert_eq!(
        result.scopes[&SocialPopulationScope::Young]
            .members(&"parents".into())
            .unwrap(),
        Population(0)
    );
    assert_eq!(
        result.scopes[&SocialPopulationScope::WorkingOther]
            .members(&"union".into())
            .unwrap(),
        Population(0)
    );
    let mut profiles_reordered = profiles.clone();
    for profile in profiles_reordered.values_mut() {
        profile.cells.reverse();
    }
    assert_eq!(
        catalogue
            .project(ages(), Population(400), &profiles_reordered)
            .unwrap(),
        result
    );
    let restored: ScopedSocialMemberships =
        serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    restored
        .validate(&catalogue, &profiles, ages(), Population(400))
        .unwrap();
    let zero = catalogue
        .project(AgeStocks::default(), Population(0), &profiles)
        .unwrap();
    assert_eq!(zero.count_all(&BTreeSet::new()).unwrap(), Population(0));
}
#[test]
fn missing_scopes_child_union_membership_incomplete_job_tags_and_excess_jobs_are_rejected() {
    let (catalogue, original) = fixture();
    for variant in 0..4 {
        let mut profiles = original.clone();
        match variant {
            0 => {
                profiles.remove(&SocialPopulationScope::Older);
            }
            1 => {
                profiles
                    .get_mut(&SocialPopulationScope::Young)
                    .unwrap()
                    .cells[0]
                    .memberships
                    .insert("union".into());
            }
            2 => {
                profiles
                    .get_mut(&SocialPopulationScope::WorkingEmployed)
                    .unwrap()
                    .cells[0]
                    .memberships
                    .remove(&catalogue.employment_group);
            }
            3 => {
                profiles
                    .get_mut(&SocialPopulationScope::WorkingOther)
                    .unwrap()
                    .cells[0]
                    .memberships
                    .insert(catalogue.employment_group.clone());
            }
            _ => unreachable!(),
        }
        assert!(
            catalogue
                .project(ages(), Population(400), &profiles)
                .is_err(),
            "variant {variant}"
        );
    }
    assert!(
        catalogue
            .project(ages(), Population(601), &original)
            .is_err()
    );
    let mut invalid = catalogue.clone();
    invalid.eligibility.remove(&"faith".into());
    assert!(invalid.validate_profiles(&original).is_err());
    let valid = catalogue
        .project(ages(), Population(400), &original)
        .unwrap();
    let mut forged = valid.clone();
    forged.population = Population(1001);
    assert!(
        forged
            .validate(&catalogue, &original, ages(), Population(400))
            .is_err()
    );
}

#[test]
fn country_cohort_and_scope_coverage_are_explicit_and_keep_attributed_openings_immutable() {
    let content = Content::demographic_bundled().unwrap();
    let (catalogue, scopes) = fixture();
    let definition = SocialMembershipDefinition {
        source: content.sources.last().unwrap().id.clone(),
        source_version: "fictional-joint-test-1".into(),
        description: "Explicit authored cross-sectional profiles; not national membership observations or individual identity histories.".into(),
        catalogue,
        profiles: content.countries.iter().map(|country| (country.id.clone(), country.households.iter().map(|group| (group.id.clone(), scopes.clone())).collect())).collect(),
    };
    definition.validate(&content).unwrap();
    let before = content.clone();
    for variant in 0..5 {
        let mut invalid = definition.clone();
        let country = &content.countries[0];
        let group = &country.households[0];
        match variant {
            0 => invalid.source = "missing-source".into(),
            1 => {
                invalid.profiles.remove(&country.id);
            }
            2 => {
                invalid
                    .profiles
                    .get_mut(&country.id)
                    .unwrap()
                    .remove(&group.id);
            }
            3 => {
                invalid
                    .profiles
                    .get_mut(&country.id)
                    .unwrap()
                    .get_mut(&group.id)
                    .unwrap()
                    .remove(&SocialPopulationScope::Young);
            }
            4 => {
                invalid
                    .profiles
                    .get_mut(&country.id)
                    .unwrap()
                    .get_mut(&group.id)
                    .unwrap()
                    .get_mut(&SocialPopulationScope::Older)
                    .unwrap()
                    .cells[0]
                    .share = BasisPoints::new(4999);
            }
            _ => unreachable!(),
        }
        assert!(invalid.validate(&content).is_err(), "variant {variant}");
    }
    let restored: SocialMembershipDefinition =
        serde_json::from_slice(&serde_json::to_vec(&definition).unwrap()).unwrap();
    restored.validate(&content).unwrap();
    assert_eq!(restored, definition);
    assert_eq!(content, before);
    let mut renamed = content.clone();
    renamed.countries.reverse();
    for country in &mut renamed.countries {
        country.name = "Renamed country display".into();
        country.households.reverse();
        for group in &mut country.households {
            group.name = "Renamed household display".into();
        }
    }
    definition.validate(&renamed).unwrap();
}
