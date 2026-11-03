use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const RULES_VERSION: u32 = 2;
pub const MAX_DAY: u32 = 36_525;
/// Money is in millions of constant-2000 USD-equivalent game credits.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Money(pub i64);
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Population(pub u64);
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Account {
    Treasury,
    Households,
    Firms,
    External,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Accounts {
    pub treasury: Money,
    pub households: Money,
    pub firms: Money,
    pub external: Money,
}
impl Accounts {
    pub fn checked_total(&self) -> Option<i64> {
        self.treasury
            .0
            .checked_add(self.households.0)?
            .checked_add(self.firms.0)?
            .checked_add(self.external.0)
    }
    pub fn total(&self) -> i64 {
        self.treasury.0 + self.households.0 + self.firms.0 + self.external.0
    }
    pub fn balance(&self, account: Account) -> i64 {
        match account {
            Account::Treasury => self.treasury.0,
            Account::Households => self.households.0,
            Account::Firms => self.firms.0,
            Account::External => self.external.0,
        }
    }
    fn balance_mut(&mut self, account: Account) -> &mut i64 {
        match account {
            Account::Treasury => &mut self.treasury.0,
            Account::Households => &mut self.households.0,
            Account::Firms => &mut self.firms.0,
            Account::External => &mut self.external.0,
        }
    }
    pub(crate) fn transfer(&mut self, from: Account, to: Account, requested: i64) -> i64 {
        let paid = requested.max(0).min(self.balance(from));
        *self.balance_mut(from) -= paid;
        *self.balance_mut(to) += paid;
        paid
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LedgerEntry {
    pub day: u32,
    pub country: CountryId,
    pub from: Account,
    pub to: Account,
    pub amount: Money,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyInstance {
    pub id: u64,
    pub policy: String,
    pub value: i32,
    pub start_day: u32,
    pub due_day: u32,
    pub cost: Money,
    pub stage: PolicyStage,
    pub consent_day: u32,
    pub reserved: Money,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DailyReport {
    pub revenue: Money,
    pub spending: Money,
    pub interest: Money,
    pub unpaid: Money,
    pub wages: Money,
    pub consumption: Money,
    pub energy_coverage: i32,
    pub food_coverage: i32,
    pub service_delivery: i32,
    pub output: Money,
    pub shock: i32,
    pub energy_produced: i64,
    pub energy_imported: i64,
    pub energy_used: i64,
    pub food_produced: i64,
    pub food_imported: i64,
    pub food_consumed: i64,
    pub food_lost: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Observation {
    pub day: u32,
    pub treasury: Money,
    pub debt: Money,
    pub approval: i32,
    pub needs: i32,
    pub output: Money,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Country {
    pub id: CountryId,
    pub population: Population,
    pub accounts: Accounts,
    pub households: Vec<Household>,
    pub committed_cash: Money,
    pub council_support: i32,
    pub confidence_crises: u32,
    pub next_leadership_review: Option<u32>,
    pub budget: BudgetPeriod,
    pub budget_reviews: Vec<BudgetReview>,
    pub opening_money: Money,
    pub debt: Money,
    pub arrears: Money,
    pub arrears_households: Money,
    pub arrears_external: Money,
    pub arrears_firms: Money,
    pub tax: i32,
    pub services: i32,
    pub welfare: i32,
    pub energy_capacity: i32,
    pub opening_energy_capacity: i32,
    pub opening_food_capacity: i32,
    pub food_capacity: i32,
    pub administration: i32,
    pub approval: i32,
    pub elite_support: i32,
    pub legislative_support: i32,
    pub ruling_party: String,
    pub coalition: Vec<String>,
    pub executive: String,
    pub seats: BTreeMap<String, u16>,
    pub next_election: Option<u32>,
    pub government_number: u32,
    pub projects: Vec<PolicyInstance>,
    pub energy_stock: i64,
    pub food_stock: i64,
    pub report: DailyReport,
    pub history: Vec<Observation>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Action {
    Enact { policy: String, value: i32 },
    Borrow { amount: Money },
    Repay { amount: Money },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Command {
    pub id: u64,
    pub day: u32,
    pub issuer: CountryId,
    pub action: Action,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    pub day: u32,
    pub country: CountryId,
    pub important: bool,
    pub text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct JournalEntry {
    pub command: Command,
    pub ai: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Scenario {
    Sandbox,
    FiscalSupplyCrisis,
}
impl Scenario {
    pub fn id(self) -> &'static str {
        match self {
            Self::Sandbox => "sandbox",
            Self::FiscalSupplyCrisis => "crisis",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct World {
    pub rules: u32,
    pub content_hash: u64,
    pub seed: u64,
    pub scenario: Scenario,
    pub day: u32,
    pub player: CountryId,
    pub countries: BTreeMap<CountryId, Country>,
    pub next_command: u64,
    pub next_project: u64,
    pub journal: Vec<JournalEntry>,
    pub events: Vec<Event>,
    pub ledger: Vec<LedgerEntry>,
    pub milestone_reported: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyPreview {
    pub cost: Money,
    pub delay: u32,
    pub annual_budget_change: Money,
    pub route: String,
    pub explanation: String,
    pub consent_delay: u32,
    pub upfront_payment: Money,
    pub household_impacts: Vec<HouseholdImpact>,
}
impl World {
    pub fn new(content: &Content, player: CountryId, seed: u64) -> Result<Self, String> {
        Self::new_scenario(content, player, seed, Scenario::Sandbox)
    }
    pub fn new_scenario(
        content: &Content,
        player: CountryId,
        seed: u64,
        scenario: Scenario,
    ) -> Result<Self, String> {
        content.validate()?;
        if content.country(&player).is_none() {
            return Err("Unknown player country".into());
        }
        let countries = content
            .countries
            .iter()
            .map(|definition| {
                let mut overlay = definition.clone();
                if scenario == Scenario::FiscalSupplyCrisis && definition.id == player {
                    overlay.tax = 10;
                    overlay.services = 18;
                    overlay.welfare = 12;
                    overlay.energy = 40;
                    overlay.food = 40;
                    overlay.capacity = 60;
                    overlay.approval = 20;
                }
                let d = &overlay;
                let accounts = Accounts {
                    treasury: Money(d.gdp / 20),
                    households: Money(d.gdp / 3),
                    firms: Money(d.gdp / 2),
                    external: Money(d.gdp * 3),
                };
                let opening_money = Money(accounts.total());
                let seats: BTreeMap<_, _> =
                    d.parties.iter().map(|p| (p.id.clone(), p.seats)).collect();
                let support = d.coalition.iter().map(|p| i32::from(seats[p])).sum();
                (
                    d.id.clone(),
                    Country {
                        id: d.id.clone(),
                        population: Population(d.population),
                        households: crate::households::initial_households(d, accounts.households),
                        committed_cash: Money(0),
                        council_support: 65,
                        confidence_crises: 0,
                        next_leadership_review: d.government.leadership_review_days,
                        budget: BudgetPeriod {
                            start_day: 0,
                            end_day: crate::budget::next_fiscal_end(0),
                            opening_cash: accounts.treasury,
                            forecast: BudgetForecast {
                                daily_revenue: Money(
                                    d.gdp / 365 * 70 / 100
                                        * i64::from(d.tax)
                                        * i64::from(d.capacity)
                                        / 10000,
                                ),
                                daily_spending: Money(
                                    d.gdp / 365 * i64::from(d.services) / 100
                                        + d.gdp / 365 * i64::from(d.welfare) / 100,
                                ),
                                daily_interest: Money((d.gdp / 3 * 4 / 100 / 365).max(1)),
                            },
                            revenue: Money(0),
                            payments: Money(0),
                            capital: Money(0),
                            borrowing: Money(0),
                            principal_repaid: Money(0),
                        },
                        budget_reviews: Vec::new(),
                        accounts,
                        opening_money,
                        debt: Money(d.gdp / 3),
                        arrears: Money(0),
                        arrears_households: Money(0),
                        arrears_external: Money(0),
                        arrears_firms: Money(0),
                        tax: d.tax,
                        services: d.services,
                        welfare: d.welfare,
                        energy_capacity: d.energy,
                        opening_energy_capacity: d.energy,
                        opening_food_capacity: d.food,
                        food_capacity: d.food,
                        administration: d.capacity,
                        approval: d.approval,
                        elite_support: 65,
                        legislative_support: support,
                        ruling_party: d.ruling_party.clone(),
                        coalition: d.coalition.clone(),
                        executive: d.executive.clone(),
                        seats,
                        next_election: d.regime.competitive().then_some(730),
                        government_number: 1,
                        projects: Vec::new(),
                        energy_stock: 0,
                        food_stock: 0,
                        report: DailyReport::default(),
                        history: Vec::new(),
                    },
                )
            })
            .collect();
        let mut world = Self {
            rules: RULES_VERSION,
            content_hash: content.hash(),
            seed,
            scenario,
            day: 0,
            player: player.clone(),
            countries,
            next_command: 1,
            next_project: 1,
            journal: Vec::new(),
            events: Vec::new(),
            ledger: Vec::new(),
            milestone_reported: false,
        };
        for (id, country) in &mut world.countries {
            country.budget.forecast = country.forecast(content.country(id).expect("definition"));
        }
        world.event(&player, true, "Welcome. Review supply gaps, fiscal balance and governing support. Five-year objective: needs ≥90%, approval ≥50%, no arrears, debt ≤100% of opening GDP. Values are authored game approximations.");
        if scenario == Scenario::FiscalSupplyCrisis {
            world.event(&player,true,"Authored fiscal/supply crisis: selected government starts with 40% domestic supplies, 10% tax, 18% services and 12% welfare. These are scenario overrides, not historical observations. Stabilize finances, rebuild supply and maintain authority.");
        }
        Ok(world)
    }
    pub fn player_country(&self) -> &Country {
        &self.countries[&self.player]
    }
    pub fn hash(&self) -> u64 {
        crate::stable_hash(&serde_json::to_vec(self).expect("world serialization"))
    }
    pub fn command(&self, issuer: CountryId, action: Action) -> Command {
        Command {
            id: self.next_command,
            day: self.day,
            issuer,
            action,
        }
    }
    pub fn execute(&mut self, content: &Content, command: Command) -> Result<(), String> {
        self.apply(content, command, false)
    }
    pub fn issue(&mut self, content: &Content, action: Action) -> Result<(), String> {
        self.apply(content, self.command(self.player.clone(), action), false)
    }
    pub(crate) fn event(&mut self, country: &CountryId, important: bool, text: impl Into<String>) {
        self.events.push(Event {
            day: self.day,
            country: country.clone(),
            important,
            text: text.into(),
        });
        if self.events.len() > 400 {
            self.events.remove(0);
        }
    }
    pub(crate) fn pay(
        &mut self,
        country: &CountryId,
        from: Account,
        to: Account,
        amount: i64,
        reason: &str,
    ) -> i64 {
        self.transfer(country, from, to, amount, reason, None)
    }
    pub(crate) fn transfer(
        &mut self,
        country: &CountryId,
        from: Account,
        to: Account,
        amount: i64,
        reason: &str,
        weights: Option<&[i64]>,
    ) -> i64 {
        let c = self.countries.get_mut(country).expect("known country");
        let requested = if from == Account::Treasury {
            amount
                .max(0)
                .min(c.accounts.treasury.0 - c.committed_cash.0)
        } else {
            amount
        };
        let paid = c.accounts.transfer(from, to, requested);
        if from == Account::Households && paid > 0 {
            let split = apportion(
                paid,
                &c.households.iter().map(|h| h.cash.0).collect::<Vec<_>>(),
            );
            for (h, amount) in c.households.iter_mut().zip(split) {
                h.cash.0 -= amount;
            }
        }
        if to == Account::Households && paid > 0 {
            let default_weights: Vec<_> =
                c.households.iter().map(|h| h.population.0 as i64).collect();
            let split = apportion(paid, weights.unwrap_or(&default_weights));
            for (h, amount) in c.households.iter_mut().zip(split) {
                h.cash.0 += amount;
                h.income.0 += amount;
            }
        }
        if paid > 0 {
            self.ledger.push(LedgerEntry {
                day: self.day,
                country: country.clone(),
                from,
                to,
                amount: Money(paid),
                reason: reason.into(),
            });
        }
        paid
    }
    /// Validate snapshot invariants before accepting a loaded campaign.
    pub fn validate(&self, content: &Content) -> Result<(), String> {
        content.validate()?;
        if self.rules != RULES_VERSION || self.content_hash != content.hash() {
            return Err("Rules or content pack mismatch".into());
        }
        if self.day > MAX_DAY
            || !self.countries.contains_key(&self.player)
            || self.countries.len() != content.countries.len()
            || self.next_command != self.journal.len() as u64 + 1
        {
            return Err("Invalid world clock, player, roster or command sequence".into());
        }
        for (id, c) in &self.countries {
            let d = content.country(id).ok_or("Unknown country in save")?;
            if id != &c.id
                || c.accounts.checked_total() != Some(c.opening_money.0)
                || c.opening_money.0 != d.gdp / 20 + d.gdp / 3 + d.gdp / 2 + d.gdp * 3
                || [
                    c.accounts.treasury.0,
                    c.accounts.households.0,
                    c.accounts.firms.0,
                    c.accounts.external.0,
                    c.debt.0,
                    c.arrears.0,
                    c.arrears_households.0,
                    c.arrears_external.0,
                    c.arrears_firms.0,
                ]
                .iter()
                .any(|v| !(0..=1_000_000_000_000).contains(v))
                || c.arrears.0 != c.arrears_households.0 + c.arrears_external.0 + c.arrears_firms.0
                || !(10..=45).contains(&c.tax)
                || !(4..=18).contains(&c.services)
                || !(0..=12).contains(&c.welfare)
                || !(0..=100).contains(&c.approval)
                || !(0..=100).contains(&c.elite_support)
                || !(0..=100).contains(&c.legislative_support)
                || !(1..=100).contains(&c.administration)
                || c.opening_energy_capacity
                    != if self.scenario == Scenario::FiscalSupplyCrisis && id == &self.player {
                        40
                    } else {
                        d.energy
                    }
                || c.opening_food_capacity
                    != if self.scenario == Scenario::FiscalSupplyCrisis && id == &self.player {
                        40
                    } else {
                        d.food
                    }
                || !(1..=200).contains(&c.energy_capacity)
                || !(1..=200).contains(&c.food_capacity)
                || c.population.0 != d.population
                || !(0..=1_000_000_000).contains(&c.energy_stock)
                || !(0..=1_000_000_000).contains(&c.food_stock)
                || c.projects.len() > content.policies.len()
                || c.history.len() > 120
                || c.seats.values().map(|v| u32::from(*v)).sum::<u32>() != 100
                || c.seats.len() != d.parties.len()
                || d.parties.iter().any(|p| !c.seats.contains_key(&p.id))
                || !c.seats.contains_key(&c.ruling_party)
                || !c.coalition.contains(&c.ruling_party)
                || c.coalition.iter().any(|id| !c.seats.contains_key(id))
                || c.coalition
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != c.coalition.len()
                || c.next_election
                    .is_some_and(|day| day <= self.day || day > MAX_DAY + 1460)
            {
                return Err(format!(
                    "Invalid country balances, institutions or ranges: {}",
                    id.0
                ));
            }
            if c.households
                .iter()
                .any(|h| h.arrears.0 < 0 || h.arrears.0 > 1_000_000_000_000)
                || c.households
                    .iter()
                    .map(|h| i128::from(h.arrears.0))
                    .sum::<i128>()
                    != i128::from(c.arrears_households.0)
            {
                return Err("Household creditor claims do not reconcile".into());
            }
            if c.households.len() != d.households.len()
                || c.households
                    .iter()
                    .map(|h| u128::from(h.population.0))
                    .sum::<u128>()
                    != u128::from(c.population.0)
                || c.households.iter().any(|h| {
                    ![
                        h.cash.0,
                        h.income.0,
                        h.tax.0,
                        h.consumption.0,
                        h.basket_cost.0,
                    ]
                    .iter()
                    .all(|v| (0..=1_000_000_000_000).contains(v))
                        || h.tax.0 > h.income.0
                        || h.working_age.0 > h.population.0
                        || h.employed.0 > h.working_age.0
                        || !(0..=100).contains(&h.food_coverage)
                        || !(0..=100).contains(&h.approval)
                })
                || c.households
                    .iter()
                    .map(|h| i128::from(h.cash.0))
                    .sum::<i128>()
                    != i128::from(c.accounts.households.0)
                || c.households
                    .iter()
                    .zip(&d.households)
                    .any(|(h, d)| h.id != d.id)
                || !(0..=100).contains(&c.council_support)
                || c.committed_cash.0 < 0
                || c.committed_cash.0 > c.accounts.treasury.0
                || c.projects
                    .iter()
                    .map(|p| i128::from(p.reserved.0))
                    .sum::<i128>()
                    != i128::from(c.committed_cash.0)
                || c.budget.start_day > self.day
                || c.budget.end_day <= self.day
                || c.budget.end_day > MAX_DAY + 366
                || !c.budget.valid_ranges()
                || c.budget.expected_cash() != c.accounts.treasury.0
                || c.budget_reviews.iter().any(|r| {
                    !r.period.valid_ranges() || r.period.expected_cash() != r.closing_cash.0
                })
                || c.budget_reviews.len() > 10
                || c.next_leadership_review.is_some_and(|day| day <= self.day)
            {
                return Err(format!(
                    "Invalid household, commitment or budget reconciliation: {}",
                    id.0
                ));
            }
            let mut pending = std::collections::BTreeSet::new();
            for project in &c.projects {
                let p = content
                    .policy(&project.policy)
                    .ok_or("Missing pending policy definition")?;
                if !pending.insert(&project.policy)
                    || project.id >= self.next_project
                    || project.reserved.0 < 0
                    || (project.stage == PolicyStage::AwaitingConsent
                        && project.consent_day <= self.day)
                    || (project.stage == PolicyStage::Implementing && project.reserved.0 != 0)
                    || project.due_day <= self.day
                    || project.due_day > MAX_DAY + 1000
                    || !(p.min..=p.max).contains(&project.value)
                {
                    return Err("Invalid pending policy".into());
                }
            }
        }
        Ok(())
    }
}
/// Gregorian calendar from the fixed bookmark, including century leap-year rules.
pub fn date(day: u32) -> String {
    let mut year = 2000;
    let mut remainder = day;
    loop {
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let length = if leap { 366 } else { 365 };
        if remainder < length {
            let months = [
                31,
                if leap { 29 } else { 28 },
                31,
                30,
                31,
                30,
                31,
                31,
                30,
                31,
                30,
                31,
            ];
            for (month, days) in months.iter().enumerate() {
                if remainder < *days {
                    return format!("{year:04}-{:02}-{:02}", month + 1, remainder + 1);
                }
                remainder -= days;
            }
        }
        remainder -= length;
        year += 1;
    }
}
