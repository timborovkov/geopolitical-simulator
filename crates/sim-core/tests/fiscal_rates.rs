use sim_core::*;

#[test]
fn flat_tax_and_spending_forecasts_match_funded_settlement_at_rate_boundaries() {
    for gdp in [7000, 26_000] {
        for tax in [10, 28, 45] {
            for administration in [1, 90, 100] {
                let mut content =
                    Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
                for definition in &mut content.countries {
                    definition.gdp = gdp;
                    definition.tax = tax;
                    definition.capacity = administration;
                    definition.services = 18;
                    definition.welfare = 12;
                    definition.energy = 120;
                    definition.food = 120;
                }
                content.validate().unwrap();
                let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
                let definition = content.country(&world.player).unwrap();
                let forecast = world.player_country().forecast(definition, None);
                world.advance(&content, sim_core::DaySpan::new(1)).unwrap();
                let country = world.player_country();
                assert_eq!(
                    country.report.revenue, forecast.daily_revenue,
                    "GDP {gdp}, tax {tax}, administration {administration}"
                );
                assert_eq!(
                    country.budget.payments,
                    Money(forecast.daily_spending.0 + forecast.daily_interest.0)
                );
                assert_eq!(country.report.interest, forecast.daily_interest);
                assert_eq!(country.arrears, Money(0));
                assert_eq!(
                    country
                        .households
                        .iter()
                        .map(|group| group.tax.0)
                        .sum::<i64>(),
                    country.report.revenue.0
                );
                world.validate(&content).unwrap();
                assert_eq!(persistence::replay(&world, &content).unwrap(), world);
            }
        }
    }
}

#[test]
fn invalid_stored_policy_rates_fail_validation_before_typed_calculations() {
    for content in [
        Content::classic_bundled().unwrap(),
        Content::bundled().unwrap(),
    ] {
        let world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        for field in ["tax", "administration", "services", "welfare"] {
            let mut invalid = world.clone();
            let country = invalid.countries.get_mut(&world.player).unwrap();
            match field {
                "tax" => country.tax = -1,
                "administration" => country.administration = -1,
                "services" => country.services = -1,
                "welfare" => country.welfare = -1,
                _ => unreachable!(),
            }
            assert!(invalid.validate(&content).is_err());
            assert!(persistence::encode(&invalid, &content).is_err());
        }
    }
}
