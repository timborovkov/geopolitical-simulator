use sim_core::*;

fn fixture() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-cash-management-test.json"
    ))
    .unwrap()
}

#[test]
fn planning_quotes_full_requirements_without_treating_a_target_or_retained_cash_as_spendable() {
    let c = fixture();
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    w.issue(
        &c,
        Action::Repay {
            amount: w.player_country().available_cash(),
        },
    )
    .unwrap();
    let before = w.clone();
    let plan = w.policy_funding_plan(&c, &"income_tax".into(), 30).unwrap();
    assert_eq!(
        (
            plan.preview.cost,
            plan.available_cash,
            plan.retained_cash,
            plan.shortfall
        ),
        (Money(20), Money(0), Money(0), Money(20))
    );
    assert_eq!(plan.preview.consent_delay, 14);
    assert_eq!(w, before);
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
    assert_eq!(w, before);
    let request = CashHoldId(w.next_command.0);
    w.issue(
        &c,
        Action::HoldCash {
            target: Money(20),
            days: 90,
        },
    )
    .unwrap();
    assert_eq!(
        w.policy_funding_plan(&c, &"income_tax".into(), 30)
            .unwrap()
            .retained_cash,
        Money(0)
    );
    w.advance(&c, 1).unwrap();
    let before = w.clone();
    let held = w.policy_funding_plan(&c, &"income_tax".into(), 30).unwrap();
    assert!(held.retained_cash.0 > 0 && held.retained_cash.0 < 20);
    assert!(w.preview(&c, &w.player, &"income_tax".into(), 30).is_err());
    assert_eq!(held.shortfall, Money((20 - held.available_cash.0).max(0)));
    assert_eq!(w, before);
    assert!(w.preview(&c, &w.player, &"income_tax".into(), 30).is_err());
    for _ in 0..10 {
        if w.player_country().retained_cash() == Money(20) {
            break;
        }
        w.advance(&c, 1).unwrap();
    }
    assert_eq!(w.player_country().retained_cash(), Money(20));
    w.issue(&c, Action::ReleaseCash { request }).unwrap();
    let released = w.policy_funding_plan(&c, &"income_tax".into(), 30).unwrap();
    assert_eq!(released.shortfall, Money(0));
    assert_eq!(
        released.preview,
        w.preview(&c, &w.player, &"income_tax".into(), 30).unwrap()
    );
    w.issue(
        &c,
        Action::Enact {
            policy: "income_tax".into(),
            value: 30,
        },
    )
    .unwrap();
    assert!(
        w.policy_funding_plan(&c, &"income_tax".into(), 31)
            .unwrap_err()
            .contains("pending")
    );
    w.validate(&c).unwrap();
    assert_eq!(persistence::replay(&w, &c).unwrap(), w);
}

#[test]
fn funding_plans_keep_backing_value_and_content_checks_in_force() {
    let c = fixture();
    let mut w = World::new(&c, CountryId("AUR".into()), 17).unwrap();
    w.issue(
        &c,
        Action::EmergencyBorrow {
            amount: Money(1000),
        },
    )
    .unwrap();
    let before = w.clone();
    assert!(
        w.policy_funding_plan(&c, &"income_tax".into(), 30)
            .unwrap_err()
            .contains("backing")
    );
    let support = w.policy_funding_plan(&c, &"coalition".into(), 1).unwrap();
    assert_eq!(
        support.preview,
        w.preview(&c, &w.player, &"coalition".into(), 1).unwrap()
    );
    assert!(w.policy_funding_plan(&c, &"income_tax".into(), 99).is_err());
    let mut other = c.clone();
    other.name.push_str(" other version");
    assert!(
        w.policy_funding_plan(&other, &"coalition".into(), 1)
            .unwrap_err()
            .contains("exact content")
    );
    assert_eq!(w, before);
}
