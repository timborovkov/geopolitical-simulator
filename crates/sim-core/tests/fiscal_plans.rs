use sim_core::*;
fn setup() -> (Content, World) {
    let c = Content::from_json(include_str!(
        "../../../data/synthetic-fiscal-plans-test.json"
    ))
    .unwrap();
    let w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    (c, w)
}
fn target() -> FiscalTargets {
    FiscalTargets {
        income_tax: 28,
        services: 10,
        welfare: 3,
    }
}
fn propose(c: &Content, w: &mut World, year: u32, t: FiscalTargets) -> BudgetBillId {
    let id = BudgetBillId(w.next_command);
    w.issue(c, Action::ProposeBudget { year, targets: t })
        .unwrap();
    id
}
fn backing(c: &Content, w: &mut World) {
    for _ in 0..2 {
        let p = w.preview(c, &w.player, &"coalition".into(), 1).unwrap();
        w.issue(
            c,
            Action::Enact {
                policy: "coalition".into(),
                value: 1,
            },
        )
        .unwrap();
        w.advance(c, p.delay).unwrap();
    }
}
fn bill(w: &World) -> &BudgetBill {
    w.player_country()
        .fiscal_plans
        .as_ref()
        .unwrap()
        .bills
        .last()
        .unwrap()
}
fn check(c: &Content, w: &World) {
    w.validate(c).unwrap();
    let bytes = persistence::encode(w, c).unwrap();
    assert_eq!(persistence::decode(&bytes, c).unwrap(), *w);
    assert_eq!(persistence::replay(w, c).unwrap(), *w);
}
#[test]
fn a_supplementary_budget_reserves_then_funds_actual_programmes_and_replays_dispatch() {
    let (c, mut w) = setup();
    let player = w.player.clone();
    let before = w.player_country().clone();
    let p = w.preview_budget(&c, &player, 2000, &target()).unwrap();
    let id = propose(&c, &mut w, 2000, target());
    assert_eq!(w.player_country().committed_cash, p.implementation_cost);
    assert_eq!(
        w.player_country().accounts.treasury.0,
        before.accounts.treasury.0 - p.fee.0
    );
    assert_eq!(
        FiscalTargets::current(w.player_country()),
        FiscalTargets::current(&before)
    );
    assert!(w.player_country().projects.is_empty());
    check(&c, &w);
    assert!(
        w.preview(&c, &player, &"income_tax".into(), 30)
            .unwrap_err()
            .contains("pending budget")
    );
    w.advance(&c, p.review_day).unwrap();
    assert_eq!(bill(&w).status, BudgetBillStatus::Enacted);
    assert_eq!(bill(&w).programmes.len(), 3);
    assert_eq!(w.player_country().committed_cash, Money(0));
    assert_eq!(
        FiscalTargets::current(w.player_country()),
        FiscalTargets::current(&before)
    );
    assert_eq!(
        w.journal
            .iter()
            .filter(|e| e.budget_bill == Some(id))
            .count(),
        3
    );
    check(&c, &w);
    let due = w
        .player_country()
        .projects
        .iter()
        .map(|p| p.due_day)
        .max()
        .unwrap();
    w.advance(&c, due - w.day).unwrap();
    assert_eq!(FiscalTargets::current(w.player_country()), target());
    assert!(bill(&w).programmes.iter().all(|p| {
        w.player_country()
            .commitments
            .as_ref()
            .unwrap()
            .iter()
            .any(|r| r.id == p.project && r.direct_effects.len() == 1)
    }));
    check(&c, &w);
}
#[test]
fn withdrawal_keeps_fees_and_interval_but_releases_implementation() {
    let (c, mut w) = setup();
    let before = w.player_country().accounts.treasury;
    let id = propose(&c, &mut w, 2000, target());
    let fee = bill(&w).fee;
    w.issue(&c, Action::WithdrawBudget { request: id }).unwrap();
    assert_eq!(w.player_country().accounts.treasury.0, before.0 - fee.0);
    assert_eq!(w.player_country().committed_cash, Money(0));
    assert!(
        w.preview_budget(&c, &w.player, 2000, &target())
            .unwrap_err()
            .contains("interval")
    );
    check(&c, &w);
}
#[test]
fn failed_review_is_atomic_and_cannot_spend_protected_local_cash() {
    let (c, mut w) = setup();
    propose(&c, &mut w, 2000, target());
    w.countries
        .get_mut(&w.player.clone())
        .unwrap()
        .legislative_support = 0;
    let due = bill(&w).review_day;
    w.advance(&c, due).unwrap();
    assert!(matches!(bill(&w).status, BudgetBillStatus::Refused { .. }));
    assert!(w.player_country().projects.is_empty());
    assert_eq!(w.player_country().committed_cash, Money(0));
    assert!(!w.journal.iter().any(|e| e.budget_bill.is_some()));
    w.validate(&c).unwrap();
}
#[test]
fn annual_plans_never_operate_before_january_and_preserve_year_end_execution() {
    let (c, mut w) = setup();
    assert!(
        w.preview_budget(&c, &w.player, 2001, &target())
            .unwrap_err()
            .contains("window")
    );
    w.advance(&c, 200).unwrap();
    // Ordinary AI and settlement run before proposal; retain the current values for unchanged items.
    let mut t = FiscalTargets::current(w.player_country());
    t.income_tax = if t.income_tax == 28 { 29 } else { 28 };
    let id = propose(&c, &mut w, 2001, t.clone());
    let review = bill(&w).review_day;
    let jan = parse_date("2001-01-01").unwrap();
    let original = w.player_country().tax;
    w.advance(&c, review - w.day).unwrap();
    assert_eq!(bill(&w).status, BudgetBillStatus::Enacted);
    assert_eq!(
        w.player_country()
            .projects
            .iter()
            .find(|p| p.policy == "income_tax")
            .unwrap()
            .due_day,
        jan
    );
    check(&c, &w);
    w.advance(&c, jan - 1 - w.day).unwrap();
    assert_eq!(w.player_country().tax, original);
    check(&c, &w);
    w.advance(&c, 1).unwrap();
    assert_eq!(w.player_country().tax, t.income_tax);
    assert_eq!(w.player_country().budget_reviews.len(), 1);
    assert!(w.journal.iter().any(|e| e.budget_bill == Some(id)));
    check(&c, &w);
}
#[test]
fn successor_closes_pending_bills_but_keeps_enacted_funded_projects() {
    let (c, mut w) = setup();
    backing(&c, &mut w);
    propose(&c, &mut w, 2000, target());
    w.issue(
        &c,
        Action::Reconstitute {
            target: Regime::Military,
            contested: true,
        },
    )
    .unwrap();
    w.advance(&c, 14).unwrap();
    assert_eq!(bill(&w).status, BudgetBillStatus::Superseded);
    assert_eq!(w.player_country().committed_cash, Money(0));
    check(&c, &w);
    let (c, mut w) = setup();
    backing(&c, &mut w);
    propose(&c, &mut w, 2000, target());
    let review = bill(&w).review_day;
    w.advance(&c, review - w.day).unwrap();
    w.issue(
        &c,
        Action::Reconstitute {
            target: Regime::Military,
            contested: true,
        },
    )
    .unwrap();
    w.advance(&c, 14).unwrap();
    assert_eq!(bill(&w).status, BudgetBillStatus::Enacted);
    assert!(!w.player_country().projects.is_empty());
    check(&c, &w);
}
#[test]
fn public_plans_hide_pending_deliberation_and_private_finance() {
    let (c, mut w) = setup();
    propose(&c, &mut w, 2000, target());
    let report = w.public_report(&c, &w.player).unwrap();
    assert_eq!(report.fiscal_plans, Some(Vec::new()));
    w.advance(&c, bill(&w).review_day).unwrap();
    let report = w.public_report(&c, &w.player).unwrap();
    let published = serde_json::to_value(report.fiscal_plans.unwrap()).unwrap();
    assert_eq!(published.as_array().unwrap().len(), 1);
    let s = published.to_string();
    for private in [
        "fee",
        "implementation_cost",
        "sponsor",
        "programmes",
        "forecast",
    ] {
        assert!(!s.contains(private));
    }
    check(&c, &w);
}
#[test]
fn malformed_dispatch_versions_and_reservations_are_rejected() {
    let (c, mut w) = setup();
    propose(&c, &mut w, 2000, target());
    let mut bad = w.clone();
    bad.countries
        .get_mut(&w.player.clone())
        .unwrap()
        .committed_cash = Money(0);
    assert!(bad.validate(&c).is_err());
    w.advance(&c, bill(&w).review_day).unwrap();
    let mut bad = w.clone();
    bad.countries
        .get_mut(&w.player.clone())
        .unwrap()
        .fiscal_plans
        .as_mut()
        .unwrap()
        .bills[0]
        .proposed_day = u32::MAX;
    assert!(bad.validate(&c).is_err());
    let mut bad = w.clone();
    bad.journal
        .iter_mut()
        .find(|e| e.budget_bill.is_some())
        .unwrap()
        .budget_bill = None;
    assert!(bad.validate(&c).is_err());
    let mut bad = w.clone();
    bad.countries
        .get_mut(&w.player.clone())
        .unwrap()
        .fiscal_plans
        .as_mut()
        .unwrap()
        .bills[0]
        .programmes
        .pop();
    assert!(bad.validate(&c).is_err());
    let mut bytes: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&w, &c).unwrap()).unwrap();
    assert_eq!(bytes["schema"], 13);
    bytes["schema"] = 12.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&bytes).unwrap(), &c)
            .unwrap_err()
            .contains("format 13")
    );
    let mut old = c.clone();
    old.schema = 11;
    old.rules = 11;
    assert!(old.validate().is_err());
    check(&c, &w);
}
#[test]
fn overlapping_proposals_invalid_years_and_unfunded_packages_reject_without_mutation() {
    let (c, mut w) = setup();
    for (y, t) in [
        (1999, target()),
        (2002, target()),
        (
            2000,
            FiscalTargets {
                income_tax: 99,
                services: 9,
                welfare: 3,
            },
        ),
        (2000, FiscalTargets::current(w.player_country())),
    ] {
        let hash = w.hash();
        assert!(
            w.issue(
                &c,
                Action::ProposeBudget {
                    year: y,
                    targets: t
                }
            )
            .is_err()
        );
        assert_eq!(w.hash(), hash);
    }
    w.issue(
        &c,
        Action::Enact {
            policy: "services".into(),
            value: 10,
        },
    )
    .unwrap();
    let hash = w.hash();
    assert!(
        w.issue(
            &c,
            Action::ProposeBudget {
                year: 2000,
                targets: target()
            }
        )
        .is_err()
    );
    assert_eq!(w.hash(), hash);
}
#[test]
fn combined_costs_and_local_claims_are_checked_before_any_fee_is_paid() {
    let (c, mut w) = setup();
    let p = w.preview_budget(&c, &w.player, 2000, &target()).unwrap();
    // Preserve the four-account total while leaving less than the package requires.
    let ctry = w.countries.get_mut(&w.player.clone()).unwrap();
    let remaining = p.implementation_cost.0 + p.fee.0 - 1;
    let moved = ctry.accounts.treasury.0 - remaining;
    ctry.accounts.treasury.0 -= moved;
    ctry.accounts.firms.0 += moved;
    ctry.budget.capital.0 += moved;
    let hash = w.hash();
    assert!(
        w.issue(
            &c,
            Action::ProposeBudget {
                year: 2000,
                targets: target()
            }
        )
        .unwrap_err()
        .contains("uncommitted")
    );
    assert_eq!(w.hash(), hash);
    w.validate(&c).unwrap();
    let (c, mut w) = setup();
    w.advance(&c, 10).unwrap();
    let p = w.preview_budget(&c, &w.player, 2000, &target()).unwrap();
    let ctry = w.countries.get_mut(&w.player.clone()).unwrap();
    let local = ctry.accounts.treasury.0 - ctry.available_cash().0;
    assert!(local > 0);
    let remaining = (p.implementation_cost.0 + p.fee.0).max(local);
    let moved = ctry.accounts.treasury.0 - remaining;
    ctry.accounts.treasury.0 -= moved;
    ctry.accounts.firms.0 += moved;
    ctry.budget.capital.0 += moved;
    assert!(w.player_country().accounts.treasury.0 >= p.implementation_cost.0 + p.fee.0);
    let hash = w.hash();
    assert!(
        w.issue(
            &c,
            Action::ProposeBudget {
                year: 2000,
                targets: target()
            }
        )
        .is_err()
    );
    assert_eq!(w.hash(), hash);
    w.validate(&c).unwrap();
}
#[test]
fn budget_delivery_can_fulfil_support_without_fabricating_an_extra_programme() {
    let (c, mut w) = setup();
    w.issue(
        &c,
        Action::DeclareUndertaking {
            template: "service-agreement".into(),
            project: None,
        },
    )
    .unwrap();
    propose(&c, &mut w, 2000, target());
    let due = bill(&w).review_day;
    w.advance(&c, due).unwrap();
    let service = bill(&w)
        .programmes
        .iter()
        .find(|p| p.policy == "services")
        .unwrap()
        .project;
    let due = w
        .player_country()
        .projects
        .iter()
        .find(|p| p.id == service.0)
        .unwrap()
        .due_day;
    w.advance(&c, due - w.day).unwrap();
    assert!(
        w.player_country()
            .undertakings
            .as_ref()
            .unwrap()
            .records
            .iter()
            .any(|r| r.status == UndertakingStatus::Fulfilled { evidence: service })
    );
    check(&c, &w);
}
#[test]
fn funded_annual_programmes_keep_their_dates_and_ordinary_withdrawal_is_sunk() {
    let (c, mut w) = setup();
    w.advance(&c, 200).unwrap();
    let id = propose(&c, &mut w, 2001, target());
    let review = bill(&w).review_day;
    w.advance(&c, review - w.day).unwrap();
    let p = bill(&w).programmes[0].project;
    assert!(w.preview_amendment(&c, &w.player, p, 29).is_err());
    assert!(w.issue(&c, Action::WithdrawBudget { request: id }).is_err());
    let before = w.player_country().accounts.treasury;
    w.issue(&c, Action::CancelCommitment { request: p })
        .unwrap();
    assert_eq!(w.player_country().accounts.treasury, before);
    assert_eq!(bill(&w).status, BudgetBillStatus::Enacted);
    check(&c, &w);
}
#[test]
fn fiscal_ai_uses_the_same_budget_commands_and_long_campaign_replay() {
    let (c, mut w) = setup();
    w.advance(&c, 720).unwrap();
    assert!(
        w.journal
            .iter()
            .any(|e| e.ai && matches!(e.command.action, Action::ProposeBudget { .. }))
    );
    check(&c, &w);
}
#[test]
fn a_later_programme_refusal_rolls_back_earlier_dispatch_and_payments() {
    let c = Content::from_json(include_str!(
        "../../../data/synthetic-fiscal-plans-test.json"
    ))
    .unwrap();
    let mut w = World::new(&c, CountryId("COR".into()), 17).unwrap();
    propose(
        &c,
        &mut w,
        2000,
        FiscalTargets {
            income_tax: 28,
            services: 14,
            welfare: 3,
        },
    );
    let fee = bill(&w).fee;
    // Tax still passes royal authority; the later large service expansion no longer does.
    w.countries
        .get_mut(&w.player.clone())
        .unwrap()
        .council_support = 60;
    w.advance(&c, bill(&w).review_day).unwrap();
    assert!(
        matches!(&bill(&w).status,BudgetBillStatus::Refused{reason} if reason.contains("royal spending"))
    );
    assert!(w.player_country().projects.is_empty());
    assert_eq!(w.player_country().budget.capital, fee);
    assert_eq!(w.player_country().committed_cash, Money(0));
    assert!(!w.journal.iter().any(|e| e.budget_bill.is_some()));
    w.validate(&c).unwrap();
}
#[test]
fn proposals_near_campaign_end_reject_without_overflow_or_partial_costs() {
    let mut c = Content::from_json(include_str!(
        "../../../data/synthetic-fiscal-plans-test.json"
    ))
    .unwrap();
    c.start = "2094-01-01".into();
    c = c.at_bookmark("2094-01-01").unwrap();
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    w.day = MAX_DAY - 1;
    let hash = w.hash();
    assert!(
        w.issue(
            &c,
            Action::ProposeBudget {
                year: 2099,
                targets: target()
            }
        )
        .is_err()
    );
    assert_eq!(w.hash(), hash);
}
#[test]
fn constitutional_review_clocks_are_honored_once_and_survive_later_history() {
    let (mut c, _) = setup();
    let military = c
        .constitutions
        .as_mut()
        .unwrap()
        .templates
        .iter_mut()
        .find(|t| t.regime == Regime::Military)
        .unwrap();
    military.government.consent_days = 180;
    military.government.leadership_review_days = Some(1827);
    c.validate().unwrap();
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    backing(&c, &mut w);
    w.issue(
        &c,
        Action::Reconstitute {
            target: Regime::Military,
            contested: true,
        },
    )
    .unwrap();
    w.advance(&c, 14).unwrap();
    let p = w.preview_budget(&c, &w.player, 2000, &target()).unwrap();
    assert_eq!(p.review_day, w.day + 180);
    propose(&c, &mut w, 2000, target());
    check(&c, &w);
    w.advance(&c, 180).unwrap();
    assert_eq!(bill(&w).status, BudgetBillStatus::Enacted);
    let tax = w
        .player_country()
        .projects
        .iter()
        .find(|p| p.policy == "income_tax")
        .unwrap();
    assert_eq!(tax.consent_day, w.day);
    assert!(tax.due_day - w.day < 180);
    check(&c, &w);
}
#[test]
fn january_delivery_and_settlement_belong_to_the_new_fiscal_year() {
    let (c, mut w) = setup();
    w.advance(&c, 200).unwrap();
    propose(&c, &mut w, 2001, target());
    let jan = parse_date("2001-01-01").unwrap();
    w.advance(&c, jan - 1 - w.day).unwrap();
    let previous = w.player_country().budget.clone();
    let closing_cash = w.player_country().accounts.treasury;
    w.advance(&c, 1).unwrap();
    let ctry = w.player_country();
    let review = ctry.budget_reviews.last().unwrap();
    assert_eq!(
        review.period, previous,
        "January settlement must not change the closed year's execution"
    );
    assert_eq!(review.closing_cash, closing_cash);
    assert_eq!(ctry.budget.revenue, ctry.report.revenue);
    assert_eq!(
        ctry.budget.payments.0,
        ctry.report.spending.0 + ctry.report.interest.0
    );
    assert_eq!(FiscalTargets::current(ctry), target());
    check(&c, &w);
}
#[test]
fn january_review_charges_new_year_implementation_and_late_supplementary_bills_are_rejected() {
    let (c, mut w) = setup();
    let jan = parse_date("2001-01-01").unwrap();
    w.advance(&c, jan - 14).unwrap();
    let hash = w.hash();
    assert!(
        w.issue(
            &c,
            Action::ProposeBudget {
                year: 2000,
                targets: target()
            }
        )
        .unwrap_err()
        .contains("selected fiscal year")
    );
    assert_eq!(w.hash(), hash);
    propose(&c, &mut w, 2001, target());
    let cost = bill(&w).implementation_cost;
    let fee = bill(&w).fee;
    w.advance(&c, 13).unwrap();
    let previous = w.player_country().budget.clone();
    assert_eq!(previous.capital, fee);
    w.advance(&c, 1).unwrap();
    assert_eq!(bill(&w).status, BudgetBillStatus::Enacted);
    assert_eq!(
        w.player_country().budget_reviews.last().unwrap().period,
        previous
    );
    assert_eq!(w.player_country().budget.capital, cost);
    check(&c, &w);
}
#[test]
fn calendar_forecasts_count_only_the_periods_actual_settlement_days() {
    let (c, mut w) = setup();
    let initial = w.player_country().budget.clone();
    assert_eq!(initial.opening_day_settled, Some(false));
    assert_eq!(
        initial.expected_revenue(initial.end_day),
        initial.forecast.daily_revenue.0 * 365
    );
    let jan = parse_date("2001-01-01").unwrap();
    w.advance(&c, jan).unwrap();
    let next = &w.player_country().budget;
    assert_eq!(next.opening_day_settled, Some(true));
    assert_eq!(next.expected_revenue(jan), next.forecast.daily_revenue.0);
    assert_eq!(
        next.expected_revenue(next.end_day),
        next.forecast.daily_revenue.0 * 365
    );
    let mut bad = w.clone();
    bad.countries
        .get_mut(&w.player.clone())
        .unwrap()
        .budget
        .opening_day_settled = None;
    assert!(bad.validate(&c).is_err());
    check(&c, &w);
}
