use sim_core::*;
fn fixture(funded: bool) -> Content {
    let mut json: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/synthetic-debt-maturities-test.json"
    ))
    .unwrap();
    json["finance"]["maturity_days"] = 30.into();
    if funded {
        for country in json["countries"].as_array_mut().unwrap() {
            country["tax"] = 45.into();
            country["services"] = 4.into();
            country["welfare"] = 0.into();
            country["capacity"] = 100.into();
            country["energy"] = 120.into();
            country["food"] = 120.into();
        }
    }
    Content::from_json(&json.to_string()).unwrap()
}
fn loan(world: &mut World, content: &Content, amount: i64) -> LoanId {
    let id = LoanId(world.next_command.0);
    world
        .issue(
            content,
            Action::Borrow {
                amount: Money(amount),
            },
        )
        .unwrap();
    id
}
fn instrument(world: &World, loan: LoanId) -> &DebtInstrument {
    world
        .player_country()
        .financing
        .as_ref()
        .unwrap()
        .instruments
        .iter()
        .find(|row| row.id == loan)
        .unwrap()
}
fn replay(world: &World, content: &Content) {
    world.validate(content).unwrap();
    assert_eq!(persistence::replay(world, content).unwrap(), *world);
}
#[test]
fn maturity_follows_actual_approval_not_request_and_reconciles_cash_principal_and_interest() {
    let content = fixture(true);
    let mut w = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let id = loan(&mut w, &content, 1000);
    assert_eq!(
        w.player_country()
            .financing
            .as_ref()
            .unwrap()
            .instruments
            .len(),
        1
    );
    w.advance(&content, sim_core::DaySpan::new(14)).unwrap();
    assert_eq!(
        instrument(&w, id).maturity_day,
        Some(sim_core::CampaignDay::new(44))
    );
    let outlook = w.funding_outlook(&content, 30, None).unwrap();
    assert_eq!(outlook.principal_due, Money(1000));
    assert_eq!(
        outlook.points[29].without_loan.0,
        outlook.available_cash.0 + outlook.daily_balance.0 * 29
    );
    assert_eq!(
        outlook.points[30].without_loan.0,
        outlook.available_cash.0 + outlook.daily_balance.0 * 30 - 1000
    );
    let hash = w.hash();
    let conditional = w
        .funding_outlook(
            &content,
            44,
            Some(LoanAssumption {
                amount: Money(2000),
                decision_day: sim_core::CampaignDay::new(28),
            }),
        )
        .unwrap();
    assert_eq!(conditional.principal_due, Money(1000));
    assert_eq!(conditional.conditional_principal_due, Money(3000));
    assert_eq!(
        conditional.points[43].if_approved.0 - conditional.points[43].without_loan.0,
        2000 - conditional.additional_daily_interest.0 * 30
    );
    assert_eq!(
        conditional.points[44].if_approved.0 - conditional.points[44].without_loan.0,
        -conditional.additional_daily_interest.0 * 31
    );
    assert_eq!(w.hash(), hash);
    w.advance(&content, sim_core::DaySpan::new(29)).unwrap();
    assert_eq!(instrument(&w, id).outstanding, Money(1000));
    let opening = w.player_country().debt;
    let interest = w.player_country().report.interest;
    let bytes = persistence::encode(&w, &content).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["schema"],
        16
    );
    let mut resumed = persistence::decode(&bytes, &content).unwrap();
    w.advance(&content, sim_core::DaySpan::new(1)).unwrap();
    resumed
        .advance(&content, sim_core::DaySpan::new(1))
        .unwrap();
    assert_eq!(w, resumed);
    assert_eq!(instrument(&w, id).outstanding, Money(0));
    assert_eq!(instrument(&w, id).maturity_repaid, Some(Money(1000)));
    assert_eq!(
        instrument(&w, id).last_maturity_payment_day,
        Some(sim_core::CampaignDay::new(44))
    );
    assert_eq!(w.player_country().debt.0, opening.0 - 1000);
    assert!(w.player_country().report.interest.0 <= interest.0);
    assert_eq!(w.player_country().budget.principal_repaid, Money(1000));
    assert_eq!(
        w.player_country().budget.expected_cash(),
        w.player_country().accounts.treasury.0
    );
    assert!(w.ledger.iter().any(|row| row.country == w.player
        && row.day == sim_core::CampaignDay::new(44)
        && row.from == Account::Treasury
        && row.to == Account::External
        && row.amount == Money(1000)));
    replay(&w, &content);
    let mut old: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    old["schema"] = 15.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&old).unwrap(), &content)
            .unwrap_err()
            .contains("format 16")
    );
}
#[test]
fn unfunded_maturity_stays_outstanding_retries_and_does_not_create_duplicate_operating_arrears() {
    let mut content = fixture(false);
    for policy in &mut content.policies {
        if ["energy", "agriculture"].contains(&policy.id.as_str()) {
            policy.cost_bps = BasisPoints::new(500);
        }
    }
    content.validate().unwrap();
    let mut w = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let id = loan(&mut w, &content, 10000);
    w.advance(&content, sim_core::DaySpan::new(43)).unwrap();
    for policy in ["energy", "agriculture"] {
        w.issue(
            &content,
            Action::Enact {
                policy: policy.into(),
                value: 1,
            },
        )
        .unwrap();
    }
    let reserved = w.player_country().committed_cash;
    assert!(reserved.0 > 0);
    let before = w.player_country().debt.0;
    w.advance(&content, sim_core::DaySpan::new(1)).unwrap();
    assert_eq!(w.player_country().committed_cash, reserved);
    assert!(w.player_country().accounts.treasury.0 >= reserved.0);
    let row = instrument(&w, id);
    let paid = row.maturity_repaid.unwrap().0;
    assert!(
        paid > 0 && paid < 10000,
        "expected partial cash-backed principal, got {paid}"
    );
    assert_eq!(row.outstanding.0, 10000 - paid);
    assert_eq!(w.player_country().debt.0, before - paid);
    assert_eq!(
        w.player_country()
            .financing
            .as_ref()
            .unwrap()
            .principal_due_through(w.day.units())
            .0,
        10000 - paid
    );
    let remaining = row.outstanding.0;
    w.advance(&content, sim_core::DaySpan::new(1)).unwrap();
    assert!(instrument(&w, id).outstanding.0 < remaining);
    assert_eq!(
        w.player_country().budget.expected_cash(),
        w.player_country().accounts.treasury.0
    );
    replay(&w, &content);
}
#[test]
fn mixed_voluntary_and_maturity_redemption_preserves_fifo_and_rejects_forged_history() {
    let content = fixture(true);
    let mut w = World::new(&content, CountryId("COR".into()), 17).unwrap();
    let first = loan(&mut w, &content, 1000);
    w.advance(&content, sim_core::DaySpan::new(10)).unwrap();
    let second = loan(&mut w, &content, 2000);
    w.issue(&content, Action::Repay { amount: Money(500) })
        .unwrap();
    w.advance(&content, sim_core::DaySpan::new(20)).unwrap();
    assert_eq!(instrument(&w, first).outstanding, Money(0));
    assert_eq!(instrument(&w, second).outstanding, Money(2000));
    w.issue(&content, Action::Repay { amount: Money(500) })
        .unwrap();
    w.advance(&content, sim_core::DaySpan::new(10)).unwrap();
    assert_eq!(instrument(&w, second).outstanding, Money(0));
    assert_eq!(
        w.player_country().financing.as_ref().unwrap().instruments[0].outstanding,
        Money(100000 / 3 - 1000)
    );
    replay(&w, &content);
    let mut tampered = w.clone();
    tampered
        .countries
        .get_mut(&w.player)
        .unwrap()
        .financing
        .as_mut()
        .unwrap()
        .instruments[1]
        .last_maturity_payment_day = Some(sim_core::CampaignDay::new(29));
    assert!(tampered.validate(&content).is_err());
    let mut tampered = w.clone();
    tampered
        .countries
        .get_mut(&w.player)
        .unwrap()
        .financing
        .as_mut()
        .unwrap()
        .instruments[1]
        .maturity_repaid = Some(Money(999));
    assert!(tampered.validate(&content).is_err());
}
#[test]
fn maturity_rules_are_versioned_bounded_and_leave_the_preserved_starter_unchanged() {
    let mut content = fixture(true);
    content.finance.as_mut().unwrap().maturity_days = Some(DaySpan::new(29));
    assert!(content.validate().is_err());
    let mut content = fixture(true);
    content.finance.as_mut().unwrap().maturity_days = None;
    assert!(content.validate().is_err());
    let mut content = Content::national_fiscal_bundled().unwrap();
    content.finance.as_mut().unwrap().maturity_days = Some(DaySpan::new(180));
    assert!(content.validate().is_err());
    let content = Content::national_fiscal_bundled().unwrap();
    let mut w = World::new(&content, CountryId("FRA".into()), 17).unwrap();
    w.advance(&content, sim_core::DaySpan::new(138)).unwrap();
    assert_eq!(w.hash(), 0x4be2fd07e49920b2);
    assert!(
        w.countries
            .values()
            .flat_map(|c| c.financing.iter().flat_map(|s| &s.instruments))
            .all(|loan| loan.maturity_day.is_none() && loan.maturity_repaid.is_none())
    );
    replay(&w, &content);
}
