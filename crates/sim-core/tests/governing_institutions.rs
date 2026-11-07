use sim_core::*;
fn fixture() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-governing-institutions-test.json"
    ))
    .unwrap()
}
fn view(c: &Content, w: &World) -> GoverningInstitutionsView {
    w.governing_institutions_view(c, Viewer::Government(w.player.clone()), &w.player)
        .unwrap()
        .unwrap()
}
fn check(c: &Content, w: &World) {
    w.validate(c).unwrap();
    assert_eq!(
        persistence::decode(&persistence::encode(w, c).unwrap(), c).unwrap(),
        *w
    );
    assert_eq!(persistence::replay(w, c).unwrap(), *w);
}
fn fast_transitions(c: &mut Content) {
    let rules = c.constitutions.as_mut().unwrap();
    rules.cooldown_days = DaySpan::new(30);
    for route in [&mut rules.lawful, rules.contested.as_mut().unwrap()] {
        route.days = DaySpan::new(1);
        route.cost_bps = BasisPoints::new(1);
        route.upfront_bps = BasisPoints::new(2500);
        route.legislative_threshold = 0;
        route.elite_threshold = 0;
        route.council_threshold = 0;
        route.approval_threshold = 0;
        route.approval_cost = 0;
        route.legislative_cost = 0;
        route.elite_cost = 0;
        route.council_cost = 0;
    }
}
#[test]
fn authored_institutions_are_complete_stable_and_independent_of_labels_and_order() {
    let mut c = Content::governing_institutions_bundled().unwrap();
    assert_eq!(
        c.countries,
        Content::executive_identities_bundled().unwrap().countries
    );
    assert_eq!(
        c.governing_institutions
            .as_ref()
            .unwrap()
            .institutions
            .len(),
        105
    );
    assert_eq!(Content::bundled().unwrap().hash(), c.hash());
    assert_eq!(Content::bundled_for_hash(c.hash()).unwrap(), c);
    let w = World::new(&c, CountryId("GBR".into()), 17).unwrap();
    let original = view(&c, &w).context;
    let d = c.governing_institutions.as_mut().unwrap();
    d.institutions.reverse();
    d.bindings.reverse();
    for i in &mut d.institutions {
        i.name = "Renamed modeled body".into();
    }
    c.validate().unwrap();
    let w = World::new(&c, CountryId("GBR".into()), 17).unwrap();
    assert_eq!(view(&c, &w).context, original);
    for mutate in 0..5 {
        let mut bad = c.clone();
        let d = bad.governing_institutions.as_mut().unwrap();
        match mutate {
            0 => {
                d.institutions[0].id = d.institutions[1].id.clone();
            }
            1 => {
                d.bindings[0].primary = d.bindings[6].primary.clone();
            }
            2 => {
                d.institutions[0].quality = "historical".into();
            }
            3 => {
                d.bindings.pop();
            }
            _ => {
                d.institutions[0].source = "unknown".into();
            }
        }
        assert!(bad.validate().is_err());
    }
}
#[test]
fn current_views_and_command_lookup_respect_owner_boundary_without_mutation() {
    let c = fixture();
    for country in &c.countries {
        let mut w = World::new(&c, country.id.clone(), 17).unwrap();
        w.issue(&c, Action::Repay { amount: Money(1) }).unwrap();
        let before = w.clone();
        let v = view(&c, &w);
        assert_eq!(v.context.route, country.government.route);
        assert_eq!(v.context.government, GovernmentId(1));
        assert_eq!(
            w.command_governing_context(&c, Viewer::Government(w.player.clone()), CommandId(1))
                .unwrap(),
            Some(v.context)
        );
        assert!(
            w.governing_institutions_view(&c, Viewer::Observer, &w.player)
                .unwrap()
                .is_none()
        );
        assert!(
            w.command_governing_context(&c, Viewer::Observer, CommandId(1))
                .unwrap()
                .is_none()
        );
        let foreign = c
            .countries
            .iter()
            .find(|d| d.id != w.player)
            .unwrap()
            .id
            .clone();
        assert!(
            w.command_governing_context(&c, Viewer::Government(foreign.clone()), CommandId(1))
                .unwrap()
                .is_none()
        );
        assert!(
            w.governing_institutions_view(&c, Viewer::Government(w.player.clone()), &foreign)
                .unwrap()
                .is_none()
        );
        for id in [0, u64::MAX, 2] {
            assert!(
                w.command_governing_context(
                    &c,
                    Viewer::Government(w.player.clone()),
                    CommandId(id)
                )
                .unwrap()
                .is_none()
            );
        }
        assert_eq!(before, w);
        assert_eq!(persistence::snapshot_schema(&w), 29);
        check(&c, &w);
    }
}
#[test]
fn all_regimes_change_active_routes_without_replacing_institution_identities() {
    let mut c = fixture();
    fast_transitions(&mut c);
    c.validate().unwrap();
    let institutions = c.governing_institutions.clone();
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    for target in [
        Regime::Presidential,
        Regime::AbsoluteMonarchy,
        Regime::SingleParty,
        Regime::Military,
        Regime::Theocracy,
        Regime::Personalist,
        Regime::SemiPresidential,
        Regime::Parliamentary,
    ] {
        let before = view(&c, &w).context;
        let command = w.next_command;
        w.issue(
            &c,
            Action::Reconstitute {
                target: target.clone(),
                contested: true,
            },
        )
        .unwrap();
        w.advance(&c, DaySpan::new(30)).unwrap();
        assert_eq!(w.governing_regime(&c, &w.player).unwrap(), &target);
        assert_eq!(
            w.command_governing_context(&c, Viewer::Government(w.player.clone()), command)
                .unwrap(),
            Some(before)
        );
        let current = view(&c, &w);
        assert_eq!(
            current.context.route,
            c.constitutions
                .as_ref()
                .unwrap()
                .template(&target)
                .unwrap()
                .government
                .route
        );
        assert_eq!(c.governing_institutions, institutions);
        w.issue(&c, Action::Repay { amount: Money(1) }).unwrap();
        w.validate(&c).unwrap();
    }
    check(&c, &w);
}
#[test]
fn malformed_command_context_and_earlier_pack_injection_are_rejected() {
    let c = fixture();
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    w.issue(&c, Action::Repay { amount: Money(1) }).unwrap();
    for mutate in 0..5 {
        let mut bad = w.clone();
        let context = bad.journal[0].governing_context.as_mut().unwrap();
        match mutate {
            0 => context.government = GovernmentId(2),
            1 => context.primary = InstitutionId("institution:BOR:legislature".into()),
            2 => {
                context.council = Some(InstitutionId("institution:AUR:consultative_council".into()))
            }
            3 => context.route = DecisionRoute::ExecutiveDecree,
            _ => bad.journal[0].governing_context = None,
        }
        assert!(bad.validate(&c).is_err());
    }
    let old = Content::executive_identities_bundled().unwrap();
    let mut legacy = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    legacy
        .issue(&old, Action::Repay { amount: Money(1) })
        .unwrap();
    assert!(legacy.journal[0].governing_context.is_none());
    legacy.journal[0].governing_context = w.journal[0].governing_context.clone();
    assert!(legacy.validate(&old).is_err());
    check(&c, &w);
}
#[test]
fn new_context_metadata_preserves_actual_commands_economy_and_ai_over_800_days() {
    let old = Content::executive_identities_bundled().unwrap();
    let new = Content::governing_institutions_bundled().unwrap();
    let mut a = World::new_scenario(
        &old,
        CountryId("GBR".into()),
        17,
        Scenario::FiscalSupplyCrisis,
    )
    .unwrap();
    let mut b = World::new_scenario(
        &new,
        CountryId("GBR".into()),
        17,
        Scenario::FiscalSupplyCrisis,
    )
    .unwrap();
    a.advance(&old, DaySpan::new(800)).unwrap();
    b.advance(&new, DaySpan::new(800)).unwrap();
    b.validate(&new).unwrap();
    assert!(b.journal.iter().any(|e| e.ai));
    let mut normalized = b.clone();
    normalized.rules = a.rules;
    normalized.content_hash = a.content_hash;
    for e in &mut normalized.journal {
        e.governing_context = None;
    }
    assert_eq!(normalized, a);
    check(&new, &b);
}

#[test]
fn budget_dispatch_keeps_its_submission_mandate_before_same_day_leadership_replacement() {
    let mut c = fixture();
    let bor = c.countries.iter_mut().find(|c| c.id.0 == "BOR").unwrap();
    bor.government.leadership_review_days = Some(DaySpan::new(30));
    bor.government.threshold = 1;
    bor.government.council_threshold = 0;
    c.validate().unwrap();
    let mut w = World::new(&c, CountryId("BOR".into()), 17).unwrap();
    w.advance(&c, DaySpan::new(9)).unwrap();
    let mut targets = FiscalTargets::current(w.player_country());
    targets.income_tax += 1;
    w.issue(
        &c,
        Action::ProposeBudget {
            year: 2000,
            targets,
        },
    )
    .unwrap();
    assert_eq!(
        w.player_country().fiscal_plans.as_ref().unwrap().bills[0].review_day,
        sim_core::CampaignDay::new(30)
    );
    w.advance(&c, DaySpan::new(20)).unwrap();
    // Explicit host edit supplies a deterministic loss of council backing. It is
    // replayed as a developer bypass, never attributed to a governing institution.
    w.issue_developer(
        &c,
        w.player.clone(),
        DeveloperEdit {
            field: DeveloperField::CouncilSupport,
            value: 0,
        },
    )
    .unwrap();
    let developer = w.journal.last().unwrap().clone();
    assert!(developer.governing_context.is_none());
    w.advance(&c, DaySpan::new(1)).unwrap();
    let dispatch = w
        .journal
        .iter()
        .find(|e| e.command.issuer == w.player && e.budget_bill.is_some())
        .unwrap()
        .clone();
    assert_eq!(dispatch.command.day, CampaignDay::new(30));
    assert_eq!(
        dispatch.governing_context.as_ref().unwrap().government,
        GovernmentId(1)
    );
    assert_eq!(w.player_country().government_number, GovernmentId(2));
    w.issue(&c, Action::Repay { amount: Money(1) }).unwrap();
    let latest = w.journal.last().unwrap();
    assert_eq!(latest.command.day, dispatch.command.day);
    assert_eq!(
        latest.governing_context.as_ref().unwrap().government,
        GovernmentId(2)
    );
    let mut forged = w.clone();
    forged
        .countries
        .get_mut(&w.player)
        .unwrap()
        .fiscal_plans
        .as_mut()
        .unwrap()
        .bills[0]
        .sponsor = GovernmentId(2);
    forged.journal[dispatch.command.id.0 as usize - 1]
        .governing_context
        .as_mut()
        .unwrap()
        .government = GovernmentId(2);
    assert!(
        forged
            .validate(&c)
            .unwrap_err()
            .contains("proposal context")
    );
    let mut invalid = w.clone();
    invalid.journal[developer.command.id.0 as usize - 1].governing_context =
        latest.governing_context.clone();
    assert!(invalid.validate(&c).is_err());
    check(&c, &w);
}

#[test]
fn institution_context_requires_its_own_content_and_snapshot_format() {
    let c = fixture();
    let w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    let mut raw: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&w, &c).unwrap()).unwrap();
    raw["schema"] = serde_json::json!(28);
    assert!(
        persistence::decode(&serde_json::to_vec(&raw).unwrap(), &c)
            .unwrap_err()
            .contains("format 29")
    );
    let mut missing = c.clone();
    missing.governing_institutions = None;
    assert!(missing.validate().is_err());
    let mut wrong = c.clone();
    wrong.schema = 27;
    wrong.rules = 27;
    assert!(wrong.validate().is_err());
    let mut altered = c.clone();
    altered.name.push_str(" changed");
    assert!(
        w.governing_institutions_view(&altered, Viewer::Government(w.player.clone()), &w.player)
            .is_err()
    );
    let old = Content::executive_identities_bundled().unwrap();
    let oldworld = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    assert!(
        oldworld
            .governing_institutions_view(
                &old,
                Viewer::Government(oldworld.player.clone()),
                &oldworld.player
            )
            .unwrap()
            .is_none()
    );
}
