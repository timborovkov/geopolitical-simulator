use sim_core::*;
use std::collections::{BTreeMap, BTreeSet};
fn grain() -> GoodId {
    GoodId("grain".into())
}
fn farm() -> SectorId {
    SectorId("farm".into())
}
fn quantity(q: u64) -> BTreeMap<GoodId, ProductionQuantity> {
    BTreeMap::from([(grain(), ProductionQuantity(q))])
}
fn fixture() -> (SectorProductionDefinition, ProductionDayRequest) {
    let definition = SectorProductionDefinition {
        source: "fictional-inventory-tests".into(),
        source_version: "1".into(),
        description: "Authored test recipe and stocks; not a national production observation."
            .into(),
        goods: BTreeMap::from([(
            grain(),
            ProductionGood {
                name: "Test grain".into(),
                unit: "authored grain unit".into(),
            },
        )]),
        capital_kinds: BTreeSet::new(),
        sectors: BTreeMap::from([(
            farm(),
            SectorRecipe {
                name: "Test farm".into(),
                industry: IndustryKind::Agriculture,
                priority: 0,
                labor_per_batch: LaborDays(1),
                capital_per_batch: BTreeMap::new(),
                inputs_per_batch: BTreeMap::new(),
                outputs_per_batch: quantity(1),
            },
        )]),
    };
    let request = ProductionDayRequest {
        capacities: BTreeMap::from([(
            farm(),
            SectorCapacity {
                installed_batches: ProductionBatches(1),
                requested_batches: ProductionBatches(1),
            },
        )]),
        workers: Population(1),
        labor: LaborDays(1),
        capital: BTreeMap::new(),
        imports: quantity(0),
        exports: quantity(0),
        final_use: quantity(1),
    };
    (definition, request)
}
fn storage(capacity: u64, loss: u32) -> BTreeMap<GoodId, ProductionStorageRule> {
    BTreeMap::from([(
        grain(),
        ProductionStorageRule {
            capacity: ProductionQuantity(capacity),
            loss_bps: BasisPoints::new(loss),
        },
    )])
}
#[test]
fn final_use_capacity_disposal_and_spoilage_reconcile_to_opening_and_output() {
    let (definition, request) = fixture();
    let opening =
        ProductionInventory::new(&definition, CampaignDay::new(561), quantity(3)).unwrap();
    let (next, receipt) = opening
        .apply_day(&definition, &storage(2, 5000), &request)
        .unwrap();
    assert_eq!(next.day, CampaignDay::new(562));
    let flow = &receipt.storage[&grain()];
    assert_eq!(
        (
            flow.after_production.0,
            flow.final_use.0,
            flow.discarded.0,
            flow.spoiled.0,
            flow.closing.0
        ),
        (4, 1, 1, 1, 1)
    );
    let totals = &next.positions[&grain()].totals;
    assert_eq!(
        (
            totals.produced.0,
            totals.final_use.0,
            totals.discarded.0,
            totals.spoiled.0
        ),
        (1, 1, 1, 1)
    );
    assert_eq!(opening.positions[&grain()].quantity, ProductionQuantity(3));
    next.validate(&definition).unwrap();
}
#[test]
fn small_stocks_carry_fractional_spoilage_and_empty_positions_clear_the_carry() {
    let (definition, mut request) = fixture();
    request
        .capacities
        .get_mut(&farm())
        .unwrap()
        .requested_batches = ProductionBatches(0);
    request.final_use = quantity(0);
    let mut inventory =
        ProductionInventory::new(&definition, CampaignDay::new(0), quantity(3)).unwrap();
    for expected in [3000, 6000, 9000, 2000] {
        inventory = inventory
            .apply_day(&definition, &storage(10, 1000), &request)
            .unwrap()
            .0;
        assert_eq!(inventory.positions[&grain()].loss_carry, expected);
    }
    assert_eq!(
        inventory.positions[&grain()].quantity,
        ProductionQuantity(2)
    );
    assert_eq!(
        inventory.positions[&grain()].totals.spoiled,
        ProductionQuantity(1)
    );
    request.final_use = quantity(2);
    inventory = inventory
        .apply_day(&definition, &storage(10, 1000), &request)
        .unwrap()
        .0;
    assert_eq!(inventory.positions[&grain()].loss_carry, 0);
    assert_eq!(
        inventory.positions[&grain()].quantity,
        ProductionQuantity(0)
    );
    assert_eq!(
        inventory.positions[&grain()].totals.final_use,
        ProductionQuantity(2)
    );
}
#[test]
fn zero_storage_services_can_be_used_today_and_unused_output_is_not_stored() {
    let (mut definition, mut request) = fixture();
    definition.sectors.get_mut(&farm()).unwrap().industry = IndustryKind::Services;
    let inventory =
        ProductionInventory::new(&definition, CampaignDay::new(0), quantity(0)).unwrap();
    let consumed = inventory
        .apply_day(&definition, &storage(0, 0), &request)
        .unwrap()
        .0;
    assert_eq!(
        consumed.positions[&grain()].totals.final_use,
        ProductionQuantity(1)
    );
    assert_eq!(
        consumed.positions[&grain()].totals.discarded,
        ProductionQuantity(0)
    );
    assert_eq!(consumed.positions[&grain()].quantity, ProductionQuantity(0));
    request.final_use = quantity(0);
    let unused = inventory
        .apply_day(&definition, &storage(0, 0), &request)
        .unwrap()
        .0;
    assert_eq!(
        unused.positions[&grain()].totals.discarded,
        ProductionQuantity(1)
    );
    assert_eq!(unused.positions[&grain()].quantity, ProductionQuantity(0));
}
#[test]
fn save_resume_retains_fractional_losses_and_cumulative_flows_on_actual_dates() {
    let (definition, request) = fixture();
    let rules = storage(10, 175);
    let mut direct =
        ProductionInventory::new(&definition, CampaignDay::new(561), quantity(7)).unwrap();
    for _ in 0..17 {
        direct = direct.apply_day(&definition, &rules, &request).unwrap().0;
    }
    let mut resumed: ProductionInventory =
        serde_json::from_str(&serde_json::to_string(&direct).unwrap()).unwrap();
    resumed.validate(&definition).unwrap();
    for _ in 0..90 {
        let (next, receipt) = direct.apply_day(&definition, &rules, &request).unwrap();
        let (restored, same_receipt) = resumed.apply_day(&definition, &rules, &request).unwrap();
        assert_eq!(receipt, same_receipt);
        assert_eq!(next, restored);
        direct = next;
        resumed = restored;
    }
    assert_eq!(direct.day, CampaignDay::new(668));
    assert_eq!(
        direct.positions[&grain()].totals.produced,
        ProductionQuantity(107)
    );
    assert_eq!(
        direct.positions[&grain()].totals.final_use,
        ProductionQuantity(107)
    );
    assert_eq!(
        direct.positions[&grain()].quantity.0 + direct.positions[&grain()].totals.spoiled.0,
        7
    );
}
#[test]
fn explicit_trade_receipts_reconcile_and_new_imports_only_feed_later_production() {
    let (mut definition, mut request) = fixture();
    definition
        .sectors
        .get_mut(&farm())
        .unwrap()
        .inputs_per_batch = quantity(1);
    let inventory =
        ProductionInventory::new(&definition, CampaignDay::new(0), quantity(0)).unwrap();
    request.imports = quantity(5);
    request.exports = quantity(1);
    request.final_use = quantity(1);
    let (first, receipt) = inventory
        .apply_day(&definition, &storage(10, 0), &request)
        .unwrap();
    assert_eq!(receipt.production.sectors[0].produced, ProductionBatches(0));
    assert_eq!(first.positions[&grain()].quantity, ProductionQuantity(3));
    assert_eq!(
        first.positions[&grain()].totals.imported,
        ProductionQuantity(5)
    );
    assert_eq!(
        first.positions[&grain()].totals.exported,
        ProductionQuantity(1)
    );
    request.imports = quantity(0);
    request.exports = quantity(0);
    let (second, receipt) = first
        .apply_day(&definition, &storage(10, 0), &request)
        .unwrap();
    assert_eq!(receipt.production.sectors[0].produced, ProductionBatches(1));
    assert_eq!(second.positions[&grain()].quantity, ProductionQuantity(2));
    assert_eq!(
        second.positions[&grain()].totals.industrial_use,
        ProductionQuantity(1)
    );
    second.validate(&definition).unwrap();
}
#[test]
fn post_storage_capital_use_debits_actual_goods_without_advancing_the_clock() {
    let (definition, request) = fixture();
    let opening =
        ProductionInventory::new(&definition, CampaignDay::new(561), quantity(3)).unwrap();
    assert!(opening.withdraw_final(&definition, &quantity(1)).is_err());
    let settled = opening
        .apply_day(&definition, &storage(10, 1000), &request)
        .unwrap()
        .0;
    let before = settled.clone();
    let delivered = settled.withdraw_final(&definition, &quantity(3)).unwrap();
    assert_eq!(delivered.day, CampaignDay::new(562));
    assert_eq!(
        delivered.positions[&grain()].quantity,
        ProductionQuantity(0)
    );
    assert_eq!(delivered.positions[&grain()].loss_carry, 0);
    assert_eq!(
        delivered.positions[&grain()].totals.final_use,
        ProductionQuantity(4)
    );
    assert!(settled.withdraw_final(&definition, &quantity(4)).is_err());
    assert_eq!(settled, before);
    delivered.validate(&definition).unwrap();
}
#[test]
fn malformed_positions_overspending_calendar_and_overflow_fail_without_advancing() {
    let (definition, mut request) = fixture();
    let inventory =
        ProductionInventory::new(&definition, CampaignDay::new(561), quantity(3)).unwrap();
    let before = inventory.clone();
    request.final_use = quantity(5);
    assert!(
        inventory
            .apply_day(&definition, &storage(10, 0), &request)
            .unwrap_err()
            .contains("Final use exceeds")
    );
    request.final_use = quantity(1);
    request.exports = quantity(5);
    assert!(
        inventory
            .apply_day(&definition, &storage(10, 0), &request)
            .unwrap_err()
            .contains("Exports exceed")
    );
    request.exports = quantity(0);
    request.imports = quantity(u64::MAX);
    assert!(
        inventory
            .apply_day(&definition, &storage(10, 0), &request)
            .unwrap_err()
            .contains("overflow")
    );
    request.imports = quantity(0);
    assert!(
        inventory
            .apply_day(&definition, &storage(10, 10001), &request)
            .is_err()
    );
    assert!(
        inventory
            .apply_day(&definition, &BTreeMap::new(), &request)
            .is_err()
    );
    for variant in 0..4 {
        let mut forged = inventory.clone();
        let position = forged.positions.get_mut(&grain()).unwrap();
        match variant {
            0 => position.quantity = ProductionQuantity(4),
            1 => position.loss_carry = 10000,
            2 => {
                position.quantity = ProductionQuantity(0);
                position.totals.final_use = ProductionQuantity(3);
                position.loss_carry = 1;
            }
            3 => {
                forged.positions.remove(&grain());
            }
            _ => unreachable!(),
        }
        assert!(forged.validate(&definition).is_err(), "variant {variant}");
    }
    let exhausted =
        ProductionInventory::new(&definition, CampaignDay::new(MAX_DAY), quantity(3)).unwrap();
    assert!(
        exhausted
            .apply_day(&definition, &storage(10, 0), &request)
            .unwrap_err()
            .contains("calendar")
    );
    // Reach the counter limit through an actual balanced conversion, rather
    // than accepting an invented cumulative flow at the opening bookmark.
    let mut large = definition.clone();
    let recipe = large.sectors.get_mut(&farm()).unwrap();
    recipe.inputs_per_batch = quantity(3);
    recipe.outputs_per_batch = quantity(u64::MAX);
    request.final_use = quantity(u64::MAX - 3);
    let counter = ProductionInventory::new(&large, CampaignDay::new(0), quantity(3))
        .unwrap()
        .apply_day(&large, &storage(u64::MAX, 0), &request)
        .unwrap()
        .0;
    counter.validate(&large).unwrap();
    assert!(
        counter
            .apply_day(&large, &storage(u64::MAX, 0), &request)
            .unwrap_err()
            .contains("overflow")
    );
    let mut invented_opening = inventory.clone();
    let p = invented_opening.positions.get_mut(&grain()).unwrap();
    p.totals.produced = ProductionQuantity(1);
    p.totals.final_use = ProductionQuantity(1);
    assert!(invented_opening.validate(&definition).is_err());
    let mut future_origin = inventory.clone();
    future_origin.start_day = CampaignDay::new(562);
    assert!(future_origin.validate(&definition).is_err());
    assert_eq!(inventory, before);
    let mut different_units = definition.clone();
    different_units.goods.get_mut(&grain()).unwrap().unit = "different physical units".into();
    assert!(inventory.validate(&different_units).is_err());
    let mut different_recipe = definition.clone();
    different_recipe
        .sectors
        .get_mut(&farm())
        .unwrap()
        .outputs_per_batch = quantity(2);
    assert!(inventory.validate(&different_recipe).is_err());
}
