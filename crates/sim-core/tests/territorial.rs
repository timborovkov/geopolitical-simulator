use sim_core::*;
fn content() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-territorial-test.json"
    ))
    .unwrap()
}
fn ready() -> Content {
    let mut c = content();
    let r = c.territorial.as_mut().unwrap();
    r.lawful.legislative_threshold = 50;
    r.lawful.elite_threshold = 50;
    r.lawful.council_threshold = 40;
    r.lawful.approval_threshold = 40;
    r.local_consent_approval = 50;
    r.contested.as_mut().unwrap().elite_threshold = 50;
    r.contested.as_mut().unwrap().council_threshold = 40;
    r.requisition.as_mut().unwrap().elite_threshold = 50;
    r.requisition.as_mut().unwrap().council_threshold = 40;
    c
}
fn start(c: &Content, id: &str) -> World {
    World::new(c, CountryId(id.into()), 17).unwrap()
}
fn region(id: &str, name: &str) -> RegionId {
    RegionId(format!("region:{id}:{name}"))
}
fn reform(w: &mut World, c: &Content, target: TerritorialForm, contested: bool) {
    w.issue(c, Action::ReformTerritory { target, contested })
        .unwrap();
}
fn check(w: &World, c: &Content) {
    w.validate(c).unwrap();
    let bytes = persistence::encode(w, c).unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["schema"],
        11
    );
    assert_eq!(persistence::decode(&bytes, c).unwrap(), *w);
    persistence::replay(w, c).unwrap();
}
#[test]
fn lawful_devolution_changes_real_powers_and_tax_rights_without_changing_the_cabinet() {
    let c = ready();
    let mut w = start(&c, "COR");
    let opening = w.player_country().accounts.total();
    let p = w
        .preview_territorial_reform(&c, &w.player, &TerritorialForm::Federation, false)
        .unwrap();
    assert!(p.rights_lost.is_empty());
    assert_eq!(p.admin_cost, Money(500));
    assert_eq!(p.upfront, Money(100));
    assert_eq!(p.reserved, Money(400));
    let hash = w.hash();
    w.preview_territorial_reform(&c, &w.player, &TerritorialForm::Federation, false)
        .unwrap();
    assert_eq!(w.hash(), hash);
    reform(&mut w, &c, TerritorialForm::Federation, false);
    check(&w, &c);
    w.advance(&c, 90).unwrap();
    assert_eq!(
        w.territorial_form_at(&c, &w.player, w.day).unwrap(),
        TerritorialForm::Federation
    );
    assert_eq!(w.player_country().government_number, 1);
    assert_eq!(w.player_country().accounts.total(), opening);
    assert_eq!(
        w.player_country()
            .subnational
            .as_ref()
            .unwrap()
            .regions
            .iter()
            .map(|r| r.tax_entitlement_bps)
            .sum::<i64>(),
        3000
    );
    assert!(
        w.preview_regional_order(
            &c,
            &w.player,
            &region("COR", "city"),
            &RegionalOrderKind::Directive { level: 3 }
        )
        .is_err()
    );
    check(&w, &c);
}
#[test]
fn lawful_removal_requires_local_consent_and_reserves_compensation() {
    let mut c = ready();
    c.territorial.as_mut().unwrap().local_consent_approval = 65;
    let w = start(&c, "BOR");
    assert!(
        w.preview_territorial_reform(&c, &w.player, &TerritorialForm::DelegatedUnitary, false)
            .unwrap_err()
            .contains("local-consent")
    );
    c.territorial.as_mut().unwrap().local_consent_approval = 0;
    let mut w = start(&c, "BOR");
    let p = w
        .preview_territorial_reform(&c, &w.player, &TerritorialForm::DelegatedUnitary, false)
        .unwrap();
    assert_eq!(p.compensations.len(), 2);
    assert_eq!(p.reserved, Money(1400));
    reform(&mut w, &c, TerritorialForm::DelegatedUnitary, false);
    assert_eq!(w.player_country().committed_cash, Money(1400));
    check(&w, &c);
    w.advance(&c, 90).unwrap();
    assert_eq!(
        w.player_country()
            .territorial
            .as_ref()
            .unwrap()
            .history
            .len(),
        1
    );
    assert_eq!(w.player_country().committed_cash, Money(0));
    assert!(
        w.regional_mandate_at(&c, &w.player, &region("BOR", "province"), 89)
            .unwrap()
            .autonomous_services
    );
    assert!(
        !w.regional_mandate_at(&c, &w.player, &region("BOR", "province"), 90)
            .unwrap()
            .autonomous_services
    );
    check(&w, &c);
}
#[test]
fn withdrawal_and_review_refusal_keep_sunk_administration_and_interval() {
    let c = ready();
    let mut w = start(&c, "AUR");
    reform(&mut w, &c, TerritorialForm::Federation, false);
    w.issue(
        &c,
        Action::WithdrawTerritorialReform {
            request: TerritorialOrderId(1),
        },
    )
    .unwrap();
    assert_eq!(w.player_country().committed_cash, Money(0));
    assert_eq!(w.player_country().accounts.treasury, Money(4900));
    assert!(
        w.preview_territorial_reform(&c, &w.player, &TerritorialForm::Federation, false)
            .is_err()
    );
    check(&w, &c);
    let mut c = ready();
    c.territorial.as_mut().unwrap().lawful.days = 1;
    let mut w = start(&c, "AUR");
    w.advance(&c, 20).unwrap();
    reform(&mut w, &c, TerritorialForm::Federation, false);
    w.issue(
        &c,
        Action::RequisitionRegionalFunds {
            region: region("AUR", "province"),
            amount: Money(1),
        },
    )
    .unwrap();
    w.advance(&c, 1).unwrap();
    assert!(matches!(
        w.player_country().territorial.as_ref().unwrap().reforms[0].status,
        TerritorialReformStatus::Refused { .. }
    ));
    assert_eq!(w.player_country().committed_cash, Money(0));
    check(&w, &c);
}
#[test]
fn contested_change_has_explicit_power_gate_and_accepted_grants_survive_new_charters() {
    let c = content();
    let w = start(&c, "AUR");
    assert!(
        w.preview_territorial_reform(&c, &w.player, &TerritorialForm::DelegatedUnitary, true)
            .is_err()
    );
    let mut c = ready();
    c.territorial
        .as_mut()
        .unwrap()
        .contested
        .as_mut()
        .unwrap()
        .days = 20;
    let mut w = start(&c, "AUR");
    w.issue(
        &c,
        Action::RegionalOrder {
            region: region("AUR", "province"),
            kind: RegionalOrderKind::Grant { boost: 1, days: 30 },
        },
    )
    .unwrap();
    reform(&mut w, &c, TerritorialForm::DelegatedUnitary, true);
    check(&w, &c);
    w.advance(&c, 20).unwrap();
    let s = w.player_country().subnational.as_ref().unwrap();
    assert_eq!(s.orders[0].status, RegionalOrderStatus::Implementing);
    assert_eq!(s.orders[0].spent, Money(7));
    assert_eq!(
        w.territorial_form_at(&c, &w.player, w.day).unwrap(),
        TerritorialForm::DelegatedUnitary
    );
    check(&w, &c);
    w.advance(&c, 23).unwrap();
    assert_eq!(
        w.player_country().subnational.as_ref().unwrap().orders[0].status,
        RegionalOrderStatus::Completed
    );
    check(&w, &c);
}
#[test]
fn new_charter_releases_pending_regional_escrow_but_keeps_fees_and_history() {
    let mut c = ready();
    c.territorial.as_mut().unwrap().lawful.days = 7;
    let mut w = start(&c, "AUR");
    w.issue(
        &c,
        Action::RegionalOrder {
            region: region("AUR", "province"),
            kind: RegionalOrderKind::Grant { boost: 1, days: 30 },
        },
    )
    .unwrap();
    reform(&mut w, &c, TerritorialForm::Federation, false);
    w.advance(&c, 7).unwrap();
    let o = &w.player_country().subnational.as_ref().unwrap().orders[0];
    assert_eq!(o.status, RegionalOrderStatus::Superseded);
    assert_eq!(o.escrow, Money(0));
    assert_eq!(o.paid_administration, Money(20));
    check(&w, &c);
}
#[test]
fn contested_requisition_changes_discretion_keeps_settled_report_and_cannot_take_grants() {
    let c = ready();
    let mut w = start(&c, "AUR");
    w.issue(
        &c,
        Action::RegionalOrder {
            region: region("AUR", "province"),
            kind: RegionalOrderKind::Grant { boost: 1, days: 30 },
        },
    )
    .unwrap();
    w.advance(&c, 20).unwrap();
    let before = w.player_country().clone();
    let local = &before.subnational.as_ref().unwrap().regions[0];
    let amount = local.cash;
    assert!(amount.0 > 0);
    assert!(
        w.preview_regional_requisition(&c, &w.player, &local.id, Money(amount.0 + 1))
            .is_err()
    );
    let p = w
        .preview_regional_requisition(&c, &w.player, &local.id, amount)
        .unwrap();
    w.issue(
        &c,
        Action::RequisitionRegionalFunds {
            region: local.id.clone(),
            amount,
        },
    )
    .unwrap();
    let after = w.player_country();
    assert_eq!(
        after.accounts.treasury.0,
        before.accounts.treasury.0 - p.fee.0
    );
    assert_eq!(
        after.available_cash().0,
        before.available_cash().0 + amount.0 - p.fee.0
    );
    assert_eq!(after.accounts.total(), before.accounts.total());
    assert_eq!(
        after.subnational.as_ref().unwrap().orders,
        before.subnational.as_ref().unwrap().orders
    );
    assert_eq!(
        after.subnational.as_ref().unwrap().regions[0].last,
        local.last
    );
    assert!(!w.is_developer_modified());
    check(&w, &c);
    w.advance(&c, 1).unwrap();
    check(&w, &c);
}
#[test]
fn published_charter_history_excludes_private_reservations_and_pending_reviews() {
    let c = ready();
    let mut w = start(&c, "COR");
    let before = w.public_report(&c, &w.player).unwrap();
    reform(&mut w, &c, TerritorialForm::Federation, false);
    assert_eq!(w.public_report(&c, &w.player).unwrap(), before);
    w.advance(&c, 90).unwrap();
    let view = w.project(&c, Viewer::Observer).unwrap();
    assert!(view.own.is_none());
    let t = view.reports[&w.player].territorial.as_ref().unwrap();
    assert_eq!(t.form, TerritorialForm::Federation);
    assert_eq!(t.last_change.as_ref().unwrap().day, 90);
    check(&w, &c);
}
#[test]
fn snapshot_format_and_corrupted_territorial_dates_are_rejected() {
    let c = ready();
    let mut w = start(&c, "COR");
    reform(&mut w, &c, TerritorialForm::Federation, false);
    check(&w, &c);
    let mut json: serde_json::Value =
        serde_json::from_slice(&persistence::encode(&w, &c).unwrap()).unwrap();
    json["schema"] = 10.into();
    assert!(
        persistence::decode(&serde_json::to_vec(&json).unwrap(), &c)
            .unwrap_err()
            .contains("format 11")
    );
    let s = w
        .countries
        .get_mut(&w.player.clone())
        .unwrap()
        .territorial
        .as_mut()
        .unwrap();
    s.reforms[0].proposed_day = u32::MAX;
    assert!(w.validate(&c).is_err());
}

#[test]
fn national_succession_releases_pending_territorial_reservations() {
    let mut c = ready();
    let route = c
        .constitutions
        .as_mut()
        .unwrap()
        .contested
        .as_mut()
        .unwrap();
    route.elite_threshold = 50;
    route.council_threshold = 40;
    let mut w = start(&c, "COR");
    reform(&mut w, &c, TerritorialForm::Federation, false);
    w.issue(
        &c,
        Action::Reconstitute {
            target: Regime::Military,
            contested: true,
        },
    )
    .unwrap();
    w.advance(&c, 14).unwrap();
    assert_eq!(
        w.player_country().territorial.as_ref().unwrap().reforms[0].status,
        TerritorialReformStatus::Superseded
    );
    assert_eq!(w.player_country().committed_cash, Money(0));
    assert_eq!(
        w.territorial_form_at(&c, &w.player, w.day).unwrap(),
        TerritorialForm::DelegatedUnitary
    );
    check(&w, &c);
}
#[test]
fn a_delivered_historical_directive_remains_valid_after_the_authority_becomes_autonomous() {
    let c = ready();
    let mut w = start(&c, "COR");
    w.advance(&c, 1).unwrap();
    w.issue(
        &c,
        Action::RegionalOrder {
            region: region("COR", "city"),
            kind: RegionalOrderKind::Directive { level: 3 },
        },
    )
    .unwrap();
    reform(&mut w, &c, TerritorialForm::Federation, false);
    w.advance(&c, 90).unwrap();
    assert!(
        w.regional_mandate_at(&c, &w.player, &region("COR", "city"), w.day)
            .unwrap()
            .autonomous_services
    );
    assert_eq!(
        w.player_country().subnational.as_ref().unwrap().regions[1].service_level,
        3
    );
    check(&w, &c);
}
#[test]
fn autonomous_ai_uses_the_same_paid_territorial_commands_and_replays() {
    let c = ready();
    let mut w = start(&c, "COR");
    w.advance(&c, 270).unwrap();
    assert!(w.journal.iter().any(|entry| entry.ai
        && matches!(
            entry.command.action,
            Action::ReformTerritory {
                contested: false,
                ..
            }
        )));
    assert!(
        w.journal
            .iter()
            .filter(|entry| matches!(entry.command.action, Action::ReformTerritory { .. }))
            .all(|entry| !entry.developer)
    );
    check(&w, &c);
}
#[test]
fn malformed_rights_funds_and_history_are_rejected_without_arithmetic_panics() {
    let c = ready();
    let mut w = start(&c, "COR");
    reform(&mut w, &c, TerritorialForm::Federation, false);
    w.advance(&c, 90).unwrap();
    check(&w, &c);
    for kind in 0..5 {
        let mut bad = w.clone();
        let state = bad
            .countries
            .get_mut(&w.player)
            .unwrap()
            .territorial
            .as_mut()
            .unwrap();
        match kind {
            0 => state.reforms[0].reserved = Money(i64::MIN),
            1 => state.history[0].compensations.push(RegionalCompensation {
                region: region("COR", "city"),
                amount: Money(i64::MAX),
            }),
            2 => state.history[0].day = u32::MAX,
            3 => state.next_requisition_day = u32::MAX,
            _ => state.reforms[0].sponsor = u32::MAX,
        }
        assert!(bad.validate(&c).is_err(), "mutation {kind}");
    }
    let mut bad = c.clone();
    bad.territorial.as_mut().unwrap().templates[0].tax_pool_bps = 10001;
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.territorial.as_mut().unwrap().source = "missing".into();
    assert!(bad.validate().is_err());
    let mut bad = c.clone();
    bad.rules = 9;
    bad.schema = 9;
    assert!(bad.validate().is_err());
}
