use sim_core::*;
use std::collections::{BTreeMap, BTreeSet};
fn good(id: &str) -> GoodId {
    GoodId(id.into())
}
fn sector(id: &str) -> SectorId {
    SectorId(id.into())
}
fn capital(id: &str) -> CapitalKindId {
    CapitalKindId(id.into())
}
fn quantities(values: &[(&str, u64)]) -> BTreeMap<GoodId, ProductionQuantity> {
    values
        .iter()
        .map(|(id, q)| (good(id), ProductionQuantity(*q)))
        .collect()
}
fn definition() -> SectorProductionDefinition {
    let recipes = [
        (
            "extraction",
            IndustryKind::Extraction,
            1,
            "machines",
            vec![("energy", 1)],
            vec![("ore", 2)],
        ),
        (
            "agriculture",
            IndustryKind::Agriculture,
            2,
            "machines",
            vec![("energy", 2)],
            vec![("food", 3)],
        ),
        (
            "manufacturing",
            IndustryKind::Manufacturing,
            1,
            "machines",
            vec![("energy", 1), ("ore", 2)],
            vec![("manufactured", 1)],
        ),
        (
            "construction",
            IndustryKind::Construction,
            1,
            "tools",
            vec![("energy", 1), ("manufactured", 2)],
            vec![("construction", 1)],
        ),
        (
            "services",
            IndustryKind::Services,
            1,
            "machines",
            vec![("energy", 1)],
            vec![("services", 1)],
        ),
    ];
    SectorProductionDefinition {
        source: "fictional-sector-tests".into(),
        source_version: "1".into(),
        description:
            "Authored physical recipes; no historical production claims or monetary settlement."
                .into(),
        goods: [
            "energy",
            "ore",
            "food",
            "manufactured",
            "construction",
            "services",
        ]
        .into_iter()
        .map(|id| {
            (
                good(id),
                ProductionGood {
                    name: id.into(),
                    unit: format!("authored {id} unit"),
                },
            )
        })
        .collect(),
        capital_kinds: BTreeSet::from([capital("machines"), capital("tools")]),
        sectors: recipes
            .into_iter()
            .enumerate()
            .map(|(i, (id, industry, labor, kind, inputs, outputs))| {
                (
                    sector(id),
                    SectorRecipe {
                        name: id.into(),
                        industry,
                        priority: i as u16,
                        labor_per_batch: LaborDays(labor),
                        capital_per_batch: BTreeMap::from([(capital(kind), CapitalServices(1))]),
                        inputs_per_batch: quantities(&inputs),
                        outputs_per_batch: quantities(&outputs),
                    },
                )
            })
            .collect(),
    }
}
fn capacity(d: &SectorProductionDefinition) -> BTreeMap<SectorId, SectorCapacity> {
    d.sectors
        .keys()
        .map(|id| {
            (
                id.clone(),
                SectorCapacity {
                    installed_batches: ProductionBatches(10),
                    requested_batches: ProductionBatches(3),
                },
            )
        })
        .collect()
}
fn stocks(d: &SectorProductionDefinition) -> BTreeMap<GoodId, ProductionQuantity> {
    d.goods
        .keys()
        .map(|id| (id.clone(), ProductionQuantity(100)))
        .collect()
}
fn machines() -> BTreeMap<CapitalKindId, CapitalServices> {
    BTreeMap::from([
        (capital("machines"), CapitalServices(100)),
        (capital("tools"), CapitalServices(100)),
    ])
}
#[test]
fn five_industries_share_one_framework_and_reconcile_each_good_and_factor_separately() {
    let d = definition();
    let opening = stocks(&d);
    let plan = d
        .plan_day(
            &capacity(&d),
            &opening,
            Population(100),
            LaborDays(100),
            &machines(),
        )
        .unwrap();
    assert_eq!(plan.sectors.len(), 5);
    assert!(
        plan.sectors
            .iter()
            .all(|s| s.produced == ProductionBatches(3))
    );
    assert_eq!(plan.goods[&good("energy")].closing, ProductionQuantity(82));
    assert_eq!(plan.goods[&good("ore")].closing, ProductionQuantity(100));
    assert_eq!(
        plan.goods[&good("manufactured")].closing,
        ProductionQuantity(97)
    );
    assert_eq!(plan.goods[&good("food")].closing, ProductionQuantity(109));
    assert_eq!(plan.labor_unused, LaborDays(82));
    assert_eq!(
        plan.capital_unused[&capital("machines")],
        CapitalServices(88)
    );
    assert_eq!(plan.capital_unused[&capital("tools")], CapitalServices(97));
    for flow in plan.goods.values() {
        assert_eq!(
            flow.opening.0 - flow.consumed.0 + flow.produced.0,
            flow.closing.0
        );
    }
    assert_eq!(opening, stocks(&d));
    let decoded: ProductionDay =
        serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();
    assert_eq!(decoded, plan);
    d.validate_day(&decoded, &capacity(&d), Population(100))
        .unwrap();
    let mut forged = decoded.clone();
    forged.goods.get_mut(&good("food")).unwrap().closing.0 += 1;
    assert!(
        d.validate_day(&forged, &capacity(&d), Population(100))
            .is_err()
    );
    let mut reordered = decoded.clone();
    reordered.sectors.reverse();
    assert!(
        d.validate_day(&reordered, &capacity(&d), Population(100))
            .is_err()
    );
}
#[test]
fn explicit_priority_and_stable_ids_allocate_scarce_inputs_without_name_based_rules() {
    let mut d = definition();
    for recipe in d.sectors.values_mut() {
        recipe.priority = 1;
    }
    d.sectors.get_mut(&sector("agriculture")).unwrap().priority = 0;
    let mut opening = stocks(&d);
    opening.insert(good("energy"), ProductionQuantity(7));
    let plan = d
        .plan_day(
            &capacity(&d),
            &opening,
            Population(100),
            LaborDays(100),
            &machines(),
        )
        .unwrap();
    assert_eq!(plan.sectors[0].sector, sector("agriculture"));
    assert_eq!(plan.sectors[0].produced, ProductionBatches(3));
    // The tied construction ID precedes extraction and receives the final unit.
    assert_eq!(plan.sectors[1].sector, sector("construction"));
    assert_eq!(plan.sectors[1].produced, ProductionBatches(1));
    assert!(
        plan.sectors[1].limits.iter().any(|limit| limit.binding
            && limit.constraint == ProductionConstraint::Input(good("energy")))
    );
    assert_eq!(plan.goods[&good("energy")].closing, ProductionQuantity(0));
    let mut renamed = d.clone();
    for recipe in renamed.sectors.values_mut() {
        recipe.name = "A translated sector".into();
    }
    for g in renamed.goods.values_mut() {
        g.name = "A translated commodity".into();
    }
    assert_eq!(
        renamed
            .plan_day(
                &capacity(&d),
                &opening,
                Population(100),
                LaborDays(100),
                &machines()
            )
            .unwrap(),
        plan
    );
}
#[test]
fn labor_shared_capital_and_installed_capacity_cannot_be_double_booked() {
    let d = definition();
    let mut installed = capacity(&d);
    installed
        .get_mut(&sector("extraction"))
        .unwrap()
        .installed_batches = ProductionBatches(1);
    let mut capital = machines();
    capital.insert(CapitalKindId("machines".into()), CapitalServices(2));
    let plan = d
        .plan_day(
            &installed,
            &stocks(&d),
            Population(3),
            LaborDays(3),
            &capital,
        )
        .unwrap();
    assert_eq!(plan.sectors[0].produced, ProductionBatches(1));
    assert!(
        plan.sectors[0]
            .limits
            .iter()
            .any(|l| l.binding && l.constraint == ProductionConstraint::InstalledCapacity)
    );
    assert_eq!(plan.sectors[1].produced, ProductionBatches(1));
    assert!(
        plan.sectors[1]
            .limits
            .iter()
            .any(|l| l.binding && l.constraint == ProductionConstraint::Labor)
    );
    assert!(plan.sectors[1].limits.iter().any(|l| l.binding
        && l.constraint == ProductionConstraint::Capital(CapitalKindId("machines".into()))));
    assert!(
        plan.sectors[2..]
            .iter()
            .all(|s| s.produced == ProductionBatches(0))
    );
    assert_eq!(plan.labor_unused, LaborDays(0));
    assert_eq!(
        plan.capital_unused[&CapitalKindId("machines".into())],
        CapitalServices(0)
    );
}
#[test]
fn outputs_only_feed_the_following_day_and_empty_cycles_cannot_bootstrap_goods() {
    let mut d = definition();
    d.sectors
        .retain(|id, _| [sector("extraction"), sector("manufacturing")].contains(id));
    let mut opening = stocks(&d);
    opening.insert(good("ore"), ProductionQuantity(0));
    let first = d
        .plan_day(
            &capacity(&d),
            &opening,
            Population(100),
            LaborDays(100),
            &machines(),
        )
        .unwrap();
    assert_eq!(first.sectors[0].produced, ProductionBatches(3));
    assert_eq!(first.sectors[1].produced, ProductionBatches(0));
    let next = first
        .goods
        .iter()
        .map(|(id, flow)| (id.clone(), flow.closing))
        .collect();
    let second = d
        .plan_day(
            &capacity(&d),
            &next,
            Population(100),
            LaborDays(100),
            &machines(),
        )
        .unwrap();
    assert_eq!(second.sectors[1].produced, ProductionBatches(3));
    d.sectors
        .get_mut(&sector("extraction"))
        .unwrap()
        .inputs_per_batch = quantities(&[("manufactured", 1)]);
    let empty = d
        .goods
        .keys()
        .map(|id| (id.clone(), ProductionQuantity(0)))
        .collect();
    let circular = d
        .plan_day(
            &capacity(&d),
            &empty,
            Population(100),
            LaborDays(100),
            &machines(),
        )
        .unwrap();
    assert!(
        circular
            .sectors
            .iter()
            .all(|s| s.produced == ProductionBatches(0))
    );
    assert!(
        circular
            .goods
            .values()
            .all(|f| f.closing == ProductionQuantity(0))
    );
}
#[test]
fn malformed_recipes_coverage_labor_and_overflow_fail_without_changing_inputs() {
    let d = definition();
    let opening = stocks(&d);
    let original = opening.clone();
    assert!(
        d.plan_day(
            &capacity(&d),
            &opening,
            Population(2),
            LaborDays(3),
            &machines()
        )
        .is_err()
    );
    let mut missing = opening.clone();
    missing.remove(&good("food"));
    assert!(
        d.plan_day(
            &capacity(&d),
            &missing,
            Population(100),
            LaborDays(100),
            &machines()
        )
        .is_err()
    );
    for variant in 0..5 {
        let mut invalid = d.clone();
        let recipe = invalid.sectors.get_mut(&sector("agriculture")).unwrap();
        match variant {
            0 => {
                recipe
                    .inputs_per_batch
                    .insert(good("unknown"), ProductionQuantity(1));
            }
            1 => {
                recipe
                    .inputs_per_batch
                    .insert(good("energy"), ProductionQuantity(0));
            }
            2 => {
                recipe.outputs_per_batch.clear();
            }
            3 => {
                recipe.capital_per_batch.clear();
                recipe.inputs_per_batch.clear();
                recipe.labor_per_batch = LaborDays(0);
            }
            4 => {
                recipe
                    .capital_per_batch
                    .insert(capital("unknown"), CapitalServices(1));
            }
            _ => unreachable!(),
        }
        assert!(invalid.validate().is_err(), "variant {variant}");
    }
    let mut overflow = d.clone();
    overflow
        .sectors
        .get_mut(&sector("extraction"))
        .unwrap()
        .outputs_per_batch
        .insert(good("ore"), ProductionQuantity(u64::MAX));
    assert!(
        overflow
            .plan_day(
                &capacity(&d),
                &opening,
                Population(100),
                LaborDays(100),
                &machines()
            )
            .unwrap_err()
            .contains("overflow")
    );
    let mut overflow_stock = opening.clone();
    overflow_stock.insert(good("food"), ProductionQuantity(u64::MAX));
    assert!(
        d.plan_day(
            &capacity(&d),
            &overflow_stock,
            Population(100),
            LaborDays(100),
            &machines()
        )
        .unwrap_err()
        .contains("overflow")
    );
    assert_eq!(opening, original);
}
