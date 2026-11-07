use sim_core::*;

#[test]
fn nonzero_bookmark_schedules_budget_and_loan_consent_across_new_year_and_replays_continuation() {
    let content = Content::from_json(include_str!(
        "../../../data/synthetic-governing-institutions-test.json"
    ))
    .unwrap()
    .at_bookmark("2000-12-18")
    .unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    let opening = world.day;
    let january = CampaignDay::new(parse_date("2001-01-01").unwrap());
    assert_eq!(
        world.player_country().cabinet.as_ref().unwrap().since,
        opening
    );
    assert_eq!(
        world.player_country().constitution.as_ref().unwrap().since,
        opening
    );
    assert_eq!(
        world
            .player_country()
            .next_election
            .unwrap()
            .elapsed_since(opening),
        Some(DaySpan::new(730))
    );
    let mut targets = FiscalTargets::current(world.player_country());
    targets.income_tax += 1;
    world
        .issue(
            &content,
            Action::ProposeBudget {
                year: 2001,
                targets,
            },
        )
        .unwrap();
    let loan = LoanId(world.next_command.0);
    world
        .issue(
            &content,
            Action::Borrow {
                amount: Money(1000),
            },
        )
        .unwrap();
    let bill = &world.player_country().fiscal_plans.as_ref().unwrap().bills[0];
    assert_eq!(bill.proposed_day, opening);
    assert_eq!(bill.review_day, january);
    assert_eq!(bill.effective_day, january);
    assert_eq!(
        bill.review_day.elapsed_since(opening),
        Some(DaySpan::new(14))
    );
    let pending = world
        .player_country()
        .financing
        .as_ref()
        .unwrap()
        .pending
        .as_ref()
        .unwrap();
    assert_eq!(pending.requested_day, opening);
    assert_eq!(pending.decision_day, january);
    let numeric: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&world, &content).unwrap()).unwrap();
    assert_eq!(numeric["schema"], 29);
    assert_eq!(
        numeric["world"]["countries"]["AUR"]["fiscal_plans"]["bills"][0]["review_day"],
        january.units()
    );
    assert_eq!(
        numeric["world"]["countries"]["AUR"]["financing"]["pending"]["decision_day"],
        january.units()
    );
    world.advance(&content, DaySpan::new(13)).unwrap();
    let before_january = persistence::encode(&world, &content).unwrap();
    let mut resumed = persistence::decode(&before_january, &content).unwrap();
    for campaign in [&mut world, &mut resumed] {
        campaign.advance(&content, DaySpan::new(1)).unwrap();
        let country = campaign.player_country();
        let bill = &country.fiscal_plans.as_ref().unwrap().bills[0];
        assert_eq!(bill.status, BudgetBillStatus::Enacted);
        assert_eq!(bill.resolved_day, Some(january));
        assert_eq!(country.budget.start_day, january);
        let issued = country
            .financing
            .as_ref()
            .unwrap()
            .instruments
            .iter()
            .find(|instrument| instrument.id == loan)
            .unwrap();
        assert_eq!(issued.issued_day, january);
        assert_eq!(
            issued.maturity_day,
            january.checked_add(content.finance.as_ref().unwrap().maturity_days.unwrap())
        );
        assert!(
            country
                .projects
                .iter()
                .any(|project| project.start_day == january
                    && project.consent_day == january
                    && project.due_day > january)
        );
        campaign
            .issue(
                &content,
                Action::HoldCash {
                    target: Money(100),
                    days: DaySpan::new(1),
                },
            )
            .unwrap();
        let expiry = january.checked_add(DaySpan::new(1)).unwrap();
        campaign.advance(&content, DaySpan::new(30)).unwrap();
        let held = campaign
            .player_country()
            .cash_management
            .as_ref()
            .unwrap()
            .records
            .last()
            .unwrap();
        assert_eq!(held.start_day, january);
        assert_eq!(held.expiry_day, expiry);
        assert!(matches!(held.ended, Some((day, CashHoldEnd::Expired)) if day == expiry));
        campaign.validate(&content).unwrap();
    }
    assert_eq!(world, resumed);
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    assert_eq!(
        persistence::decode(&persistence::encode(&world, &content).unwrap(), &content).unwrap(),
        world
    );
}
