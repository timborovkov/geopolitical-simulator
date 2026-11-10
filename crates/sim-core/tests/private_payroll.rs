use sim_core::*;
fn fixture() -> (Content, World) {
    let c = Content::from_json(include_str!(
        "../../../data/synthetic-private-payroll-test.json"
    ))
    .unwrap();
    let w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    (c, w)
}
#[test]
fn payroll_contracts_and_cash_reconcile_through_review_and_replay() {
    let (c, mut w) = fixture();
    w.validate(&c).unwrap();
    let parent = Content::from_json(include_str!(
        "../../../data/synthetic-private-investment-test.json"
    ))
    .unwrap();
    let prior = World::new(&parent, w.player.clone(), 17).unwrap();
    for (id, country) in &w.countries {
        assert_eq!(country.accounts, prior.countries[id].accounts);
        assert_eq!(country.population, prior.countries[id].population);
        for h in &country.households {
            assert!(h.employed.0 <= h.working_age.0);
        }
    }
    w.advance(&c, DaySpan::new(60)).unwrap();
    let s = w.player_country().private_payroll.as_ref().unwrap();
    assert!(s.contracts.iter().any(|g| g.jobs.0 > 0 && g.assessed.0 > 0));
    for g in &s.contracts {
        assert_eq!(g.assessed.0 - g.paid.0, g.unpaid.0);
    }
    let p = w
        .business_profit_report(&c, Viewer::Government(w.player.clone()), &w.player)
        .unwrap()
        .owned
        .unwrap()
        .cumulative;
    assert_eq!(p.payroll_obligations, Some(s.unpaid().unwrap()));
    assert_eq!(
        p.result().unwrap().0,
        p.cash_change.0 + p.distributions.0 + p.inventory_value.0 + p.receivables.0
            - p.deferred_capital.0
            + p.private_capital.unwrap().net_assets.0
            - s.unpaid().unwrap().0
    );
    assert_eq!(w, persistence::replay(&w, &c).unwrap());
    assert_eq!(
        w,
        persistence::decode(&persistence::encode(&w, &c).unwrap(), &c).unwrap()
    );
    assert_eq!(persistence::snapshot_schema(&w), 41);
}
#[test]
fn payroll_inspection_respects_owner_and_legacy_campaigns() {
    let (c, w) = fixture();
    assert!(
        w.private_payroll_report(&c, Viewer::Observer, &w.player)
            .unwrap()
            .is_none()
    );
    assert!(
        w.private_payroll_report(
            &c,
            Viewer::Government(w.player.clone()),
            &CountryId("BOR".into())
        )
        .unwrap()
        .is_none()
    );
    let old = Content::business_finance_bundled().unwrap();
    let old_world = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    assert_eq!(old_world.hash(), 0xb1a34e0e7596b9a9);
    assert!(old_world.player_country().private_payroll.is_none());
}

#[test]
fn invalid_payroll_contracts_and_receipts_refuse_commands_atomically() {
    let (c, mut w) = fixture();
    w.advance(&c, DaySpan::new(30)).unwrap();
    for n in 0..20 {
        let mut bad = w.clone();
        let player = bad.player.clone();
        let country = bad.countries.get_mut(&player).unwrap();
        let s = country.private_payroll.as_mut().unwrap();
        match n {
            0 => s.contracts[0].opening_jobs.0 += 1,
            1 => s.contracts[0].jobs.0 += 1,
            2 => s.contracts[0].full_daily_payroll.0 += 1,
            3 => s.contracts[0].assessed.0 += 1,
            4 => s.contracts[0].paid.0 += 1,
            5 => s.contracts[0].unpaid.0 += 1,
            6 => s.last.as_mut().unwrap().groups[0].current_paid.0 += 1,
            7 => s.last.as_mut().unwrap().groups[0].arrears_paid.0 += 1,
            8 => {
                country
                    .business
                    .as_mut()
                    .unwrap()
                    .last_decision
                    .as_mut()
                    .unwrap()
                    .earnings_protection
                    .as_mut()
                    .unwrap()
                    .payroll_creditors
                    .as_mut()
                    .unwrap()
                    .0 += 1
            }
            9 => {
                country
                    .private_investment
                    .as_mut()
                    .unwrap()
                    .last_review
                    .as_mut()
                    .unwrap()
                    .payroll_creditors
                    .as_mut()
                    .unwrap()
                    .0 += 1
            }
            10 => {
                country
                    .business_costs
                    .as_mut()
                    .unwrap()
                    .day_opening
                    .payroll_obligations
                    .as_mut()
                    .unwrap()
                    .0 += 1
            }
            11 => s.as_of = CampaignDay::new(0),
            12 => s.last.as_mut().unwrap().opening_cash.0 += 1,
            13 => s.last.as_mut().unwrap().groups[0].opening_unpaid.0 = -1,
            14 => s.last.as_mut().unwrap().groups[0].paid.0 = i64::MAX,
            15 => s.last.as_mut().unwrap().public_service_paid.0 = i64::MAX,
            16 => {
                country
                    .business
                    .as_mut()
                    .unwrap()
                    .last_decision
                    .as_mut()
                    .unwrap()
                    .earnings_protection
                    .as_mut()
                    .unwrap()
                    .payroll_creditors = Some(Money(i64::MAX))
            }
            17 => country.households[0].id = "unknown-group".into(),
            18 => {
                s.contracts.push(s.contracts[0].clone());
                country.households.push(country.households[0].clone());
            }
            _ => s.last.as_mut().unwrap().nonwage_paid.0 += 1,
        }
        let original = bad.clone();
        assert!(bad.validate(&c).is_err(), "variant {n}");
        assert!(bad.advance(&c, DaySpan::new(1)).is_err(), "variant {n}");
        assert_eq!(bad, original);
        assert!(
            bad.issue(
                &c,
                Action::Enact {
                    policy: "welfare".into(),
                    value: 6
                }
            )
            .is_err(),
            "variant {n}"
        );
        assert_eq!(bad, original);
    }
}
#[test]
fn unpaid_wages_survive_save_and_repays_without_duplicate_current_funding() {
    let (c, mut w) = fixture();
    for field in [DeveloperField::EnergyCapacity, DeveloperField::FoodCapacity] {
        w.issue_developer(&c, w.player.clone(), DeveloperEdit { field, value: 1 })
            .unwrap();
    }
    w.advance(&c, DaySpan::new(900)).unwrap();
    let s = w.player_country().private_payroll.as_ref().unwrap();
    assert!(
        s.unpaid().unwrap().0 > 0,
        "Supply collapse must eventually exhaust cash for contractual wages"
    );
    assert_eq!(
        w,
        persistence::decode(&persistence::encode(&w, &c).unwrap(), &c).unwrap()
    );
    for field in [DeveloperField::EnergyCapacity, DeveloperField::FoodCapacity] {
        w.issue_developer(&c, w.player.clone(), DeveloperEdit { field, value: 200 })
            .unwrap();
    }
    let mut observed_repayment = false;
    for _ in 0..72 {
        w.advance(&c, DaySpan::new(30)).unwrap();
        let s = w.player_country().private_payroll.as_ref().unwrap();
        let d = s.last.as_ref().unwrap();
        observed_repayment |= d.arrears_paid().unwrap().0 > 0;
        assert_eq!(
            d.current_factors_paid().unwrap().0,
            w.player_country().report.wages.0 - d.arrears_paid().unwrap().0
        );
        if s.unpaid().unwrap().0 == 0 {
            break;
        }
    }
    assert!(observed_repayment);
    assert_eq!(
        w.player_country()
            .private_payroll
            .as_ref()
            .unwrap()
            .unpaid()
            .unwrap(),
        Money(0)
    );
    assert_eq!(w, persistence::replay(&w, &c).unwrap());
}
