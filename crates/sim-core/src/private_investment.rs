//! Cash-funded private equipment, distinct from public assets and public input stocks.
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivateInvestmentDefinition {
    pub source: String,
    pub source_version: String,
    pub description: String,
    pub capital_cost_bps: BasisPoints,
    pub capacity_points: u32,
    pub installation_days: DaySpan,
    pub lifetime_days: DaySpan,
    pub maximum_payback_days: DaySpan,
    pub maximum_active_per_good: u32,
}
impl PrivateInvestmentDefinition {
    pub(crate) fn validate(&self, c: &Content) -> Result<(), String> {
        if self.description.is_empty()
            || self.source_version.is_empty()
            || !c.sources.iter().any(|s| s.id == self.source)
            || !(1..=1000).contains(&self.capital_cost_bps.units())
            || !(1..=20).contains(&self.capacity_points)
            || !(1..=365).contains(&self.installation_days.units())
            || !(365..=7300).contains(&self.lifetime_days.units())
            || self.maximum_payback_days.units() == 0
            || self.maximum_payback_days >= self.lifetime_days
            || !(1..=8).contains(&self.maximum_active_per_good)
        {
            return Err("Private equipment needs bounded, attributed acquisition and service-life assumptions".into());
        }
        Ok(())
    }
    pub fn cost(&self, c: &Content, id: &CountryId) -> Result<Money, String> {
        let gdp = c
            .country(id)
            .ok_or("Unknown equipment country")?
            .gdp
            .units();
        Ok(Money(
            i64::try_from(
                (i128::from(gdp) * i128::from(self.capital_cost_bps.units()) / 10000).max(1),
            )
            .map_err(|_| "Equipment price overflow")?,
        ))
    }
}
/// Stable equipment identity, distinct from a policy or acquisition date.
/// ```compile_fail
/// use sim_core::{PrivateEquipmentId, CampaignDay};
/// let equipment: PrivateEquipmentId = CampaignDay::new(30);
/// ```
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct PrivateEquipmentId(pub u32);
/// One delivered equipment lot. The bounded external equipment market can supply
/// one lot per country per 30-day review; it does not draw on public trade quotas.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivateEquipment {
    pub id: PrivateEquipmentId,
    pub good: GoodKind,
    pub acquired: CampaignDay,
    pub commissioned: CampaignDay,
    pub retired: CampaignDay,
    pub cost: Money,
}
impl PrivateEquipment {
    fn depreciation(&self, day: CampaignDay) -> Result<Money, String> {
        let life = self
            .retired
            .units()
            .checked_sub(self.commissioned.units())
            .filter(|n| *n > 0)
            .ok_or("Equipment service life must be positive")?;
        let age = day
            .units()
            .saturating_sub(self.commissioned.units())
            .min(life);
        Ok(Money(
            i64::try_from(i128::from(self.cost.0) * i128::from(age) / i128::from(life))
                .map_err(|_| "Equipment depreciation overflow")?,
        ))
    }
    fn active(&self, day: CampaignDay) -> bool {
        day >= self.commissioned && day < self.retired
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivateCapitalTotals {
    pub acquisitions: Money,
    pub depreciation: Money,
    pub net_assets: Money,
}
impl PrivateCapitalTotals {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if [self.acquisitions, self.depreciation, self.net_assets]
            .iter()
            .any(|x| !(0..=1_000_000_000_000_000_000).contains(&x.0))
            || i128::from(self.acquisitions.0) - i128::from(self.depreciation.0)
                != i128::from(self.net_assets.0)
        {
            return Err("Private fixed assets do not reconcile".into());
        }
        Ok(())
    }
    pub(crate) fn change_since(&self, old: &Self) -> Self {
        Self {
            acquisitions: Money(self.acquisitions.0 - old.acquisitions.0),
            depreciation: Money(self.depreciation.0 - old.depreciation.0),
            net_assets: Money(self.net_assets.0 - old.net_assets.0),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrivateInvestmentReason {
    Purchased,
    NoObservedImports,
    PaybackTooLong,
    EquipmentLimit,
    EarningsProtected,
    CashProtected,
    HistoryLimit,
}
impl PrivateInvestmentReason {
    pub fn explanation(&self) -> &'static str {
        match self {
            Self::Purchased => "Equipment purchased; installation precedes production",
            Self::NoObservedImports => "No observed import expense to replace",
            Self::PaybackTooLong => "Observed avoided import costs do not justify this investment",
            Self::EquipmentLimit => {
                "Existing or installing equipment meets the private equipment ceiling"
            }
            Self::EarningsProtected => "Retained earnings do not cover the equipment price",
            Self::CashProtected => {
                "Cash retained for recent payments and unfinished client contracts"
            }
            Self::HistoryLimit => "Equipment history limit reached",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivateImportObservation {
    pub food: Food,
    pub food_cost: Money,
    pub energy: Energy,
    pub energy_cost: Money,
}
impl PrivateImportObservation {
    fn validate(&self) -> Result<(), String> {
        if [
            self.food.units(),
            self.energy.units(),
            self.food_cost.0,
            self.energy_cost.0,
        ]
        .iter()
        .any(|n| !(0..=1_000_000_000_000).contains(n))
        {
            return Err("Private investment import observation is invalid".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivateInvestmentReview {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payroll_creditors: Option<Money>,
    pub day: CampaignDay,
    pub reason: PrivateInvestmentReason,
    pub good: Option<GoodKind>,
    pub price: Money,
    pub expected_daily_saving: Money,
    pub retained_earnings: Money,
    pub payment_buffer: Money,
    pub client_advances: Money,
    pub available_cash: Money,
    pub opening_cash: Money,
    pub operating_receipts_before: Money,
    pub public_receipts_before: Money,
    pub payments_before: Money,
    pub distributions_before: Money,
    pub checkpoint: BusinessProfitTotals,
    pub observed: PrivateImportObservation,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivateInvestmentState {
    pub as_of: CampaignDay,
    pub equipment: Vec<PrivateEquipment>,
    pub last_review: Option<PrivateInvestmentReview>,
}
impl PrivateInvestmentState {
    pub(crate) fn opening(day: CampaignDay) -> Self {
        Self {
            as_of: day,
            equipment: Vec::new(),
            last_review: None,
        }
    }
    fn evaluate(
        &self,
        d: &PrivateInvestmentDefinition,
        basis: i64,
        r: &PrivateInvestmentReview,
    ) -> (Option<GoodKind>, Money, PrivateInvestmentReason) {
        let extra = basis * i64::from(d.capacity_points) / 100;
        let mut candidates = vec![];
        for (good, units, cost) in [
            (
                GoodKind::Food,
                r.observed.food.units(),
                r.observed.food_cost.0,
            ),
            (
                GoodKind::Energy,
                r.observed.energy.units(),
                r.observed.energy_cost.0,
            ),
        ] {
            if units > 0 && cost > 0 {
                let saving =
                    (i128::from(cost) * i128::from(extra.min(units)) / i128::from(units)) as i64;
                let count = self
                    .equipment
                    .iter()
                    .filter(|e| e.good == good && e.retired > r.day && e.acquired < r.day)
                    .count();
                candidates.push((saving, count, good));
            }
        }
        // Prefer greatest observed avoidable expense; food wins an exact tie.
        candidates.sort_by_key(|a| std::cmp::Reverse(a.0));
        let choice = candidates
            .iter()
            .find(|(_, n, _)| *n < d.maximum_active_per_good as usize)
            .copied();
        let (saving, good) = choice.map_or((0, None), |(s, _, g)| (s, Some(g)));
        let reason = if candidates.is_empty() {
            PrivateInvestmentReason::NoObservedImports
        } else if choice.is_none() {
            PrivateInvestmentReason::EquipmentLimit
        } else if i128::from(saving) * i128::from(d.maximum_payback_days.units())
            < i128::from(r.price.0)
        {
            PrivateInvestmentReason::PaybackTooLong
        } else if r.retained_earnings.0 < r.price.0 {
            PrivateInvestmentReason::EarningsProtected
        } else if r.available_cash.0 < r.price.0 {
            PrivateInvestmentReason::CashProtected
        } else if self.equipment.iter().filter(|e| e.acquired < r.day).count() >= 256 {
            PrivateInvestmentReason::HistoryLimit
        } else {
            PrivateInvestmentReason::Purchased
        };
        (good, Money(saving), reason)
    }
    pub fn totals(&self) -> Result<PrivateCapitalTotals, String> {
        self.totals_at(self.as_of)
    }
    pub fn totals_at(&self, day: CampaignDay) -> Result<PrivateCapitalTotals, String> {
        let mut acquisitions = 0_i128;
        let mut depreciation = 0_i128;
        for e in self.equipment.iter().filter(|e| e.acquired <= day) {
            acquisitions += i128::from(e.cost.0);
            depreciation += i128::from(e.depreciation(day)?.0);
        }
        let money = |n| {
            i64::try_from(n)
                .map(Money)
                .map_err(|_| "Equipment totals overflow".to_string())
        };
        Ok(PrivateCapitalTotals {
            acquisitions: money(acquisitions)?,
            depreciation: money(depreciation)?,
            net_assets: money(acquisitions - depreciation)?,
        })
    }
    pub fn active_count(&self, good: GoodKind) -> usize {
        self.equipment
            .iter()
            .filter(|e| e.good == good && e.active(self.as_of))
            .count()
    }
}
impl Country {
    pub(crate) fn private_capacity(&self, c: &Content, good: GoodKind) -> i32 {
        let (Some(d), Some(s)) = (&c.private_investment, &self.private_investment) else {
            return 0;
        };
        // Incremental equipment uses already paid civilian factors, with one-day
        // readiness. It does not invent another payroll or take public materials.
        let costs = self.business_costs.as_ref().and_then(|b| b.last.as_ref());
        let definition = c
            .business_costs
            .as_ref()
            .expect("validated private factor shares");
        let share = match good {
            GoodKind::Food => definition.food_factor_bps,
            GoodKind::Energy => definition.energy_factor_bps,
        };
        let paid = costs.map_or(0, |b| {
            (i128::from(
                b.factors_paid.0
                    - self
                        .private_payroll
                        .as_ref()
                        .and_then(|s| s.last.as_ref())
                        .map_or(0, |day| {
                            day.arrears_paid().expect("validated payroll receipt").0
                        }),
            ) * i128::from(share.units())
                / 10000) as i64
        });
        let required = (c.country(&self.id).expect("country").gdp.units() / 365 / 100).max(1);
        let readiness = (paid.min(required).max(0) * 100 / required) as i32;
        s.active_count(good) as i32 * d.capacity_points as i32 * readiness / 100
    }
}
impl World {
    pub(crate) fn date_private_equipment(&mut self) {
        let mut notices = Vec::new();
        for c in self.countries.values_mut() {
            if let Some(s) = &mut c.private_investment {
                s.as_of = self.day;
                for equipment in &s.equipment {
                    if equipment.commissioned == self.day {
                        notices.push((c.id.clone(),format!("Private {:?} equipment #{} finished installation; actual production still needs paid factors and storage.",equipment.good,equipment.id.0)));
                    }
                    if equipment.retired == self.day {
                        notices.push((c.id.clone(),format!("Private {:?} equipment #{} reached its service-life limit and stopped production. Its acquisition cost is fully depreciated.",equipment.good,equipment.id.0)));
                    }
                }
            }
        }
        for (id, message) in notices {
            self.event(&id, false, message);
        }
    }
    pub(crate) fn review_private_investment(
        &mut self,
        content: &Content,
        id: &CountryId,
    ) -> Result<(), String> {
        let Some(d) = &content.private_investment else {
            return Ok(());
        };
        let elapsed = self.day.units() - content.start_day()?.units();
        if elapsed == 0 || !elapsed.is_multiple_of(30) {
            return Ok(());
        }
        let c = &self.countries[id];
        let state = c
            .private_investment
            .as_ref()
            .ok_or("Missing private equipment")?;
        let book = c
            .business_costs
            .as_ref()
            .ok_or("Missing private cost book")?;
        let last = book.last.as_ref().ok_or("Missing observed import costs")?;
        let price = d.cost(content, id)?;
        let b = c.business.as_ref().ok_or("Missing private businesses")?;
        let retained = Money(book.day_opening.result()?.0 - b.distributions.0);
        let buffer: i128 = self
            .ledger
            .iter()
            .filter(|e| {
                &e.country == id
                    && e.from == Account::Firms
                    && e.day.units() >= self.day.units().saturating_sub(29)
            })
            .map(|e| i128::from(e.amount.0))
            .sum();
        let buffer =
            Money(i64::try_from(buffer).map_err(|_| "Private investment buffer overflow")?);
        let (pending, _, _) = c.business_capital(content)?;
        let advances = Money(pending.iter().map(|p| p.amount.0).sum());
        let available = Money(
            (i128::from(c.accounts.firms.0)
                - i128::from(buffer.0)
                - i128::from(advances.0)
                - i128::from(
                    c.private_payroll
                        .as_ref()
                        .map(|s| s.unpaid())
                        .transpose()?
                        .map_or(0, |m| m.0),
                ))
            .max(0) as i64,
        );
        let basis = (content.country(id).expect("country").population.0 / 1000) as i64;
        let mut review = PrivateInvestmentReview {
            payroll_creditors: c.private_payroll.as_ref().map(|s| s.unpaid()).transpose()?,
            day: self.day,
            reason: PrivateInvestmentReason::NoObservedImports,
            good: None,
            price,
            expected_daily_saving: Money(0),
            retained_earnings: retained,
            payment_buffer: buffer,
            client_advances: advances,
            available_cash: available,
            opening_cash: c.accounts.firms,
            operating_receipts_before: b.operating_receipts,
            public_receipts_before: b.public_receipts,
            payments_before: b.payments,
            distributions_before: b.distributions,
            checkpoint: book.day_opening.clone(),
            observed: PrivateImportObservation {
                food: last.food.physical.imports,
                food_cost: last.food.import_cost,
                energy: last.energy.physical.imports,
                energy_cost: last.energy.import_cost,
            },
        };
        let (good, saving, reason) = state.evaluate(d, basis, &review);
        review.good = good;
        review.expected_daily_saving = saving;
        review.reason = reason.clone();
        if reason == PrivateInvestmentReason::Purchased {
            let commissioned = self
                .day
                .checked_add(d.installation_days)
                .ok_or("Equipment installation exceeds calendar")?;
            let retired = commissioned
                .checked_add(d.lifetime_days)
                .ok_or("Equipment lifetime exceeds calendar")?;
            if self.pay(
                id,
                Account::Firms,
                Account::External,
                price.0,
                "Private productive equipment acquisition",
            ) != price.0
            {
                return Err("Private equipment receipt lacks full payment".into());
            }
            let s = self
                .countries
                .get_mut(id)
                .expect("country")
                .private_investment
                .as_mut()
                .expect("equipment");
            s.equipment.push(PrivateEquipment {
                id: PrivateEquipmentId(s.equipment.len() as u32 + 1),
                good: good.expect("chosen"),
                acquired: self.day,
                commissioned,
                retired,
                cost: price,
            });
            self.event(id,false,format!("Private businesses purchased {:?} equipment for {}m; installation takes {} days.",good.expect("chosen"),price.0,d.installation_days.units()));
        }
        self.countries
            .get_mut(id)
            .expect("country")
            .private_investment
            .as_mut()
            .expect("equipment")
            .last_review = Some(review);
        Ok(())
    }
    pub(crate) fn validate_private_investment(&self, content: &Content) -> Result<(), String> {
        if self.day < content.start_day()? {
            return Err("Private investment clock precedes its bookmark".into());
        }
        for (id, c) in &self.countries {
            match (&content.private_investment, &c.private_investment) {
                (None, None) => continue,
                (Some(d), Some(s)) if (39..=41).contains(&self.rules) => {
                    if s.as_of != self.day || s.equipment.len() > 256 {
                        return Err("Private equipment date or count is invalid".into());
                    }
                    let start = content.start_day()?;
                    let mut previous = None;
                    for (i, e) in s.equipment.iter().enumerate() {
                        let elapsed = e
                            .acquired
                            .units()
                            .checked_sub(start.units())
                            .ok_or("Equipment predates bookmark")?;
                        if e.id != PrivateEquipmentId(i as u32 + 1)
                            || elapsed == 0
                            || !elapsed.is_multiple_of(30)
                            || e.acquired > self.day
                            || previous.is_some_and(|p| p >= e.acquired)
                            || e.acquired.checked_add(d.installation_days) != Some(e.commissioned)
                            || e.commissioned.checked_add(d.lifetime_days) != Some(e.retired)
                            || e.cost != d.cost(content, id)?
                        {
                            return Err("Private equipment acquisition receipt is invalid".into());
                        }
                        let active = s.equipment[..=i]
                            .iter()
                            .filter(|a| a.good == e.good && a.retired > e.acquired)
                            .count();
                        if active > d.maximum_active_per_good as usize {
                            return Err("Private equipment exceeds acquisition limit".into());
                        }
                        previous = Some(e.acquired);
                    }
                    s.totals()?.validate()?;
                    let elapsed = self.day.units() - start.units();
                    match &s.last_review {
                        None if elapsed < 30 => {}
                        Some(r) if r.day.units() == self.day.units() - elapsed % 30 => {
                            r.checkpoint.validate_checkpoint()?;
                            r.observed.validate()?;
                            let basis = (content
                                .country(id)
                                .ok_or("Unknown equipment country")?
                                .population
                                .0
                                / 1000) as i64;
                            if s.evaluate(d, basis, r)
                                != (r.good, r.expected_daily_saving, r.reason.clone())
                            {
                                return Err("Private investment reason differs from its observed costs and constraints".into());
                            }
                            let costs = &c
                                .business_costs
                                .as_ref()
                                .ok_or("Missing private cost book")?
                                .costs;
                            if r.observed.food_cost.0 > costs.food_imports.0
                                || r.observed.energy_cost.0 > costs.energy_imports.0
                            {
                                return Err(
                                    "Observed private imports exceed actual cumulative costs"
                                        .into(),
                                );
                            }
                            if r.day == self.day {
                                let last = c
                                    .business_costs
                                    .as_ref()
                                    .and_then(|b| b.last.as_ref())
                                    .ok_or("Missing private import receipt")?;
                                if r.observed
                                    != (PrivateImportObservation {
                                        food: last.food.physical.imports,
                                        food_cost: last.food.import_cost,
                                        energy: last.energy.physical.imports,
                                        energy_cost: last.energy.import_cost,
                                    })
                                {
                                    return Err("Private investment differs from actual settled import receipt".into());
                                }
                            }
                            let b = c.business.as_ref().ok_or("Missing private cash account")?;
                            let payout = b
                                .last_decision
                                .as_ref()
                                .and_then(|p| p.earnings_protection.as_ref())
                                .ok_or("Missing payout before investment review")?;
                            let paid = if r.reason == PrivateInvestmentReason::Purchased {
                                r.price.0
                            } else {
                                0
                            };
                            if r.payroll_creditors != payout.payroll_creditors
                                || r.checkpoint != payout.checkpoint
                                || r.checkpoint.private_capital
                                    != Some(s.totals_at(CampaignDay::new(r.day.units() - 1))?)
                                || r.distributions_before != b.distributions
                                || i128::from(r.retained_earnings.0)
                                    != i128::from(r.checkpoint.result()?.0)
                                        - i128::from(r.distributions_before.0)
                                || r.operating_receipts_before.0 > b.operating_receipts.0
                                || r.public_receipts_before.0 > b.public_receipts.0
                                || i128::from(r.payments_before.0) + i128::from(paid)
                                    > i128::from(b.payments.0)
                                || i128::from(r.opening_cash.0)
                                    != i128::from(b.opening_cash.0)
                                        + i128::from(r.operating_receipts_before.0)
                                        + i128::from(r.public_receipts_before.0)
                                        - i128::from(r.payments_before.0)
                                || r.payment_buffer.0 > r.payments_before.0
                                || r.client_advances.0 > r.public_receipts_before.0
                                || i128::from(r.available_cash.0)
                                    != (i128::from(r.opening_cash.0)
                                        - i128::from(r.payment_buffer.0)
                                        - i128::from(r.client_advances.0)
                                        - i128::from(r.payroll_creditors.map_or(0, |m| m.0)))
                                    .max(0)
                                || s.equipment.iter().any(|e| e.acquired == r.day)
                                    != (r.reason == PrivateInvestmentReason::Purchased)
                                || r.price != d.cost(content, id)?
                                || [
                                    r.payment_buffer,
                                    r.client_advances,
                                    r.available_cash,
                                    r.expected_daily_saving,
                                    r.opening_cash,
                                    r.operating_receipts_before,
                                    r.public_receipts_before,
                                    r.payments_before,
                                    r.distributions_before,
                                ]
                                .iter()
                                .any(|x| !(0..=1_000_000_000_000_000_000).contains(&x.0))
                                || r.available_cash.0 < 0
                                || (r.reason == PrivateInvestmentReason::Purchased
                                    && (r.good.is_none()
                                        || i128::from(r.expected_daily_saving.0)
                                            * i128::from(d.maximum_payback_days.units())
                                            < i128::from(r.price.0)
                                        || r.retained_earnings.0 < r.price.0
                                        || r.available_cash.0 < r.price.0
                                        || !s.equipment.iter().any(|e| {
                                            e.acquired == r.day && Some(e.good) == r.good
                                        })))
                            {
                                return Err("Private investment review is invalid".into());
                            }
                        }
                        _ => return Err("Private investment review is missing or misdated".into()),
                    }
                }
                _ => return Err("Private equipment requires its exact investment rules".into()),
            }
        }
        Ok(())
    }
    pub fn private_investment_report(
        &self,
        content: &Content,
        viewer: Viewer,
        id: &CountryId,
    ) -> Result<Option<PrivateInvestmentState>, String> {
        if self.content_hash != content.hash() {
            return Err("Equipment inspection requires exact content".into());
        }
        let c = self.countries.get(id).ok_or("Unknown equipment country")?;
        if viewer != Viewer::Government(self.player.clone()) || id != &self.player {
            return Ok(None);
        }
        self.validate_private_investment(content)?;
        Ok(c.private_investment.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tiny_acquisition_keeps_fractional_depreciation_until_actual_service_and_retires_fully() {
        let equipment = PrivateEquipment {
            id: PrivateEquipmentId(1),
            good: GoodKind::Food,
            acquired: CampaignDay::new(30),
            commissioned: CampaignDay::new(60),
            retired: CampaignDay::new(3710),
            cost: Money(7),
        };
        assert_eq!(
            equipment.depreciation(CampaignDay::new(59)).unwrap(),
            Money(0)
        );
        assert!(!equipment.active(CampaignDay::new(59)));
        assert!(equipment.active(CampaignDay::new(60)));
        assert_eq!(
            equipment.depreciation(CampaignDay::new(581)).unwrap(),
            Money(0)
        );
        assert_eq!(
            equipment.depreciation(CampaignDay::new(582)).unwrap(),
            Money(1)
        );
        assert_eq!(
            equipment.depreciation(CampaignDay::new(3710)).unwrap(),
            Money(7)
        );
        assert!(!equipment.active(CampaignDay::new(3710)));
        let state = PrivateInvestmentState {
            as_of: CampaignDay::new(8000),
            equipment: vec![equipment],
            last_review: None,
        };
        assert_eq!(state.totals().unwrap().net_assets, Money(0));
    }
}
