use sim_core::*;
fn fixture() -> Content {
    Content::from_json(include_str!(
        "../../../data/synthetic-sector-production-test.json"
    ))
    .unwrap()
}
#[test]
fn owner_material_planning_and_public_definitions_are_read_only_and_respect_viewers() {
    let content = fixture();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.advance(&content, DaySpan::new(2)).unwrap();
    let before = persistence::encode(&world, &content).unwrap();
    let owner = Viewer::Government(world.player.clone());
    let plan = world
        .construction_material_plan(&content, owner.clone(), &world.player, &"energy".into())
        .unwrap()
        .unwrap();
    assert!(plan.required.0 > 0);
    let report = world
        .production_report(&content, owner.clone(), &world.player)
        .unwrap()
        .unwrap();
    assert_eq!(report.owned.as_ref().unwrap().inventory.day, world.day);
    for (viewer, country) in [
        (Viewer::Observer, world.player.clone()),
        (
            Viewer::Government(CountryId("BOR".into())),
            world.player.clone(),
        ),
        (owner.clone(), CountryId("BOR".into())),
        (
            Viewer::Government(CountryId("BOR".into())),
            CountryId("BOR".into()),
        ),
    ] {
        let public = world
            .production_report(&content, viewer.clone(), &country)
            .unwrap()
            .unwrap();
        assert!(public.owned.is_none());
        assert!(serde_json::to_value(public).unwrap().get("owned").is_none());
        assert!(
            world
                .construction_material_plan(&content, viewer, &country, &"energy".into())
                .is_err()
        );
    }
    assert!(
        world
            .construction_material_plan(
                &content,
                owner.clone(),
                &world.player,
                &"income_tax".into()
            )
            .unwrap()
            .is_none()
    );
    assert!(
        world
            .construction_material_plan(&content, owner, &world.player, &"missing".into())
            .is_err()
    );
    assert!(
        world
            .production_report(
                &Content::bundled().unwrap(),
                Viewer::Observer,
                &world.player
            )
            .is_err()
    );
    assert_eq!(before, persistence::encode(&world, &content).unwrap());
    let old = Content::bundled().unwrap();
    let old_world = World::new(&old, CountryId("GBR".into()), 17).unwrap();
    assert!(
        old_world
            .production_report(&old, Viewer::Observer, &old_world.player)
            .unwrap()
            .is_none()
    );
}
#[test]
fn malformed_stocks_funding_capacity_carries_and_unauthorized_trade_cannot_advance() {
    let content = fixture();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.advance(&content, DaySpan::new(2)).unwrap();
    let id = world.player.clone();
    for variant in 0..7 {
        let mut forged = world.clone();
        let state = forged
            .production
            .as_mut()
            .unwrap()
            .countries
            .get_mut(&id)
            .unwrap();
        match variant {
            0 => state.inventory.definition_hash ^= 1,
            1 => {
                state
                    .inventory
                    .positions
                    .get_mut(&GoodId("biomass".into()))
                    .unwrap()
                    .quantity
                    .0 += 1
            }
            2 => {
                state
                    .inventory
                    .positions
                    .get_mut(&GoodId("biomass".into()))
                    .unwrap()
                    .loss_carry = 10000
            }
            3 => state.last.as_mut().unwrap().funding.private_income_paid.0 += 1,
            4 => {
                state
                    .last
                    .as_mut()
                    .unwrap()
                    .request
                    .capacities
                    .get_mut(&SectorId("construction".into()))
                    .unwrap()
                    .installed_batches
                    .0 += 1
            }
            5 => {
                let stock = state
                    .inventory
                    .positions
                    .get_mut(&GoodId("biomass".into()))
                    .unwrap();
                stock.quantity.0 += 1;
                stock.totals.imported.0 += 1;
            }
            6 => state
                .last
                .as_mut()
                .unwrap()
                .capital_uses
                .push(CapitalMaterialUse {
                    commitment: CommitmentId(999999),
                    policy: "energy".into(),
                    quantity: ProductionQuantity(100),
                }),
            _ => unreachable!(),
        }
        assert!(forged.validate(&content).is_err(), "variant {variant}");
        let before = forged.clone();
        assert!(
            forged.advance(&content, DaySpan::new(1)).is_err(),
            "variant {variant}"
        );
        assert_eq!(forged, before, "variant {variant}");
    }
}
#[test]
fn legitimate_cumulative_counter_exhaustion_rolls_back_the_entire_world_day() {
    let mut content = fixture();
    let definition = content.production.as_mut().unwrap();
    definition
        .framework
        .sectors
        .get_mut(&SectorId("manufacturing".into()))
        .unwrap()
        .outputs_per_batch
        .insert(GoodId("tools".into()), ProductionQuantity(u64::MAX));
    for country in definition.countries.values_mut() {
        country
            .stocks
            .insert(GoodId("tools".into()), ProductionQuantity(0));
        country
            .storage
            .get_mut(&GoodId("tools".into()))
            .unwrap()
            .capacity = ProductionQuantity(0);
        country
            .installed
            .insert(SectorId("manufacturing".into()), ProductionBatches(1));
    }
    content.validate().unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.advance(&content, DaySpan::new(1)).unwrap();
    world.validate(&content).unwrap();
    assert_eq!(
        world.production.as_ref().unwrap().countries[&world.player]
            .inventory
            .positions[&GoodId("tools".into())]
            .totals
            .produced,
        ProductionQuantity(u64::MAX)
    );
    let before = world.clone();
    let error = world.advance(&content, DaySpan::new(1)).unwrap_err();
    assert!(error.contains("overflow"), "{error}");
    assert_eq!(world, before);
}
#[test]
fn positive_assets_and_public_payments_cannot_staff_the_network_without_paid_private_income() {
    let mut raw = serde_json::to_value(fixture()).unwrap();
    let country = raw["countries"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|country| country["id"] == "AUR")
        .unwrap();
    country["energy"] = 1.into();
    country["gdp"] = 5000.into();
    let energy = raw["markets"]["goods"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|good| good["kind"] == "energy")
        .unwrap();
    energy["import_limit_bps"] = 0.into();
    let content = Content::from_json(&raw.to_string()).unwrap();
    let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
    world.advance(&content, DaySpan::new(1)).unwrap();
    let last = world.production.as_ref().unwrap().countries[&world.player]
        .last
        .as_ref()
        .unwrap();
    assert_eq!(last.funding.private_income_paid, Money(0));
    assert!(last.funding.service_paid.0 > 0);
    assert_eq!(last.request.labor, LaborDays(0));
    assert!(
        last.physical
            .production
            .sectors
            .iter()
            .all(|sector| sector.produced == ProductionBatches(0))
    );
    assert_eq!(world.player_country().report.service_delivery, 0);
    world.validate(&content).unwrap();
}
