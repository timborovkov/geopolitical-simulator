//! Rules-3 relative prices, with physical quantities kept dimensionally distinct.
use crate::{BasisPoints, Content, Energy, Food, NeedDays};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct GoodId(pub String);

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GoodKind {
    Energy,
    Food,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoodDefinition {
    pub id: GoodId,
    pub kind: GoodKind,
    pub name: String,
    pub unit: String,
    pub minimum_price_bps: BasisPoints,
    pub maximum_price_bps: BasisPoints,
    /// Maximum relative daily price movement, in basis points.
    pub adjustment_bps: BasisPoints,
    pub import_limit_bps: BasisPoints,
    /// Working inventory capacity in multiples of baseline daily need.
    /// ```compile_fail
    /// use sim_core::{BasisPoints, GoodDefinition};
    /// fn cannot_use_a_rate_as_storage_capacity(good: &mut GoodDefinition) {
    ///     good.storage_days = BasisPoints::new(300);
    /// }
    /// ```
    pub storage_days: NeedDays,
    pub loss_bps: BasisPoints,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketDefinition {
    pub goods: Vec<GoodDefinition>,
    /// Share of the essential basket repriced by the food price index.
    pub food_budget_share_bps: BasisPoints,
    pub source: String,
    pub source_version: String,
}
impl MarketDefinition {
    pub fn good(&self, kind: GoodKind) -> &GoodDefinition {
        self.goods
            .iter()
            .find(|good| good.kind == kind)
            .expect("validated good kind")
    }
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        if self.goods.len() != 2
            || self.food_budget_share_bps.units() > 10000
            || self.source_version.is_empty()
            || !content
                .sources
                .iter()
                .any(|source| source.id == self.source)
            || self.goods[0].id == self.goods[1].id
            || self.goods[0].kind == self.goods[1].kind
        {
            return Err("Markets require distinct food/energy definitions, a basket share and source provenance".into());
        }
        for good in &self.goods {
            if good.id.0.is_empty()
                || good.name.is_empty()
                || good.unit.is_empty()
                || !(500..=10000).contains(&good.minimum_price_bps.units())
                || !(10000..=100000).contains(&good.maximum_price_bps.units())
                || !(1..=1000).contains(&good.adjustment_bps.units())
                || good.import_limit_bps.units() > 10000
                || !(1..=30).contains(&good.storage_days.units())
                || good.loss_bps.units() > 10000
            {
                return Err(format!("Invalid good ranges or identity: {}", good.id.0));
            }
            if good.kind == GoodKind::Energy && good.loss_bps.units() != 0 {
                return Err("Energy storage losses require a network-loss model; use zero for this market version".into());
            }
        }
        Ok(())
    }
}

fn no_reserve_release<Q>() -> Option<Q> {
    None
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketSettlement<Q> {
    pub day: crate::CampaignDay,
    pub price_bps: BasisPoints,
    pub physical_need: Q,
    /// Food: household demand after affordability. Energy: requested physical industrial use; only imports have a cash settlement in this version.
    pub requested_use: Q,
    pub opening_stock: Q,
    pub production: Q,
    pub imports: Q,
    #[serde(
        default = "no_reserve_release",
        skip_serializing_if = "Option::is_none"
    )]
    pub reserve_release: Option<Q>,
    pub supply: Q,
    pub delivered: Q,
    pub loss: Q,
    pub closing_stock: Q,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MarketQuote<Q> {
    pub good: GoodId,
    /// Applies on the next daily settlement; last.price_bps records the price
    /// actually used for the last settlement, avoiding a retrospective reprice.
    pub next_price_bps: BasisPoints,
    pub last: Option<MarketSettlement<Q>>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Markets {
    pub energy: MarketQuote<Energy>,
    pub food: MarketQuote<Food>,
}
impl Markets {
    pub fn new(definition: &MarketDefinition) -> Self {
        Self {
            energy: MarketQuote {
                good: definition.good(GoodKind::Energy).id.clone(),
                next_price_bps: BasisPoints::new(10000),
                last: None,
            },
            food: MarketQuote {
                good: definition.good(GoodKind::Food).id.clone(),
                next_price_bps: BasisPoints::new(10000),
                last: None,
            },
        }
    }
    pub(crate) fn validate(
        &self,
        definition: &MarketDefinition,
        country: &crate::Country,
        day: crate::CampaignDay,
        start: crate::CampaignDay,
    ) -> Result<(), String> {
        let day = day.units();
        let start = start.units();
        validate_quote(
            &self.energy,
            definition.good(GoodKind::Energy),
            crate::CampaignDay::new(day),
            crate::CampaignDay::new(start),
            (country.population.0 / 1000) as i64,
            country.energy_stock.units(),
            [
                country.report.energy_produced.units(),
                country.report.energy_imported.units(),
                country.report.energy_used.units(),
                0,
            ],
            country
                .reserves
                .as_ref()
                .map(|s| s.energy.last.as_ref().map_or(0, |r| r.released.units())),
            Energy::units,
        )?;
        validate_quote(
            &self.food,
            definition.good(GoodKind::Food),
            crate::CampaignDay::new(day),
            crate::CampaignDay::new(start),
            (country.population.0 / 1000) as i64,
            country.food_stock.units(),
            [
                country.report.food_produced.units(),
                country.report.food_imported.units(),
                country.report.food_consumed.units(),
                country.report.food_lost.units(),
            ],
            country
                .reserves
                .as_ref()
                .map(|s| s.food.last.as_ref().map_or(0, |r| r.released.units())),
            Food::units,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_quote<Q: Copy>(
    quote: &MarketQuote<Q>,
    rule: &GoodDefinition,
    day: crate::CampaignDay,
    start: crate::CampaignDay,
    need: i64,
    stored: i64,
    report: [i64; 4],
    reserve_release: Option<i64>,
    units: impl Fn(Q) -> i64,
) -> Result<(), String> {
    let day = day.units();
    let start = start.units();
    let invalid = || format!("Invalid market settlement or price for {}", rule.id.0);
    if quote.good != rule.id
        || !(rule.minimum_price_bps.units()..=rule.maximum_price_bps.units())
            .contains(&quote.next_price_bps.units())
    {
        return Err(invalid());
    }
    let Some(last) = &quote.last else {
        return if day == start && stored == 0 && report == [0; 4] {
            Ok(())
        } else {
            Err(invalid())
        };
    };
    let physical = units(last.physical_need);
    let affordable = units(last.requested_use);
    let opening = units(last.opening_stock);
    let produced = units(last.production);
    let imported = units(last.imports);
    let supply = units(last.supply);
    let delivered = units(last.delivered);
    let loss = units(last.loss);
    let closing = units(last.closing_stock);
    if last.day != crate::CampaignDay::new(day)
        || day <= start
        || physical != need
        || affordable < 0
        || affordable > need
        || (rule.kind == GoodKind::Energy && affordable != need)
        || [
            opening, produced, imported, supply, delivered, loss, closing,
        ]
        .iter()
        .any(|value| !(0..=1_000_000_000).contains(value))
        || last.reserve_release.map(&units) != reserve_release
        || i128::from(opening)
            + i128::from(produced)
            + i128::from(imported)
            + i128::from(reserve_release.unwrap_or(0))
            != i128::from(supply)
        || imported > need * i64::from(rule.import_limit_bps.units()) / 10000
        || delivered != supply.min(affordable)
        || loss != (supply - delivered) * i64::from(rule.loss_bps.units()) / 10000
        || closing != supply - delivered - loss
        || closing != stored
        || closing > need * i64::from(rule.storage_days.units())
        || report != [produced, imported, delivered, loss]
        || next_market_price(last.price_bps, supply, affordable, rule)? != quote.next_price_bps
    {
        return Err(invalid());
    }
    Ok(())
}

/// Relative price response uses only this country's settled supply and requested
/// use (affordable food demand or physical industrial energy need). It affects the following day; caps are explicit economic assumptions.
/// ```compile_fail
/// use sim_core::{next_market_price, GoodDefinition};
/// fn cannot_pass_a_calendar_day(rule: &GoodDefinition, day: u32) {
///     next_market_price(day, 100, 100, rule);
/// }
/// ```
pub fn next_market_price(
    current: BasisPoints,
    supply: i64,
    demand: i64,
    rule: &GoodDefinition,
) -> Result<BasisPoints, String> {
    let current = current.units();
    if supply < 0
        || demand < 0
        || current < rule.minimum_price_bps.units()
        || current > rule.maximum_price_bps.units()
        || !(500..=10000).contains(&rule.minimum_price_bps.units())
        || !(10000..=100000).contains(&rule.maximum_price_bps.units())
        || !(1..=1000).contains(&rule.adjustment_bps.units())
    {
        return Err("Invalid market quantities or current price".into());
    }
    if supply == demand {
        return Ok(BasisPoints::new(current));
    }
    let imbalance = (i128::from(supply) - i128::from(demand)).abs();
    let pressure_bps = (imbalance * i128::from(rule.adjustment_bps.units())
        / i128::from(demand.max(1)))
    .min(i128::from(rule.adjustment_bps.units()));
    let movement = (i128::from(current) * pressure_bps / 10000) as u32;
    Ok(BasisPoints::new(if supply < demand {
        current
            .saturating_add(movement)
            .min(rule.maximum_price_bps.units())
    } else {
        current
            .saturating_sub(movement)
            .max(rule.minimum_price_bps.units())
    }))
}

pub(crate) fn legacy_rules() -> u32 {
    2
}
pub(crate) fn is_legacy_rules(rules: &u32) -> bool {
    *rules == 2
}
