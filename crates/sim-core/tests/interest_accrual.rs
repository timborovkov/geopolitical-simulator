use sim_core::*;

fn fixture(small: bool, funded: bool) -> Content {
    let mut json: serde_json::Value = serde_json::from_str(include_str!(
        "../../../data/synthetic-interest-accrual-test.json"
    ))
    .unwrap();
    if small || funded {
        for country in json["countries"].as_array_mut().unwrap() {
            if small {
                country["gdp"] = 5000.into();
            }
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
fn state(w: &World) -> &AccruedInterest {
    w.player_country()
        .financing
        .as_ref()
        .unwrap()
        .interest_accrual
        .as_ref()
        .unwrap()
}
fn replay(w: &World, c: &Content) {
    w.validate(c).unwrap();
    assert_eq!(*w, persistence::replay(w, c).unwrap());
}
#[test]
fn small_principal_follows_nominal_rate_and_carry_survives_principal_retirement() {
    assert_eq!(
        InterestProjection::new(Money(1000), 0)
            .unwrap()
            .due(365)
            .unwrap(),
        Money(40)
    );
    let c = fixture(true, true);
    let mut w = World::new(&c, CountryId("COR".into()), 17).unwrap();
    w.issue(&c, Action::Repay { amount: Money(250) }).unwrap();
    assert_eq!(w.player_country().debt, Money(1416));
    w.advance(&c, sim_core::DaySpan::new(365)).unwrap();
    assert_eq!(state(&w).principal_days, 1416 * 365);
    assert_eq!(state(&w).whole_assessed(), Money(56));
    assert_eq!(state(&w).paid, Money(56));
    assert_eq!(state(&w).remainder, 2_336_000); // 0.64m
    w.advance(&c, sim_core::DaySpan::new(1200)).unwrap();
    let paid = state(&w).paid;
    w.issue(
        &c,
        Action::Repay {
            amount: Money(1416),
        },
    )
    .unwrap();
    let retained = state(&w).remainder;
    w.advance(&c, sim_core::DaySpan::new(4)).unwrap();
    assert_eq!(state(&w).remainder, retained);
    assert_eq!(state(&w).last.as_ref().unwrap().amount_due, Money(0));
    assert_eq!(state(&w).paid, paid);
    replay(&w, &c);
}
#[test]
fn manual_and_delayed_loans_use_actual_assessment_timing_and_shared_fraction() {
    let c = fixture(false, true);
    for id in ["AUR", "COR"] {
        let mut w = World::new(&c, CountryId(id.into()), 17).unwrap();
        let due = w
            .preview_borrowing(&c, &w.player, Money(1000))
            .unwrap()
            .decision_day;
        let forecast = w
            .funding_outlook(
                &c,
                20,
                Some(LoanAssumption {
                    amount: Money(1000),
                    decision_day: due,
                }),
            )
            .unwrap();
        let before = w.hash();
        assert!(forecast.interest_accrual.is_some());
        assert_eq!(w.hash(), before);
        w.issue(
            &c,
            Action::Borrow {
                amount: Money(1000),
            },
        )
        .unwrap();
        let mut actually_due = 0;
        for day in 1_u32..=20 {
            w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
            actually_due += state(&w).last.as_ref().unwrap().amount_due.0;
            let expected_days = if due == sim_core::CampaignDay::new(0) {
                day
            } else {
                day.saturating_sub((due).units()) + u32::from(day >= (due).units())
            };
            assert_eq!(
                state(&w).principal_days,
                33333 * u64::from(day) + 1000 * u64::from(expected_days)
            );
            // Funded, unchanged player policies: outlook exactly predicts national cash.
            assert_eq!(
                forecast.points[day as usize].if_approved,
                w.player_country().available_cash()
            );
        }
        assert_eq!(
            actually_due,
            forecast.interest_accrual.unwrap().if_approved_due.0
        );
        w.issue(&c, Action::Repay { amount: Money(100) }).unwrap();
        replay(&w, &c);
    }
}
#[test]
fn fractional_interest_does_not_become_cash_and_unpaid_whole_units_reconcile() {
    let c = fixture(false, false);
    let mut w = World::new(&c, CountryId("COR".into()), 17).unwrap();
    // Retire as much principal as actual opening cash permits, then protect receipts.
    let available = w.player_country().available_cash();
    w.issue(&c, Action::Repay { amount: available }).unwrap();
    w.issue(
        &c,
        Action::HoldCash {
            target: Money(1000),
            days: sim_core::DaySpan::new(90),
        },
    )
    .unwrap();
    w.advance(&c, sim_core::DaySpan::new(5)).unwrap();
    assert!(w.player_country().retained_cash().0 > 0);
    assert!(w.player_country().arrears_external.0 > 0);
    assert_eq!(state(&w).paid, Money(0));
    assert_eq!(
        state(&w).whole_assessed(),
        w.player_country().arrears_external
    );
    let hold = w
        .player_country()
        .cash_management
        .as_ref()
        .unwrap()
        .active()
        .unwrap()
        .id;
    w.issue(&c, Action::ReleaseCash { request: hold }).unwrap();
    w.advance(&c, sim_core::DaySpan::new(1)).unwrap();
    assert!(state(&w).paid.0 > 0);
    assert_eq!(
        state(&w).whole_assessed().0,
        state(&w).paid.0 + w.player_country().arrears_external.0
    );
    assert_eq!(
        w.player_country().budget.expected_cash(),
        w.player_country().accounts.treasury.0
    );
    replay(&w, &c);
}
#[test]
fn snapshot_18_resumes_carry_and_rejects_forged_interest_or_forecast() {
    let c = fixture(false, true);
    let mut w = World::new(&c, CountryId("COR".into()), 17).unwrap();
    w.advance(&c, sim_core::DaySpan::new(23)).unwrap();
    let bytes = persistence::encode(&w, &c).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["schema"], 18);
    let mut resumed = persistence::decode(&bytes, &c).unwrap();
    w.advance(&c, sim_core::DaySpan::new(44)).unwrap();
    resumed.advance(&c, sim_core::DaySpan::new(44)).unwrap();
    assert_eq!(w, resumed);
    json["schema"] = 17.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&json).unwrap(), &c)
            .unwrap_err()
            .contains("format 18")
    );
    for field in ["remainder", "principal_days", "paid"] {
        let mut forged = w.clone();
        let s = forged
            .countries
            .get_mut(&w.player)
            .unwrap()
            .financing
            .as_mut()
            .unwrap()
            .interest_accrual
            .as_mut()
            .unwrap();
        match field {
            "remainder" => s.remainder += 1,
            "principal_days" => s.principal_days += 1,
            _ => s.paid.0 += 1,
        };
        assert!(forged.validate(&c).is_err(), "{field}");
    }
    let mut forged = w.clone();
    forged
        .countries
        .get_mut(&w.player)
        .unwrap()
        .budget
        .forecast
        .interest_accrual
        .as_mut()
        .unwrap()
        .opening_remainder = INTEREST_DENOMINATOR;
    assert!(forged.validate(&c).is_err());
    replay(&w, &c);
}
#[test]
fn opening_budget_uses_exact_interval_interest_including_leap_year() {
    let c = fixture(false, true);
    let w = World::new(&c, CountryId("COR".into()), 17).unwrap();
    let period = &w.player_country().budget;
    assert_eq!(period.end_day, sim_core::CampaignDay::new(366));
    let forecast = &period.forecast;
    let due = forecast
        .interest_accrual
        .as_ref()
        .unwrap()
        .due(365)
        .unwrap();
    assert_eq!(due, Money(1333));
    assert_eq!(
        period.forecast_closing_cash(),
        period.opening_cash.0 + 365 * (forecast.daily_revenue.0 - forecast.daily_spending.0)
            - due.0
    );
    let mut next = period.clone();
    next.opening_day_settled = Some(true);
    assert_eq!(
        next.forecast_closing_cash(),
        next.opening_cash.0 + 366 * (forecast.daily_revenue.0 - forecast.daily_spending.0)
            - forecast
                .interest_accrual
                .as_ref()
                .unwrap()
                .due(366)
                .unwrap()
                .0
    );
    assert!(InterestProjection::new(Money(-1), 0).is_err());
    assert!(InterestProjection::new(Money(1), INTEREST_DENOMINATOR).is_err());
    let mut old = Content::cash_bundled().unwrap();
    old.finance.as_mut().unwrap().interest_accrual = Some(InterestAccrualRule::Actual365Carry);
    assert!(old.validate().is_err());
}

#[test]
fn outlook_shares_existing_fraction_with_a_new_loan_and_times_the_first_cash_gap() {
    let mut c = fixture(true, true);
    for country in &mut c.countries {
        country.tax = Percent::new(10);
    }
    c.validate().unwrap();
    let mut w = World::new(&c, CountryId("COR".into()), 17).unwrap();
    let cash = w.player_country().available_cash();
    w.issue(&c, Action::Repay { amount: cash }).unwrap();
    assert_eq!(w.player_country().debt, Money(1416));
    w.advance(&c, sim_core::DaySpan::new(3)).unwrap();
    assert_eq!(state(&w).remainder, 1_699_200);
    let forecast = w
        .funding_outlook(
            &c,
            10,
            Some(LoanAssumption {
                amount: Money(300),
                decision_day: w.day,
            }),
        )
        .unwrap();
    assert_eq!(forecast.covered_days, Some(3));
    assert_eq!(forecast.points[3].without_loan, Money(0));
    assert_eq!(forecast.points[3].if_approved, Money(299));
    w.issue(&c, Action::Borrow { amount: Money(300) }).unwrap();
    w.advance(&c, sim_core::DaySpan::new(3)).unwrap();
    assert_eq!(state(&w).whole_assessed(), Money(1));
    assert_eq!(
        forecast.points[3].if_approved,
        w.player_country().available_cash()
    );
    replay(&w, &c);
}
