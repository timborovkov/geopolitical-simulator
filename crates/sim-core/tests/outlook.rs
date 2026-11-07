use sim_core::*;

fn setup() -> (Content, World) {
    let content =
        Content::from_json(include_str!("../../../data/synthetic-finance-test.json")).unwrap();
    let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    (content, world)
}

#[test]
fn conditional_loan_delta_matches_actual_disbursement_and_interest_without_mutating_the_plan() {
    let (content, mut baseline) = setup();
    let mut borrowed = baseline.clone();
    let loan = LoanAssumption {
        amount: Money(10000),
        decision_day: 14,
    };
    let before = borrowed.hash();
    let plan = borrowed.funding_outlook(&content, 15, Some(loan)).unwrap();
    assert_eq!(borrowed.hash(), before);
    assert_eq!(plan.additional_daily_interest, Money(1));
    assert_eq!(plan.points[13].if_approved, plan.points[13].without_loan);
    borrowed
        .issue(
            &content,
            Action::Borrow {
                amount: loan.amount,
            },
        )
        .unwrap();
    for offset in 1..=15 {
        borrowed
            .advance(&content, sim_core::DaySpan::new(1))
            .unwrap();
        baseline
            .advance(&content, sim_core::DaySpan::new(1))
            .unwrap();
        let actual_delta = borrowed.player_country().accounts.treasury.0
            - baseline.player_country().accounts.treasury.0;
        let point = &plan.points[offset];
        assert_eq!(actual_delta, point.if_approved.0 - point.without_loan.0);
    }
    assert_eq!(
        plan.points[14].if_approved.0 - plan.points[14].without_loan.0,
        9999
    );
    persistence::replay(&borrowed, &content).unwrap();
}

#[test]
fn immediate_manual_borrowing_adds_cash_now_and_interest_from_the_next_settlement() {
    let (content, world) = setup();
    let loan = LoanAssumption {
        amount: Money(10000),
        decision_day: world.day.units(),
    };
    let plan = world.funding_outlook(&content, 1, Some(loan)).unwrap();
    assert_eq!(
        plan.points[0].if_approved.0 - plan.points[0].without_loan.0,
        10000
    );
    assert_eq!(
        plan.points[1].if_approved.0 - plan.points[1].without_loan.0,
        9999
    );
}

#[test]
fn cash_runway_excludes_reservations_and_pending_funds_and_keeps_gaps_signed() {
    let (content, mut world) = setup();
    world
        .issue(
            &content,
            Action::Enact {
                policy: "energy".into(),
                value: 1,
            },
        )
        .unwrap();
    world
        .issue(
            &content,
            Action::Borrow {
                amount: Money(10000),
            },
        )
        .unwrap();
    let country = world.player_country();
    let plan = world.funding_outlook(&content, 365, None).unwrap();
    assert_eq!(
        plan.available_cash.0,
        country.accounts.treasury.0 - country.committed_cash.0
    );
    let deficit = -plan.daily_balance.0;
    assert!(deficit > 0);
    assert_eq!(
        plan.covered_days.unwrap(),
        (plan.available_cash.0 / deficit) as u64
    );
    let days = plan.covered_days.unwrap() as usize;
    // Use an isolated low-cash accounting fixture for a gap inside the horizon.
    let mut constrained = world.clone();
    let country = constrained.countries.get_mut(&world.player).unwrap();
    let cash = country.accounts.treasury.0 - country.committed_cash.0;
    country.accounts.treasury.0 -= cash;
    country.accounts.external.0 += cash;
    let gap = constrained.funding_outlook(&content, 2, None).unwrap();
    assert_eq!(gap.covered_days, Some(0));
    assert_eq!(gap.points[0].without_loan, Money(0));
    assert_eq!(gap.points[1].without_loan.0, -deficit);
    assert!(days > 0);
}

#[test]
fn balanced_run_rate_has_no_deficit_countdown_or_division_by_zero() {
    let (mut content, _) = setup();
    let definition = &mut content.countries[0];
    definition.gdp = 36500;
    definition.tax = 45;
    definition.capacity = 23;
    definition.services = 4;
    definition.welfare = 0;
    let world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let plan = world.funding_outlook(&content, 365, None).unwrap();
    assert_eq!(plan.daily_balance, Money(0));
    assert_eq!(plan.covered_days, None);
    assert!(
        plan.points
            .iter()
            .all(|point| point.without_loan == plan.available_cash)
    );
}

#[test]
fn unsafe_horizons_dates_inputs_and_overflow_fail_without_state_changes() {
    let (content, world) = setup();
    let before = world.hash();
    assert!(world.funding_outlook(&content, 366, None).is_err());
    let mut other = content.clone();
    other.id = "different-pack".into();
    assert!(world.funding_outlook(&other, 1, None).is_err());
    for loan in [
        LoanAssumption {
            amount: Money(0),
            decision_day: 14,
        },
        LoanAssumption {
            amount: Money(i64::MAX),
            decision_day: 14,
        },
        LoanAssumption {
            amount: Money(1),
            decision_day: MAX_DAY + 1,
        },
    ] {
        assert!(world.funding_outlook(&content, 1, Some(loan)).is_err());
    }
    assert_eq!(world.hash(), before);
    let mut ending = world.clone();
    ending.day = CampaignDay::new(MAX_DAY);
    assert!(ending.funding_outlook(&content, 1, None).is_err());
    assert_eq!(
        ending
            .funding_outlook(&content, 0, None)
            .unwrap()
            .points
            .len(),
        1
    );
    let mut invalid = world;
    invalid.countries.get_mut(&invalid.player).unwrap().debt = Money(i64::MAX);
    assert!(invalid.funding_outlook(&content, 1, None).is_err());
    let country = invalid.countries.get_mut(&invalid.player).unwrap();
    country.debt = Money(33333);
    country.accounts.treasury = Money(i64::MAX);
    country.tax = 45;
    country.services = 4;
    country.welfare = 0;
    assert!(
        invalid
            .funding_outlook(&content, 1, None)
            .unwrap_err()
            .contains("Funding balance overflow")
    );
}
