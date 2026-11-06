//! Authored representative-resident bands on private income actually received.
//! Potential tax is an estimate; only withholding transfers money to treasury.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Flat-rule withholding keeps both rate factors until cash is rounded once.
pub(crate) fn flat_withholding(gross: Money, standard_rate: i32, administration: i32) -> Money {
    MoneyScale::ONE
        .percent(Percent::try_from(standard_rate).expect("validated standard tax rate"))
        .and_then(|scale| {
            scale.percent(Percent::try_from(administration).expect("validated collection capacity"))
        })
        .and_then(|scale| scale.apply(gross))
        .expect("bounded flat income withholding")
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct IncomeTaxBandId(pub String);
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IncomeTaxBand {
    pub id: IncomeTaxBandId,
    /// Annual private income relative to opening GDP per resident, in basis points.
    pub lower_gdp_per_resident_bps: u32,
    /// Factor applied to the operating income_tax standard rate; 10000 means 1x.
    pub rate_multiplier_bps: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IncomeTaxDefinition {
    pub annualisation_days: DaySpan,
    pub bands: Vec<IncomeTaxBand>,
    pub source: String,
    pub source_version: String,
}
impl IncomeTaxDefinition {
    fn validate_shape(&self) -> Result<(), String> {
        let mut ids = BTreeSet::new();
        if self.annualisation_days.units() != 365
            || !(2..=12).contains(&self.bands.len())
            || self
                .bands
                .first()
                .is_none_or(|b| b.lower_gdp_per_resident_bps != 0)
            || self.bands.iter().any(|b| {
                b.id.0.trim().is_empty()
                    || b.id.0.len() > 64
                    || !ids.insert(&b.id)
                    || b.lower_gdp_per_resident_bps > 1_000_000
                    || b.rate_multiplier_bps > 20_000
            })
            || self.bands.windows(2).any(|b| {
                b[0].lower_gdp_per_resident_bps >= b[1].lower_gdp_per_resident_bps
                    || b[0].rate_multiplier_bps > b[1].rate_multiplier_bps
            })
            || self.bands.iter().all(|b| b.rate_multiplier_bps == 0)
        {
            return Err("Income bands need unique IDs, complete ordered coverage, progressive bounded marginal factors and 365-day annualisation".into());
        }
        Ok(())
    }
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        self.validate_shape()?;
        if content.commitments.is_none()
            || self.source_version.is_empty()
            || !content.sources.iter().any(|s| s.id == self.source)
            || content
                .policy(&"income_tax".into())
                .is_none_or(|p| p.min < 10 || p.max > 45)
        {
            return Err("Income bands require policy delivery, supported standard rates and attributed provenance".into());
        }
        Ok(())
    }
    pub fn marginal_rate_bps(
        &self,
        band: &IncomeTaxBand,
        standard_rate: i32,
    ) -> Result<u32, String> {
        if !(10..=45).contains(&standard_rate) || band.rate_multiplier_bps > 20_000 {
            return Err("Invalid standard rate or marginal multiplier".into());
        }
        Ok(standard_rate as u32 * 100 * band.rate_multiplier_bps / 10_000)
    }
    /// A representative resident within each cohort, not an intra-cohort distribution.
    /// All slices use the cohort's aggregate annualised income, keeping money integral.
    pub fn assess(
        &self,
        d: &CountryDefinition,
        group_population: Population,
        gross: Money,
        standard_rate: i32,
        administration: i32,
    ) -> Result<IncomeTaxAssessment, String> {
        self.validate_shape()?;
        if !(0..=1_000_000_000_000).contains(&gross.0)
            || group_population.0 == 0
            || group_population.0 > d.population.0
            || d.population.0 == 0
            || !(1..=100_000_000).contains(&d.gdp)
            || !(1..=100).contains(&administration)
            || !(10..=45).contains(&standard_rate)
        {
            return Err("Income assessment requires bounded received income, population, GDP, standard rate and collection capacity".into());
        }
        let annual = i128::from(gross.0) * i128::from(self.annualisation_days.units());
        let threshold = |bps: u32| {
            i128::from(d.gdp) * i128::from(group_population.0) * i128::from(bps)
                / (i128::from(d.population.0) * 10_000)
        };
        let relative = annual * i128::from(d.population.0) * 10_000
            / (i128::from(d.gdp) * i128::from(group_population.0));
        let relative = u64::try_from(relative)
            .map_err(|_| "Annualised representative income ratio exceeds supported precision")?;
        let mut numerator = 0_i128;
        let mut slices = Vec::new();
        for (index, b) in self.bands.iter().enumerate() {
            let lower = threshold(b.lower_gdp_per_resident_bps);
            let upper = self
                .bands
                .get(index + 1)
                .map_or(annual, |b| threshold(b.lower_gdp_per_resident_bps));
            let amount = (annual.min(upper) - lower).max(0);
            if amount > 0 {
                let rate = self.marginal_rate_bps(b, standard_rate)?;
                numerator += amount * i128::from(rate);
                slices.push(IncomeTaxSlice {
                    band: b.id.clone(),
                    annual_base: Money(
                        i64::try_from(amount)
                            .map_err(|_| "Annual tax basis exceeds supported precision")?,
                    ),
                    marginal_rate_bps: rate,
                });
            }
        }
        let denominator = i128::from(self.annualisation_days.units()) * 10_000;
        let potential = i64::try_from(numerator / denominator)
            .map_err(|_| "Potential withholding exceeds supported precision")?;
        let collectible =
            i64::try_from(numerator * i128::from(administration) / (denominator * 100))
                .map_err(|_| "Collection estimate exceeds supported precision")?;
        Ok(IncomeTaxAssessment {
            gross,
            annualised_reference_bps: relative,
            potential: Money(potential),
            collectible: Money(collectible),
            slices,
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IncomeTaxSlice {
    pub band: IncomeTaxBandId,
    /// Annualised aggregate private-income basis; this is not a cash payment.
    pub annual_base: Money,
    pub marginal_rate_bps: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IncomeTaxAssessment {
    pub gross: Money,
    pub annualised_reference_bps: u64,
    pub potential: Money,
    pub collectible: Money,
    pub slices: Vec<IncomeTaxSlice>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IncomeTaxWithholding {
    pub group: HouseholdGroupId,
    pub assessment: IncomeTaxAssessment,
    pub paid: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct IncomeTaxReport {
    pub day: u32,
    pub standard_rate: i32,
    pub administration: i32,
    pub groups: Vec<IncomeTaxWithholding>,
}
impl IncomeTaxReport {
    pub fn potential(&self) -> Money {
        Money(self.groups.iter().map(|g| g.assessment.potential.0).sum())
    }
    pub fn paid(&self) -> Money {
        Money(self.groups.iter().map(|g| g.paid.0).sum())
    }
}
impl World {
    /// Conditional private-income assessment, holding supply, funding and administration fixed.
    pub fn preview_income_taxes(
        &self,
        content: &Content,
        id: &CountryId,
        standard_rate: i32,
    ) -> Result<Vec<(HouseholdGroupId, IncomeTaxAssessment)>, String> {
        let taxes = content
            .income_taxes
            .as_ref()
            .ok_or("This campaign uses the earlier flat private-income model")?;
        let c = self.countries.get(id).ok_or("Unknown country")?;
        let d = content.country(id).ok_or("Missing country definition")?;
        c.forecast_private_income(d)
            .iter()
            .zip(&c.households)
            .map(|(amount, h)| {
                Ok((
                    h.id.clone(),
                    taxes.assess(
                        d,
                        h.population,
                        Money(*amount),
                        standard_rate,
                        c.administration,
                    )?,
                ))
            })
            .collect()
    }
    pub(crate) fn validate_income_taxes(
        &self,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let c = &self.countries[id];
        let Some(rules) = &content.income_taxes else {
            return if c.income_tax_report.is_none() {
                Ok(())
            } else {
                Err("Income-band reports require the campaign's income-tax definition".into())
            };
        };
        if self.day == content.start_day()? {
            return if c.income_tax_report.is_none() {
                Ok(())
            } else {
                Err("Opening snapshots have no realized income tax".into())
            };
        }
        let report = c
            .income_tax_report
            .as_ref()
            .ok_or("Missing settled income-tax report")?;
        let d = content.country(id).ok_or("Unknown country")?;
        if report.day != self.day
            || report.groups.len() != d.households.len()
            || c.households.len() != d.households.len()
            || report.standard_rate
                != self.policy_level_at(
                    content,
                    id,
                    &"income_tax".into(),
                    self.day,
                    CommandId(0),
                )?
            || report.administration
                != self.policy_level_at(
                    content,
                    id,
                    &"administration".into(),
                    self.day,
                    CommandId(0),
                )?
        {
            return Err(
                "Invalid income-tax report date, cohort partition or settled policy settings"
                    .into(),
            );
        }
        let mut gross = 0_i128;
        let mut paid = 0_i128;
        for (line, h) in report.groups.iter().zip(&c.households) {
            let expected = rules.assess(
                d,
                h.population,
                line.assessment.gross,
                report.standard_rate,
                report.administration,
            )?;
            if line.group != h.id
                || line.assessment != expected
                || line.paid != expected.collectible
                || line.paid != h.tax
            {
                return Err("Income-band basis, marginal slices or collected cash disagree with household withholding".into());
            }
            gross += i128::from(line.assessment.gross.0);
            paid += i128::from(line.paid.0);
        }
        if gross != i128::from(c.report.wages.0) || paid != i128::from(c.report.revenue.0) {
            return Err(
                "Private income and collected revenue do not reconcile to tax reporting".into(),
            );
        }
        Ok(())
    }
}
