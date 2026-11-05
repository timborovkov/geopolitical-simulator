use sim_core::*;

#[test]
fn physical_arithmetic_rejects_overflow_and_preserves_integer_rounding() {
    let maximum = Energy::new(i64::MAX).unwrap();
    assert!(maximum.checked_add(Energy::new(1).unwrap()).is_none());
    assert!(maximum.scaled(2, 1).is_none());
    assert_eq!(maximum.scaled(2, 2), Some(maximum));
    assert_eq!(Food::new(101).unwrap().scaled(1, 100).unwrap().units(), 1);
    assert!(Food::new(1).unwrap().scaled(1, 0).is_none());
    assert!(Food::new(1).unwrap().scaled(-1, 1).is_none());
    assert!(Food::ZERO.checked_sub(Food::new(1).unwrap()).is_none());
    assert_eq!(Food::ZERO.saturating_sub(Food::new(1).unwrap()), Food::ZERO);
    assert_eq!(Food::new(9).unwrap().coverage(Food::new(10).unwrap()), 90);
    assert_eq!(maximum.coverage(Energy::new(1).unwrap()), 100);
    assert_eq!(Food::ZERO.coverage(Food::ZERO), 100);
}

#[test]
fn quantities_keep_legacy_json_scalars_and_reject_invalid_values() {
    assert_eq!(
        serde_json::to_string(&Food::new(42).unwrap()).unwrap(),
        "42"
    );
    assert_eq!(serde_json::from_str::<Energy>("42").unwrap().units(), 42);
    for malformed in ["-1", "1.5", "9223372036854775808", "null", "{\"units\":42}"] {
        assert!(serde_json::from_str::<Food>(malformed).is_err());
        assert!(serde_json::from_str::<Energy>(malformed).is_err());
    }
}

#[test]
fn outside_campaign_bounds_is_rejected_even_with_a_valid_quantity_type() {
    let content = Content::bundled().unwrap();
    let mut world = World::new(&content, CountryId("GBR".into()), 2000).unwrap();
    world.advance(&content, sim_core::DaySpan::new(30)).unwrap();
    let mut invalid = world.clone();
    invalid
        .countries
        .get_mut(&world.player)
        .unwrap()
        .report
        .food_imported = Food::new(i64::MAX).unwrap();
    assert!(invalid.validate(&content).is_err());
    assert!(persistence::encode(&invalid, &content).is_err());
    let bytes = persistence::encode(&world, &content).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["world"]["countries"]["GBR"]["food_stock"].is_i64());
    assert_eq!(
        persistence::decode(&bytes, &content).unwrap().hash(),
        world.hash()
    );
    persistence::replay(&world, &content).unwrap();
}

#[test]
fn rate_conversion_and_signed_scaling_are_explicit_and_checked() {
    assert_eq!(
        Percent::new(4).as_basis_points(),
        Some(BasisPoints::new(400))
    );
    assert_eq!(
        BasisPoints::new(400).as_exact_percent(),
        Some(Percent::new(4))
    );
    assert_eq!(BasisPoints::new(450).as_exact_percent(), None);
    assert_eq!(Percent::new(u32::MAX).as_basis_points(), None);
    assert_eq!(Percent::new(150).scale(Money(5)), Some(Money(7)));
    assert_eq!(Percent::new(150).scale(Money(-5)), Some(Money(-7)));
    assert_eq!(BasisPoints::new(50).scale(Money(5000)), Some(Money(25)));
    for amount in [i64::MIN, i64::MAX] {
        assert_eq!(Percent::new(100).scale(Money(amount)), Some(Money(amount)));
        assert_eq!(
            BasisPoints::new(10_000).scale(Money(amount)),
            Some(Money(amount))
        );
        assert_eq!(
            Percent::new(50).scale(Money(amount)),
            Some(Money(amount / 2))
        );
        assert!(Percent::new(101).scale(Money(amount)).is_none());
    }
    assert!(
        DaySpan::new(u32::MAX)
            .checked_add(DaySpan::new(1))
            .is_none()
    );
    assert_eq!(
        DaySpan::new(31).checked_sub(DaySpan::new(30)),
        Some(DaySpan::new(1))
    );
    assert_eq!(DaySpan::ZERO.checked_sub(DaySpan::new(1)), None);
}

#[test]
fn simple_accrual_is_independent_of_partition_and_rejects_wrong_carry_units() {
    let rate = BasisPoints::new(400);
    let principal = Money(1416);
    let year = DaySpan::new(365);
    let whole_year = rate.accrue(principal, year, year, None).unwrap();
    assert_eq!(whole_year.whole, Money(56));
    assert_eq!(whole_year.remainder.numerator(), 2_336_000);
    assert_eq!(whole_year.remainder.denominator(), 3_650_000);
    for partition in [vec![1; 365], vec![30, 335], vec![13, 17, 66, 269]] {
        let mut carry = None;
        let mut paid = 0;
        for days in partition {
            let accrued = rate
                .accrue(principal, DaySpan::new(days), year, carry)
                .unwrap();
            paid += accrued.whole.0;
            carry = Some(accrued.remainder);
        }
        assert_eq!(Money(paid), whole_year.whole);
        assert_eq!(carry, Some(whole_year.remainder));
    }
    let carry = Some(whole_year.remainder);
    assert_eq!(
        rate.accrue(Money(0), DaySpan::ZERO, year, carry)
            .unwrap()
            .remainder,
        whole_year.remainder
    );
    assert!(
        rate.accrue(principal, DaySpan::new(1), DaySpan::new(360), carry)
            .is_none()
    );
    assert!(
        rate.accrue(principal, DaySpan::new(1), DaySpan::ZERO, None)
            .is_none()
    );
    assert!(
        rate.accrue(Money(-1), DaySpan::new(1), year, None)
            .is_none()
    );
    assert!(MoneyRemainder::new(1, 1).is_none());
    assert!(MoneyRemainder::new(0, 0).is_none());
    assert!(
        BasisPoints::new(u32::MAX)
            .accrue(
                Money(i64::MAX),
                DaySpan::new(u32::MAX),
                DaySpan::new(1),
                None
            )
            .is_none()
    );
}

#[test]
fn rate_and_duration_json_retain_scalar_units_and_reject_invalid_numbers() {
    assert_eq!(serde_json::to_string(&Percent::new(4)).unwrap(), "4");
    assert_eq!(
        serde_json::to_string(&BasisPoints::new(400)).unwrap(),
        "400"
    );
    assert_eq!(serde_json::to_string(&DaySpan::new(30)).unwrap(), "30");
    assert_eq!(
        serde_json::from_str::<BasisPoints>("400").unwrap(),
        BasisPoints::new(400)
    );
    assert_eq!(
        serde_json::from_str::<DaySpan>("30").unwrap(),
        DaySpan::new(30)
    );
    for malformed in ["-1", "1.5", "4294967296", "null", "{\"units\":4}"] {
        assert!(serde_json::from_str::<Percent>(malformed).is_err());
        assert!(serde_json::from_str::<BasisPoints>(malformed).is_err());
        assert!(serde_json::from_str::<DaySpan>(malformed).is_err());
    }
}

#[test]
fn typed_interest_intervals_retain_fractions_across_conditional_principal() {
    let projection = InterestProjection::new(Money(1000), 0).unwrap();
    let days = DaySpan::new(5);
    assert_eq!(projection.due_for(days).unwrap(), Money(0));
    // Two separate rounded dues would be zero. The combined claim is one unit.
    assert_eq!(
        projection
            .due_with_addition_for(days, Money(1000), days)
            .unwrap(),
        Money(1)
    );
    for (total, extra) in [(0, 0), (5, 5), (30, 14), (365, 17), (u32::MAX, u32::MAX)] {
        assert_eq!(
            projection.due(total).unwrap(),
            projection.due_for(DaySpan::new(total)).unwrap()
        );
        assert_eq!(
            projection
                .due_with_addition(total, Money(2000), extra)
                .unwrap(),
            projection
                .due_with_addition_for(DaySpan::new(total), Money(2000), DaySpan::new(extra))
                .unwrap()
        );
    }
    assert!(
        projection
            .due_with_addition_for(DaySpan::new(1), Money(1), DaySpan::new(2))
            .is_err()
    );
    let mut forged = projection.clone();
    forged.daily_numerator += 1;
    assert!(forged.due_for(days).is_err());
}

#[test]
fn composed_money_rates_round_once_and_keep_period_units() {
    let combined = MoneyScale::ONE
        .percent(Percent::new(10))
        .unwrap()
        .percent(Percent::new(90))
        .unwrap();
    assert_eq!(combined.apply(Money(19)), Some(Money(1)));
    assert_eq!(combined.apply(Money(-19)), Some(Money(-1)));
    assert_eq!(
        Percent::new(90).scale(Percent::new(10).scale(Money(19)).unwrap()),
        Some(Money(0))
    );
    let equivalent = MoneyScale::ONE
        .basis_points(BasisPoints::new(1000))
        .unwrap()
        .percent(Percent::new(90))
        .unwrap();
    assert_eq!(combined, equivalent);
    let daily = MoneyScale::ONE
        .percent(Percent::new(4))
        .unwrap()
        .spread_over(DaySpan::new(365))
        .unwrap();
    assert_eq!(daily.apply(Money(36_500)), Some(Money(4)));
    assert_eq!(daily.apply(Money(-36_500)), Some(Money(-4)));
    assert_eq!(
        daily,
        MoneyScale::ONE
            .spread_over(DaySpan::new(365))
            .unwrap()
            .percent(Percent::new(4))
            .unwrap()
    );
    assert!(MoneyScale::ONE.spread_over(DaySpan::ZERO).is_none());
    assert!(Percent::try_from(-1).is_err());
}

#[test]
fn composed_money_rates_cancel_factors_and_reject_overflow() {
    let mut identity = MoneyScale::ONE;
    for _ in 0..100 {
        identity = identity
            .percent(Percent::new(100))
            .unwrap()
            .basis_points(BasisPoints::new(10_000))
            .unwrap();
    }
    assert_eq!(identity, MoneyScale::ONE);
    for amount in [i64::MIN, i64::MAX] {
        assert_eq!(identity.apply(Money(amount)), Some(Money(amount)));
        assert!(
            identity
                .percent(Percent::new(101))
                .unwrap()
                .apply(Money(amount))
                .is_none()
        );
    }
    let zero = MoneyScale::ONE
        .percent(Percent::ZERO)
        .unwrap()
        .percent(Percent::new(u32::MAX))
        .unwrap()
        .spread_over(DaySpan::new(u32::MAX))
        .unwrap();
    assert_eq!(zero.apply(Money(i64::MAX)), Some(Money(0)));
    let large = MoneyScale::ONE
        .percent(Percent::new(u32::MAX))
        .unwrap()
        .percent(Percent::new(u32::MAX))
        .unwrap();
    assert!(large.percent(Percent::new(u32::MAX)).is_none());
}
