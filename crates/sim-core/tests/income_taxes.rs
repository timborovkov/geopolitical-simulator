use sim_core::*;
fn content() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-income-bands-test.json"
    ))
    .unwrap()
}
fn setup() -> (Content, World) {
    let mut c = content();
    for d in &mut c.countries {
        d.energy = 120;
    }
    c.validate().unwrap();
    let w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    (c, w)
}
fn check(c: &Content, w: &World) {
    w.validate(c).unwrap();
    let b = persistence::encode(w, c).unwrap();
    assert_eq!(persistence::decode(&b, c).unwrap(), *w);
    assert_eq!(persistence::replay(w, c).unwrap(), *w);
}
#[test]
fn authored_bands_apply_marginal_slices_not_the_top_rate_to_all_income() {
    let c = content();
    let taxes = c.income_taxes.as_ref().unwrap();
    let mut d = c.countries[0].clone();
    d.gdp = 365000;
    d.population = Population(10000);
    let a = taxes
        .assess(&d, Population(1000), Money(100), 20, 80)
        .unwrap();
    assert_eq!(a.annualised_reference_bps, 10000);
    assert_eq!(a.potential, Money(20));
    assert_eq!(a.collectible, Money(16));
    assert_eq!(a.slices.iter().map(|s| s.annual_base.0).sum::<i64>(), 36500);
    assert_eq!(
        a.slices
            .iter()
            .map(|s| s.marginal_rate_bps.units())
            .collect::<Vec<_>>(),
        vec![0, 2000, 3000]
    );
    let a = taxes
        .assess(&d, Population(1000), Money(200), 20, 80)
        .unwrap();
    assert_eq!(a.potential, Money(55));
    assert_eq!(a.collectible, Money(44));
    assert_eq!(a.slices.last().unwrap().marginal_rate_bps.units(), 4000);
    assert_eq!(
        taxes
            .assess(&d, Population(1000), Money(10), 20, 80)
            .unwrap()
            .potential,
        Money(0)
    );
    assert!(
        taxes
            .assess(&d, Population(1000), Money(0), 20, 80)
            .unwrap()
            .slices
            .is_empty()
    );
}
#[test]
fn collection_is_bounded_monotone_and_has_no_tax_cliff_even_at_large_inputs() {
    let c = content();
    let t = c.income_taxes.as_ref().unwrap();
    let mut d = c.countries[0].clone();
    d.gdp = 365000;
    d.population = Population(10000);
    for rate in [10, 28, 45] {
        for admin in [1, 35, 70, 100] {
            let mut last_tax = 0;
            let mut last_net = 0;
            for gross in 0..=1024 {
                let a = t
                    .assess(&d, Population(1000), Money(gross), rate, admin)
                    .unwrap();
                assert!((0..=a.potential.0).contains(&a.collectible.0));
                assert!(a.potential.0 <= gross);
                assert!(a.collectible.0 >= last_tax);
                assert!(gross - a.collectible.0 >= last_net);
                last_tax = a.collectible.0;
                last_net = gross - a.collectible.0;
            }
        }
    }
    d.gdp = 100000000;
    d.population = Population(2000000000);
    let a = t
        .assess(&d, Population(200000000), Money(1000000000000), 45, 100)
        .unwrap();
    assert!(a.potential.0 <= a.gross.0);
    assert_eq!(a.potential, a.collectible);
    assert_eq!(
        a.slices.iter().map(|s| s.annual_base.0).sum::<i64>(),
        365000000000000
    );
    assert!(t.assess(&d, Population(0), Money(1), 24, 70).is_err());
    assert!(t.assess(&d, Population(1), Money(-1), 24, 70).is_err());
    assert!(t.assess(&d, Population(1), Money(1), 46, 70).is_err());
    assert!(t.assess(&d, Population(1), Money(1), 24, 0).is_err());
}
#[test]
fn actual_private_income_withholding_reconciles_and_public_payments_are_outside_the_base() {
    let (c, mut w) = setup();
    let before = w.player_country().accounts.total();
    let forecast = w
        .player_country()
        .forecast(c.country(&w.player).unwrap(), c.income_taxes.as_ref());
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    let country = w.player_country();
    let r = country.income_tax_report.as_ref().unwrap();
    assert_eq!(r.standard_rate, 24);
    assert_eq!(r.administration, 70);
    assert_eq!(r.paid(), country.report.revenue);
    assert_eq!(forecast.daily_revenue, country.report.revenue);
    assert!(r.potential().0 > r.paid().0);
    assert_eq!(
        r.groups.iter().map(|g| g.assessment.gross.0).sum::<i64>(),
        country.report.wages.0
    );
    assert_eq!(country.accounts.total(), before);
    let public = r.groups.iter().find(|g| g.group == "public").unwrap();
    let h = country
        .households
        .iter()
        .find(|h| h.id == "public")
        .unwrap();
    assert!(h.income.0 > public.assessment.gross.0);
    let tax_ledger = w
        .ledger
        .iter()
        .filter(|e| {
            e.country == w.player && e.from == Account::Households && e.to == Account::Treasury
        })
        .map(|e| e.amount.0)
        .sum::<i64>();
    assert_eq!(tax_ledger, r.paid().0);
    assert_eq!(
        country.households.iter().map(|h| h.tax.0).sum::<i64>(),
        r.paid().0
    );
    check(&c, &w);
}
#[test]
fn policy_delivery_changes_the_standard_rate_and_forecasts_share_the_collection_model() {
    let (c, mut w) = setup();
    let p = w.preview(&c, &w.player, &"income_tax".into(), 30).unwrap();
    let base = w
        .player_country()
        .forecast(c.country(&w.player).unwrap(), c.income_taxes.as_ref());
    let mut changed = w.player_country().clone();
    changed.tax = 30;
    let after = changed.forecast(c.country(&w.player).unwrap(), c.income_taxes.as_ref());
    assert_eq!(
        p.annual_budget_change.0,
        (base.daily_revenue.0 - after.daily_revenue.0) * 365
    );
    assert_eq!(
        p.household_impacts
            .iter()
            .map(|h| h.daily_disposable_change.0)
            .sum::<i64>()
            * 365,
        p.annual_budget_change.0
    );
    w.issue(
        &c,
        Action::Enact {
            policy: "income_tax".into(),
            value: 30,
        },
    )
    .unwrap();
    check(&c, &w);
    w.advance(&c, sim_core::DaySpan::new(p.delay.units() - 1))
        .unwrap();
    assert_eq!(
        w.player_country()
            .income_tax_report
            .as_ref()
            .unwrap()
            .standard_rate,
        24
    );
    check(&c, &w);
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    assert_eq!(
        w.player_country()
            .income_tax_report
            .as_ref()
            .unwrap()
            .standard_rate,
        30
    );
    assert_eq!(w.player_country().report.revenue, after.daily_revenue);
    check(&c, &w);
}
#[test]
fn same_day_developer_edits_do_not_rewrite_the_settled_rate_or_collection_capacity() {
    let (c, mut w) = setup();
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    let report = w.player_country().income_tax_report.clone();
    let player = w.player.clone();
    for (field, value) in [
        (DeveloperField::Tax, 45),
        (DeveloperField::Administration, 100),
    ] {
        w.issue_developer(&c, player.clone(), DeveloperEdit { field, value })
            .unwrap();
    }
    assert_eq!(w.player_country().income_tax_report, report);
    check(&c, &w);
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    let report = w.player_country().income_tax_report.as_ref().unwrap();
    assert_eq!(report.standard_rate, 45);
    assert_eq!(report.administration, 100);
    assert_eq!(report.potential(), report.paid());
    assert!(w.is_developer_modified());
    check(&c, &w);
}
#[test]
fn funded_collection_capacity_improves_withholding_only_when_it_operates() {
    let (c, mut w) = setup();
    let p = w
        .preview(&c, &w.player, &"administration".into(), 1)
        .unwrap();
    let d = c.country(&w.player).unwrap();
    let before = w.player_country().forecast(d, c.income_taxes.as_ref());
    let mut planned = w.player_country().clone();
    planned.administration = (planned.administration + 8).min(100);
    let after = planned.forecast(d, c.income_taxes.as_ref());
    let extra_collection = after.daily_revenue.0 - before.daily_revenue.0;
    assert!(extra_collection > 0);
    assert_eq!(p.annual_budget_change, Money(-365 * extra_collection));
    assert_eq!(
        p.household_impacts
            .iter()
            .map(|h| h.daily_disposable_change.0)
            .sum::<i64>(),
        -extra_collection
    );
    w.issue(
        &c,
        Action::Enact {
            policy: "administration".into(),
            value: 1,
        },
    )
    .unwrap();
    w.advance(&c, sim_core::DaySpan::new(p.delay.units() - 1))
        .unwrap();
    assert_eq!(
        w.player_country()
            .income_tax_report
            .as_ref()
            .unwrap()
            .administration,
        70
    );
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    assert_eq!(
        w.player_country()
            .income_tax_report
            .as_ref()
            .unwrap()
            .administration,
        78
    );
    check(&c, &w);
}
#[test]
fn a_budget_delivers_its_tax_rate_and_fulfils_a_nominal_standard_rate_undertaking() {
    let (mut c, _) = setup();
    let t = &mut c.undertakings.as_mut().unwrap().templates[0];
    t.policy = Some("income_tax".into());
    t.direction = GoalDirection::AtMost;
    t.delta = -2;
    c.validate().unwrap();
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    w.issue(
        &c,
        Action::DeclareUndertaking {
            template: "energy-pledge".into(),
            project: None,
        },
    )
    .unwrap();
    let mut targets = FiscalTargets::current(w.player_country());
    targets.income_tax = 22;
    let preview = w.preview_budget(&c, &w.player, 2000, &targets).unwrap();
    w.issue(
        &c,
        Action::ProposeBudget {
            year: 2000,
            targets,
        },
    )
    .unwrap();
    w.advance(&c, sim_core::DaySpan::new(preview.review_day))
        .unwrap();
    assert_eq!(
        w.player_country()
            .income_tax_report
            .as_ref()
            .unwrap()
            .standard_rate,
        24
    );
    let bill = &w.player_country().fiscal_plans.as_ref().unwrap().bills[0];
    assert_eq!(bill.status, BudgetBillStatus::Enacted);
    let source = bill.programmes[0].project;
    let due = w
        .player_country()
        .projects
        .iter()
        .find(|p| p.id == source)
        .unwrap()
        .due_day;
    w.advance(&c, sim_core::DaySpan::new(due - w.day.units()))
        .unwrap();
    assert_eq!(
        w.player_country()
            .income_tax_report
            .as_ref()
            .unwrap()
            .standard_rate,
        22
    );
    assert_eq!(
        w.player_country().undertakings.as_ref().unwrap().records[0].status,
        UndertakingStatus::Fulfilled { evidence: source }
    );
    check(&c, &w);
}
#[test]
fn falling_received_income_reduces_withholding_and_the_average_burden_without_changing_policy() {
    let c = content();
    let t = c.income_taxes.as_ref().unwrap();
    let mut d = c.countries[0].clone();
    d.gdp = 365000;
    d.population = Population(10000);
    let high = t.assess(&d, Population(1000), Money(100), 20, 80).unwrap();
    let low = t.assess(&d, Population(1000), Money(50), 20, 80).unwrap();
    assert!(low.collectible.0 < high.collectible.0);
    assert!(low.collectible.0 * high.gross.0 < high.collectible.0 * low.gross.0);
}
#[test]
fn malformed_bands_reports_and_older_envelopes_reject_cleanly() {
    let (c, mut w) = setup();
    let mut bad = c.clone();
    bad.income_taxes.as_mut().unwrap().bands[1].lower_gdp_per_resident_bps = BasisPoints::new(0);
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.income_taxes.as_mut().unwrap().bands[3].rate_multiplier_bps = BasisPoints::new(100000);
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.rules = 12;
    bad.schema = 12;
    assert!(bad.validate().is_err());
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    for field in ["date", "rate", "capacity", "slice", "base", "cash", "group"] {
        let mut bad = w.clone();
        let r = bad
            .countries
            .get_mut(&w.player.clone())
            .unwrap()
            .income_tax_report
            .as_mut()
            .unwrap();
        match field {
            "date" => r.day += 1,
            "rate" => r.standard_rate += 1,
            "capacity" => r.administration += 1,
            "slice" => {
                let rate = &mut r.groups[0].assessment.slices[0].marginal_rate_bps;
                *rate = rate.checked_add(BasisPoints::new(1)).unwrap();
            }
            "base" => r.groups[0].assessment.gross.0 += 1,
            "cash" => r.groups[0].paid.0 += 1,
            "group" => r.groups[0].group = "unknown".into(),
            _ => unreachable!(),
        };
        assert!(bad.validate(&c).is_err(), "{field}");
    }
    let mut encoded: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&w, &c).unwrap()).unwrap();
    assert_eq!(encoded["schema"], 14);
    encoded["schema"] = 13.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&encoded).unwrap(), &c)
            .unwrap_err()
            .contains("format 14")
    );
    check(&c, &w);
}
#[test]
fn progressive_collection_does_not_bypass_governing_authority() {
    let c = content();
    for d in &c.countries {
        let mut w = World::new(&c, d.id.clone(), 17).unwrap();
        for field in [
            DeveloperField::LegislativeSupport,
            DeveloperField::EliteSupport,
            DeveloperField::CouncilSupport,
        ] {
            w.issue_developer(&c, d.id.clone(), DeveloperEdit { field, value: 0 })
                .unwrap();
        }
        let hash = w.hash();
        assert!(
            w.issue(
                &c,
                Action::Enact {
                    policy: "income_tax".into(),
                    value: 30
                }
            )
            .is_err()
        );
        assert_eq!(w.hash(), hash);
        let mut t = FiscalTargets::current(w.player_country());
        t.income_tax = 30;
        assert!(
            w.issue(
                &c,
                Action::ProposeBudget {
                    year: 2000,
                    targets: t
                }
            )
            .is_err()
        );
        assert_eq!(w.hash(), hash);
        check(&c, &w);
    }
}
#[test]
fn ai_collection_and_budgets_replay_across_a_fiscal_year() {
    let (c, mut w) = setup();
    w.advance(&c, sim_core::DaySpan::new(720)).unwrap();
    assert!(
        w.journal
            .iter()
            .any(|e| e.ai && matches!(e.command.action, Action::ProposeBudget { .. }))
    );
    assert!(w.countries.iter().any(|(id, country)| id != &w.player
        && country.income_tax_report.as_ref().unwrap().standard_rate != 24));
    assert_eq!(w.player_country().budget_reviews.len(), 1);
    check(&c, &w);
}
#[test]
fn local_claims_are_based_on_withheld_cash_and_private_workbooks_are_not_published() {
    let (c, mut w) = setup();
    assert!(w.player_country().income_tax_report.is_none());
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    let country = w.player_country();
    let report = country.income_tax_report.as_ref().unwrap();
    let local = country
        .subnational
        .as_ref()
        .unwrap()
        .regions
        .iter()
        .map(|r| r.last.as_ref().unwrap().tax_receipts.0)
        .sum::<i64>();
    let entitled: i64 = country
        .subnational
        .as_ref()
        .unwrap()
        .regions
        .iter()
        .map(|r| r.tax_entitlement_bps.units_i64())
        .sum();
    assert_eq!(local, report.paid().0 * entitled / 10000);
    assert_ne!(local, report.potential().0 * entitled / 10000);
    let observer = w.project(&c, Viewer::Observer).unwrap();
    assert!(observer.own.is_none());
    let public = serde_json::to_string(&observer).unwrap();
    assert!(!public.contains("income_tax_report"));
    assert!(!public.contains("annualised_reference_bps"));
    let own = w.project(&c, Viewer::Government(w.player.clone())).unwrap();
    assert!(own.own.unwrap().income_tax_report.is_some());
    check(&c, &w);
}
#[test]
fn a_consolidated_surplus_can_still_leave_the_national_budget_short_of_cash() {
    let (c, w) = setup();
    let candidate = (10..=45)
        .find_map(|rate| {
            let mut t = FiscalTargets::current(w.player_country());
            t.income_tax = rate;
            let p = w.preview_budget(&c, &w.player, 2000, &t).ok()?;
            let consolidated = p.planned_forecast.daily_revenue.0
                - p.planned_forecast.daily_spending.0
                - p.planned_forecast.daily_interest.0;
            (consolidated > 0 && p.planned_national_balance.0 < 0).then_some((rate, p))
        })
        .expect("authored local claims can exceed a small consolidated surplus");
    let mut planned = w.clone();
    planned.countries.get_mut(&w.player).unwrap().tax = candidate.0;
    assert_eq!(
        planned.funding_outlook(&c, 0, None).unwrap().daily_balance,
        candidate.1.planned_national_balance
    );
    assert_eq!(
        w.funding_outlook(&c, 0, None).unwrap().daily_balance,
        candidate.1.baseline_national_balance
    );
}
