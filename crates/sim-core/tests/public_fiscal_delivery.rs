use sim_core::*;
fn tax(plan: &PublishedFiscalPlan) -> &PublishedFiscalDelivery {
    plan.delivery
        .iter()
        .find(|p| p.policy == "income_tax")
        .unwrap()
}
#[test]
fn public_delivery_separates_review_schedule_and_phased_setting_operation_without_changing_world_state()
 {
    let content = Content::national_fiscal_bundled().unwrap();
    let foreign = CountryId("GBR".into());
    let mut world = World::new(&content, CountryId("FRA".into()), 17).unwrap();
    world.advance(&content, sim_core::DaySpan::new(30)).unwrap();
    assert_eq!(
        world.countries[&foreign]
            .fiscal_plans
            .as_ref()
            .unwrap()
            .bills
            .len(),
        1
    );
    assert!(
        world
            .public_report(&content, &foreign)
            .unwrap()
            .fiscal_plans
            .unwrap()
            .is_empty()
    );
    world.advance(&content, sim_core::DaySpan::new(14)).unwrap();
    let plans = world
        .public_report(&content, &foreign)
        .unwrap()
        .fiscal_plans
        .unwrap();
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].enacted_day, sim_core::CampaignDay::new(44));
    assert_eq!(
        plans[0].first_scheduled_delivery,
        sim_core::CampaignDay::new(108)
    );
    assert_eq!(plans[0].delivery.len(), 3);
    assert!(plans[0].delivery.iter().all(|p| p.operation.is_none()));
    let json = serde_json::to_value(&plans).unwrap();
    let serialized = json.to_string();
    for private in [
        "programmes",
        "project",
        "command",
        "fee",
        "implementation_cost",
        "sponsor",
        "forecast",
    ] {
        assert!(!serialized.contains(private), "{private}");
    }
    let mut earlier = json[0].clone();
    earlier.as_object_mut().unwrap().remove("delivery");
    assert!(
        serde_json::from_value::<PublishedFiscalPlan>(earlier)
            .unwrap()
            .delivery
            .is_empty()
    );
    world.advance(&content, sim_core::DaySpan::new(63)).unwrap();
    assert!(
        world
            .public_report(&content, &foreign)
            .unwrap()
            .fiscal_plans
            .unwrap()[0]
            .delivery
            .iter()
            .all(|p| p.operation.is_none())
    );
    world.advance(&content, sim_core::DaySpan::new(1)).unwrap();
    let plans = world
        .public_report(&content, &foreign)
        .unwrap()
        .fiscal_plans
        .unwrap();
    assert_eq!(
        tax(&plans[0]).operation,
        Some(PublishedFiscalOperation {
            day: sim_core::CampaignDay::new(108),
            value: 30
        })
    );
    assert!(
        plans[0]
            .delivery
            .iter()
            .find(|p| p.policy == "services")
            .unwrap()
            .operation
            .is_none()
    );
    world.advance(&content, sim_core::DaySpan::new(30)).unwrap();
    let plans = world
        .public_report(&content, &foreign)
        .unwrap()
        .fiscal_plans
        .unwrap();
    assert_eq!(
        plans[0]
            .delivery
            .iter()
            .find(|p| p.policy == "services")
            .unwrap()
            .operation,
        Some(PublishedFiscalOperation {
            day: sim_core::CampaignDay::new(138),
            value: 8
        })
    );
    assert_eq!(
        world.hash(),
        0x4be2fd07e49920b2,
        "public feedback must preserve the previously played campaign hash"
    );
    assert_eq!(persistence::replay(&world, &content).unwrap(), world);
}
#[test]
fn public_operations_keep_linked_historical_evidence_instead_of_inferring_it_from_current_settings()
{
    let content = Content::bundled().unwrap();
    for withdraw in [true, false] {
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        world
            .issue(
                &content,
                Action::ProposeBudget {
                    year: 2000,
                    targets: FiscalTargets {
                        income_tax: 30,
                        services: 8,
                        welfare: 3,
                    },
                },
            )
            .unwrap();
        world.advance(&content, sim_core::DaySpan::new(14)).unwrap();
        let bill = world.player_country().fiscal_plans.as_ref().unwrap().bills[0].clone();
        let project = bill
            .programmes
            .iter()
            .find(|p| p.policy == "income_tax")
            .unwrap()
            .project;
        if withdraw {
            world
                .issue(&content, Action::CancelCommitment { request: project })
                .unwrap();
            world
                .issue(
                    &content,
                    Action::Enact {
                        policy: "income_tax".into(),
                        value: 30,
                    },
                )
                .unwrap();
            world.advance(&content, sim_core::DaySpan::new(94)).unwrap();
            assert_eq!(world.player_country().tax, 30);
            let plans = world
                .public_report(&content, &world.player)
                .unwrap()
                .fiscal_plans
                .unwrap();
            assert_eq!(tax(&plans[0]).target, 30);
            assert!(
                tax(&plans[0]).operation.is_none(),
                "an unlinked replacement must not certify the withdrawn budget programme"
            );
        } else {
            world.advance(&content, sim_core::DaySpan::new(94)).unwrap();
            world
                .issue(
                    &content,
                    Action::Enact {
                        policy: "income_tax".into(),
                        value: 35,
                    },
                )
                .unwrap();
            world.advance(&content, sim_core::DaySpan::new(78)).unwrap();
            assert_eq!(world.player_country().tax, 35);
            let plans = world
                .public_report(&content, &world.player)
                .unwrap()
                .fiscal_plans
                .unwrap();
            assert_eq!(
                tax(&plans[0]).operation,
                Some(PublishedFiscalOperation {
                    day: sim_core::CampaignDay::new(78),
                    value: 30
                }),
                "later repeal must retain the dated historical operation"
            );
        }
        world.validate(&content).unwrap();
        assert_eq!(persistence::replay(&world, &content).unwrap(), world);
    }
}
