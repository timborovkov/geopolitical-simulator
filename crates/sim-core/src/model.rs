use crate::developer::CommandOrigin;
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Default new-game rules; each campaign retains its content's explicit version.
pub const RULES_VERSION: u32 = 22;
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
    pub id: CommitmentId,
    pub policy: PolicyId,
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
    pub energy_produced: Energy,
    pub energy_imported: Energy,
    pub energy_used: Energy,
    pub food_produced: Food,
    pub food_imported: Food,
    pub food_consumed: Food,
    pub food_lost: Food,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserves: Option<NationalReserves>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cash_management: Option<CashManagement>,
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
    pub ruling_party: PartyId,
    pub coalition: Vec<PartyId>,
    pub executive: String,
    pub seats: BTreeMap<PartyId, u16>,
    pub next_election: Option<u32>,
    pub government_number: u32,
    pub projects: Vec<PolicyInstance>,
    pub energy_stock: Energy,
    pub food_stock: Food,
    pub report: DailyReport,
    pub history: Vec<Observation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub markets: Option<Markets>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub financing: Option<Financing>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constitution: Option<Constitution>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commitments: Option<Vec<CommitmentRecord>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cabinet: Option<Cabinet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub electoral_authority: Option<ElectoralAuthority>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subnational: Option<SubnationalState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub territorial: Option<TerritorialState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undertakings: Option<Undertakings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fiscal_plans: Option<FiscalPlans>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub income_tax_report: Option<IncomeTaxReport>,
}
impl Country {
    /// Resolve a pending project owned by this country. Completed projects remain in history.
    pub fn project(&self, id: &CommitmentId) -> Option<&PolicyInstance> {
        self.projects.iter().find(|project| &project.id == id)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Action {
    ProcureReserve {
        good: GoodKind,
        days: u32,
    },
    CancelReserve {
        request: ReserveOrderId,
    },
    HoldCash {
        target: Money,
        days: u32,
    },
    ReleaseCash {
        request: CashHoldId,
    },
    ProposeBudget {
        year: u32,
        targets: FiscalTargets,
    },
    WithdrawBudget {
        request: BudgetBillId,
    },
    DeclareUndertaking {
        template: String,
        project: Option<CommitmentId>,
    },
    WithdrawUndertaking {
        request: UndertakingId,
    },
    ReformTerritory {
        target: TerritorialForm,
        contested: bool,
    },
    WithdrawTerritorialReform {
        request: TerritorialOrderId,
    },
    RequisitionRegionalFunds {
        region: RegionId,
        amount: Money,
    },
    RegionalOrder {
        region: RegionId,
        kind: RegionalOrderKind,
    },
    CancelRegionalOrder {
        order: RegionalOrderId,
    },
    RequestElection,
    WithdrawElectionRequest {
        request: ElectionRequestId,
    },
    Appoint {
        office: OfficeId,
        person: PersonId,
    },
    DismissMinister {
        office: OfficeId,
    },
    CancelAppointment {
        request: AppointmentId,
    },
    AmendCommitment {
        request: CommitmentId,
        value: i32,
    },
    CancelCommitment {
        request: CommitmentId,
    },
    Enact {
        policy: PolicyId,
        value: i32,
    },
    Borrow {
        amount: Money,
    },
    Reconstitute {
        target: Regime,
        contested: bool,
    },
    CancelConstitution {
        request: TransitionId,
    },
    EmergencyBorrow {
        amount: Money,
    },
    CancelBorrow {
        request: LoanId,
    },
    Repay {
        amount: Money,
    },
    Developer {
        target: CountryId,
        edit: DeveloperEdit,
    },
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<EventId>,
    pub day: u32,
    pub country: CountryId,
    pub important: bool,
    pub text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct JournalEntry {
    pub command: Command,
    pub ai: bool,
    #[serde(default, skip_serializing_if = "crate::developer::is_false")]
    pub developer: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_bill: Option<BudgetBillId>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_sequences: Option<BTreeMap<CountryId, u64>>,
    #[serde(skip)]
    pub(crate) event_error: Option<String>,
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
        let start_day = content.start_day()?;
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
                        next_leadership_review: d
                            .government
                            .leadership_review_days
                            .map(|days| start_day + days),
                        budget: BudgetPeriod {
                            start_day,
                            end_day: crate::budget::next_fiscal_end(start_day),
                            opening_cash: accounts.treasury,
                            opening_day_settled: content.fiscal_plans.as_ref().map(|_| false),
                            forecast: BudgetForecast {
                                interest_accrual: None,
                                daily_revenue: crate::income_taxes::flat_withholding(
                                    Money(d.gdp / 365 * 70 / 100),
                                    d.tax,
                                    d.capacity,
                                ),
                                daily_spending: Money(
                                    crate::budget::daily_policy_spending(
                                        Money(d.gdp / 365),
                                        d.services,
                                    )
                                    .0 + crate::budget::daily_policy_spending(
                                        Money(d.gdp / 365),
                                        d.welfare,
                                    )
                                    .0,
                                ),
                                daily_interest: crate::interest::minimum_daily_interest(Money(
                                    d.gdp / 3,
                                )),
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
                        next_election: d.regime.competitive().then_some(start_day + 730),
                        government_number: 1,
                        projects: Vec::new(),
                        commitments: content.commitments.as_ref().map(|_| Vec::new()),
                        income_tax_report: None,
                        fiscal_plans: content
                            .fiscal_plans
                            .as_ref()
                            .map(|_| FiscalPlans::new(start_day)),
                        cabinet: content
                            .cabinet
                            .as_ref()
                            .map(|rules| Cabinet::new(rules, &d.id, start_day)),
                        undertakings: content
                            .undertakings
                            .as_ref()
                            .map(|_| Undertakings::new(start_day)),
                        territorial: content
                            .territorial
                            .as_ref()
                            .map(|_| TerritorialState::new(start_day)),
                        subnational: content.subnational.as_ref().map(|rules| {
                            SubnationalState::new(rules.charter(&d.id).expect("charter"), start_day)
                        }),
                        electoral_authority: content.elections.as_ref().map(|rules| {
                            ElectoralAuthority::new(rules, d.regime.clone(), start_day)
                        }),
                        reserves: content.reserves.as_ref().map(|_| NationalReserves::new()),
                        energy_stock: Energy::ZERO,
                        food_stock: Food::ZERO,
                        report: DailyReport::default(),
                        history: Vec::new(),
                        markets: content.markets.as_ref().map(Markets::new),
                        constitution: content
                            .constitutions
                            .as_ref()
                            .map(|_| Constitution::new(d.regime.clone(), start_day)),
                        cash_management: content
                            .cash_management
                            .as_ref()
                            .map(|_| CashManagement::default()),
                        financing: content
                            .finance
                            .as_ref()
                            .map(|rules| Financing::with_rules(d, start_day, rules)),
                    },
                )
            })
            .collect();
        let mut world = Self {
            rules: content.rules,
            content_hash: content.hash(),
            seed,
            scenario,
            day: start_day,
            player: player.clone(),
            countries,
            event_sequences: (content.rules >= 22).then(|| {
                content
                    .countries
                    .iter()
                    .map(|d| (d.id.clone(), 1))
                    .collect()
            }),
            event_error: None,
            next_command: 1,
            next_project: 1,
            journal: Vec::new(),
            events: Vec::new(),
            ledger: Vec::new(),
            milestone_reported: false,
        };
        for (id, country) in &mut world.countries {
            country.budget.forecast = country.forecast(
                content.country(id).expect("definition"),
                content.income_taxes.as_ref(),
            );
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
        self.apply(content, command, CommandOrigin::Player)
    }
    pub fn issue(&mut self, content: &Content, action: Action) -> Result<(), String> {
        self.apply(
            content,
            self.command(self.player.clone(), action),
            CommandOrigin::Player,
        )
    }
    pub(crate) fn event(&mut self, country: &CountryId, important: bool, text: impl Into<String>) {
        let id = match self.allocate_event_reference(country) {
            Ok(id) => id,
            Err(reason) => {
                self.event_error.get_or_insert(reason);
                return;
            }
        };
        self.events.push(Event {
            id,
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
            amount.max(0).min(c.available_cash().0)
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
        if self.rules != content.rules || self.content_hash != content.hash() {
            return Err("Rules or content pack mismatch".into());
        }
        let start_day = content.start_day()?;
        if self.day < start_day
            || self.day > MAX_DAY
            || !self.countries.contains_key(&self.player)
            || self.countries.len() != content.countries.len()
            || self.next_command != self.journal.len() as u64 + 1
        {
            return Err("Invalid world clock, player, roster or command sequence".into());
        }
        self.validate_event_references(content)?;
        let mut previous_day = start_day;
        for (index, entry) in self.journal.iter().enumerate() {
            let command = &entry.command;
            let valid_action = match &command.action {
                Action::ProcureReserve { days, .. } => content
                    .reserves
                    .as_ref()
                    .is_some_and(|r| (1..=r.capacity_days).contains(days)),
                Action::CancelReserve { request } => {
                    content.reserves.is_some()
                        && request.0 > 0
                        && request.0 < command.id
                        && self.journal.get(request.0 as usize - 1).is_some_and(|e| {
                            e.command.issuer == command.issuer
                                && matches!(e.command.action, Action::ProcureReserve { .. })
                        })
                }
                Action::HoldCash { target, days } => content
                    .cash_management
                    .as_ref()
                    .is_some_and(|r| target.0 > 0 && (1..=r.max_days).contains(days)),
                Action::ReleaseCash { request } => {
                    content.cash_management.is_some() && request.0 > 0 && request.0 < command.id
                }
                Action::ProposeBudget { year, targets } => {
                    content.fiscal_plans.is_some()
                        && (2000..=2099).contains(year)
                        && targets.valid(content)
                }
                Action::WithdrawBudget { request } => {
                    content.fiscal_plans.is_some()
                        && request.0 > 0
                        && request.0 < command.id
                        && self.journal.get(request.0 as usize - 1).is_some_and(|e| {
                            e.command.issuer == command.issuer
                                && matches!(e.command.action, Action::ProposeBudget { .. })
                        })
                }
                Action::DeclareUndertaking { template, .. } => content
                    .undertakings
                    .as_ref()
                    .is_some_and(|r| r.template(template).is_some()),
                Action::WithdrawUndertaking { request } => {
                    content.undertakings.is_some()
                        && request.0 > 0
                        && request.0 < command.id
                        && self.journal.get(request.0 as usize - 1).is_some_and(|e| {
                            e.command.issuer == command.issuer
                                && matches!(e.command.action, Action::DeclareUndertaking { .. })
                        })
                }
                Action::ReformTerritory { .. } => content.territorial.is_some(),
                Action::RequisitionRegionalFunds { region, amount } => {
                    content.territorial.is_some()
                        && amount.0 > 0
                        && content
                            .subnational
                            .as_ref()
                            .and_then(|r| r.charter(&command.issuer))
                            .is_some_and(|c| c.regions.iter().any(|r| r.id == *region))
                }
                Action::WithdrawTerritorialReform { request } => {
                    content.territorial.is_some()
                        && request.0 > 0
                        && request.0 < command.id
                        && self.journal.get(request.0 as usize - 1).is_some_and(|e| {
                            e.command.issuer == command.issuer
                                && matches!(e.command.action, Action::ReformTerritory { .. })
                        })
                }
                Action::RegionalOrder { region, .. } => content
                    .subnational
                    .as_ref()
                    .is_some_and(|rules| rules.mandate(&command.issuer, region).is_some()),
                Action::CancelRegionalOrder { order } => {
                    content.subnational.is_some()
                        && order.0 > 0
                        && order.0 < command.id
                        && self.journal.get(order.0 as usize - 1).is_some_and(|entry| {
                            entry.command.issuer == command.issuer
                                && matches!(entry.command.action, Action::RegionalOrder { .. })
                        })
                }
                Action::RequestElection => content.elections.is_some(),
                Action::WithdrawElectionRequest { request } => {
                    content.elections.is_some()
                        && request.0 > 0
                        && request.0 < command.id
                        && self.journal.get(request.0 as usize - 1).is_some_and(|e| {
                            e.command.issuer == command.issuer
                                && matches!(e.command.action, Action::RequestElection)
                        })
                }
                Action::Appoint { office, person } => {
                    content.cabinet.as_ref().is_some_and(|rules| {
                        rules
                            .office(office)
                            .is_some_and(|o| o.country == command.issuer)
                            && rules.candidate(person).is_some_and(|p| p.office == *office)
                    })
                }
                Action::DismissMinister { office } => {
                    content.cabinet.as_ref().is_some_and(|rules| {
                        rules
                            .office(office)
                            .is_some_and(|o| o.country == command.issuer)
                    })
                }
                Action::CancelAppointment { request } => {
                    content.cabinet.is_some()
                        && request.0 > 0
                        && request.0 < command.id
                        && self
                            .journal
                            .get(request.0 as usize - 1)
                            .is_some_and(|entry| {
                                entry.command.issuer == command.issuer
                                    && matches!(entry.command.action, Action::Appoint { .. })
                            })
                }
                Action::Enact { policy, value } => content
                    .policy(policy)
                    .is_some_and(|definition| (definition.min..=definition.max).contains(value)),
                Action::AmendCommitment { request, .. } | Action::CancelCommitment { request } => {
                    content.commitments.is_some()
                        && self
                            .countries
                            .get(&command.issuer)
                            .and_then(|c| c.commitments.as_ref())
                            .is_some_and(|records| {
                                records
                                    .iter()
                                    .any(|r| r.id == *request && r.proposal_command < command.id)
                            })
                }
                Action::Borrow { amount } | Action::Repay { amount } => {
                    (1..=1_000_000_000_000).contains(&amount.0)
                }
                Action::Reconstitute { target, contested } => {
                    content.constitutions.as_ref().is_some_and(|rules| {
                        rules.template(target).is_some() && rules.route(*contested).is_ok()
                    })
                }
                Action::CancelConstitution { request } => {
                    content.constitutions.is_some()
                        && request.0 > 0
                        && request.0 < command.id
                        && self
                            .journal
                            .get(request.0 as usize - 1)
                            .is_some_and(|entry| {
                                entry.command.issuer == command.issuer
                                    && matches!(entry.command.action, Action::Reconstitute { .. })
                            })
                }
                Action::EmergencyBorrow { amount } => {
                    content
                        .finance
                        .as_ref()
                        .and_then(|f| f.emergency.as_ref())
                        .is_some()
                        && (1..=1_000_000_000_000).contains(&amount.0)
                }
                Action::CancelBorrow { request } => {
                    content.finance.is_some()
                        && request.0 > 0
                        && request.0 < command.id
                        && self
                            .journal
                            .get(request.0 as usize - 1)
                            .is_some_and(|origin| {
                                origin.command.issuer == command.issuer
                                    && matches!(origin.command.action, Action::Borrow { .. })
                            })
                }
                Action::Developer { target, edit } => {
                    self.countries.contains_key(target) && edit.validate().is_ok()
                }
            };
            if command.id != index as u64 + 1
                || command.day < previous_day
                || command.day > self.day
                || !self.countries.contains_key(&command.issuer)
                || entry.ai == (command.issuer == self.player)
                || (entry.ai && entry.developer)
                || entry.developer != matches!(command.action, Action::Developer { .. })
                || (entry.budget_bill.is_some()
                    && (content.fiscal_plans.is_none()
                        || entry.developer
                        || !matches!(command.action, Action::Enact { .. })))
                || !valid_action
            {
                return Err(format!("Invalid command journal entry {}", index + 1));
            }
            if let (Some(technologies), Action::Enact { policy, .. }) =
                (&content.technologies, &command.action)
            {
                technologies.check_at(&command.issuer, policy, command.day)?;
            }
            previous_day = command.day;
        }
        for (id, c) in &self.countries {
            let d = content.country(id).ok_or("Unknown country in save")?;
            match (&c.markets, &content.markets) {
                (None, None) => {}
                (Some(markets), Some(definition)) => {
                    markets.validate(definition, c, self.day, start_day)?
                }
                _ => return Err("Country markets differ from the campaign rules/content".into()),
            }
            match (&c.constitution, &content.constitutions) {
                (None, None) => {}
                (Some(state), Some(_)) => state.validate(self, content, id)?,
                _ => return Err("Constitution state differs from campaign rules/content".into()),
            }
            match (&c.financing, &content.finance) {
                (None, None) => {}
                (Some(financing), Some(_)) => financing.validate(self, content, id)?,
                _ => return Err("Country financing differs from campaign rules/content".into()),
            }
            self.validate_reserves(content, id)?;
            self.validate_cash_management(content, id)?;
            self.validate_income_taxes(content, id)?;
            self.validate_fiscal_plans(content, id)?;
            self.validate_commitments(content, id)?;
            self.validate_undertakings(content, id)?;
            self.validate_cabinet(content, id)?;
            self.validate_electoral_authority(content, id)?;
            self.validate_territorial(content, id)?;
            self.validate_subnational(content, id)?;
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
                || !(0..=1_000_000_000).contains(&c.energy_stock.units())
                || !(0..=1_000_000_000).contains(&c.food_stock.units())
                || [
                    c.report.energy_produced,
                    c.report.energy_imported,
                    c.report.energy_used,
                ]
                .iter()
                .any(|v| v.units() > 1_000_000_000)
                || [
                    c.report.food_produced,
                    c.report.food_imported,
                    c.report.food_consumed,
                    c.report.food_lost,
                ]
                .iter()
                .any(|v| v.units() > 1_000_000_000)
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
                    + i128::from(
                        c.constitution
                            .as_ref()
                            .map_or(Money(0), Constitution::reserved_cash)
                            .0,
                    )
                    + i128::from(
                        c.territorial
                            .as_ref()
                            .map_or(Money(0), TerritorialState::reserved_cash)
                            .0,
                    )
                    + i128::from(
                        c.fiscal_plans
                            .as_ref()
                            .map_or(Money(0), FiscalPlans::reserved_cash)
                            .0,
                    )
                    + i128::from(
                        c.reserves
                            .as_ref()
                            .map_or(Money(0), NationalReserves::reserved_cash)
                            .0,
                    )
                    != i128::from(c.committed_cash.0)
                || c.budget.start_day < start_day
                || c.budget.start_day > self.day
                || c.budget.end_day <= self.day
                || c.budget.end_day > MAX_DAY + 366
                || c.budget.opening_day_settled
                    != content
                        .fiscal_plans
                        .as_ref()
                        .map(|_| c.budget.start_day != start_day)
                || (content.rules >= 17) != c.budget.forecast.interest_accrual.is_some()
                || !c.budget.valid_ranges()
                || c.budget.expected_cash() != c.accounts.treasury.0
                || c.budget_reviews.iter().any(|r| {
                    r.period.start_day < start_day
                        || r.period.opening_day_settled
                            != content
                                .fiscal_plans
                                .as_ref()
                                .map(|_| r.period.start_day != start_day)
                        || (content.rules >= 17) != r.period.forecast.interest_accrual.is_some()
                        || !r.period.valid_ranges()
                        || r.period.expected_cash() != r.closing_cash.0
                })
                || c.budget_reviews.len() > 10
                || c.next_leadership_review.is_some_and(|day| day <= self.day)
            {
                return Err(format!(
                    "Invalid household, commitment or budget reconciliation: {}",
                    id.0
                ));
            }
            if c.history.iter().any(|o| {
                o.day < start_day
                    || o.day > self.day
                    || !(0..=100).contains(&o.approval)
                    || !(0..=100).contains(&o.needs)
                    || [o.treasury.0, o.debt.0, o.output.0]
                        .iter()
                        .any(|v| !(0..=1_000_000_000_000).contains(v))
            }) || c.history.windows(2).any(|pair| pair[0].day >= pair[1].day)
            {
                return Err("Invalid statistical history dates or ranges".into());
            }
            let mut pending = std::collections::BTreeSet::new();
            for project in &c.projects {
                let p = content
                    .policy(&project.policy)
                    .ok_or("Missing pending policy definition")?;
                if !pending.insert(&project.policy)
                    || project.id.0 >= self.next_project
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

/// Strict Gregorian date input for the supported bookmark/campaign range.
pub fn parse_date(input: &str) -> Result<u32, String> {
    input.parse::<CivilDate>()?.campaign_day()
}
