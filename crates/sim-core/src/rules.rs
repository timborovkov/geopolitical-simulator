use crate::developer::CommandOrigin;
use crate::*;
use Account::{External, Firms, Treasury};
impl World {
    pub fn preview(
        &self,
        content: &Content,
        country: &CountryId,
        policy: &str,
        value: i32,
    ) -> Result<PolicyPreview, String> {
        let c = self.countries.get(country).ok_or("Unknown country")?;
        let d = content
            .country(country)
            .ok_or("Missing country definition")?;
        let p = content.policy(policy).ok_or("Unknown policy")?;
        if !(p.min..=p.max).contains(&value) {
            return Err(format!("Value must be {}..{}", p.min, p.max));
        }
        if c.projects.iter().any(|p| p.policy == policy) {
            return Err("This policy already has a pending commitment".into());
        }
        d.government.check(c, policy, value)?;
        if policy == "energy" && c.energy_capacity >= 200
            || policy == "agriculture" && c.food_capacity >= 200
            || policy == "administration" && c.administration >= 100
            || policy == "coalition"
                && c.legislative_support
                    .min(c.elite_support)
                    .min(c.council_support)
                    >= 100
        {
            return Err("Already at the program's capacity ceiling".into());
        }
        let current = match policy {
            "income_tax" => c.tax,
            "services" => c.services,
            "welfare" => c.welfare,
            _ => 0,
        };
        if current == value {
            return Err("The proposed level is already in force".into());
        }
        let cost = Money((d.gdp * p.cost_bps / 10000).max(1));
        let available = c.accounts.treasury.0 - c.committed_cash.0;
        if available < cost.0 {
            return Err(format!(
                "Insufficient uncommitted treasury: requires {}m; available {available}m",
                cost.0
            ));
        }
        let annual = match policy {
            "income_tax" => {
                -(d.gdp * 70 / 100 * i64::from(value - c.tax) * i64::from(c.administration) / 10000)
            }
            "services" => d.gdp * i64::from(value - c.services) / 100,
            "welfare" => d.gdp * i64::from(value - c.welfare) / 100,
            "energy" | "agriculture" => d.gdp / 1000,
            _ => 0,
        };
        let consent_delay = if policy == "coalition" {
            0
        } else {
            d.government.consent_days
        };
        let upfront_payment = if consent_delay == 0 {
            cost
        } else {
            Money((cost.0 / 10).max(1))
        };
        let daily = d.gdp / 365;
        let household_impacts = d
            .households
            .iter()
            .map(|h| {
                let change = match policy {
                    "income_tax" => {
                        -(daily * 70 / 100
                            * h.income_share
                            * i64::from(value - c.tax)
                            * i64::from(c.administration)
                            / 1000000)
                    }
                    "services" if h.id == "public" => daily * i64::from(value - c.services) / 100,
                    "welfare" => {
                        daily
                            * i64::from(value - c.welfare)
                            * if h.id == "vulnerable" {
                                80
                            } else if h.id == "workers" {
                                20
                            } else {
                                0
                            }
                            / 10000
                    }
                    _ => 0,
                };
                HouseholdImpact {
                    group: h.name.clone(),
                    daily_disposable_change: Money(change),
                }
            })
            .collect();
        Ok(PolicyPreview {
            cost,
            delay: consent_delay + p.days + (100 - c.administration) as u32 / 2,
            annual_budget_change: Money(annual),
            route: d.government.label(),
            consent_delay,
            upfront_payment,
            household_impacts,
            explanation: format!(
                "{} Affects {}. Forecast assumes opening GDP, current collection, funded wages and no future shocks. Consent is rechecked before reserved capital is paid; delivery begins afterward.",
                p.description, p.groups
            ),
        })
    }
    pub(crate) fn apply(
        &mut self,
        content: &Content,
        command: Command,
        origin: CommandOrigin,
    ) -> Result<(), String> {
        if self.content_hash != content.hash() {
            return Err("Command content pack differs from campaign".into());
        }
        if command.id != self.next_command || command.day != self.day {
            return Err("Stale or duplicate command: refresh the current state".into());
        }
        let next_command = self
            .next_command
            .checked_add(1)
            .ok_or("Command ID space exhausted")?;
        let ai = origin == CommandOrigin::Ai;
        let developer = origin == CommandOrigin::Developer;
        if developer != matches!(command.action, Action::Developer { .. }) {
            return Err("Developer edits require an explicitly enabled developer host; normal player/AI commands cannot use them".into());
        }
        if !ai && command.issuer != self.player {
            return Err("You do not control this government".into());
        }
        let c = self
            .countries
            .get(&command.issuer)
            .ok_or("Unknown issuing government")?;
        let d = content
            .country(&command.issuer)
            .ok_or("Missing country content")?;
        let id = command.issuer.clone();
        match &command.action {
            Action::Enact { policy, value } => {
                let preview = self.preview(content, &id, policy, *value)?;
                let next_project = self
                    .next_project
                    .checked_add(1)
                    .ok_or("Project ID space exhausted")?;
                let reserved = Money(preview.cost.0 - preview.upfront_payment.0);
                self.pay(
                    &id,
                    Treasury,
                    Firms,
                    preview.upfront_payment.0,
                    "Policy proposal/administration payment",
                );
                let c = self.countries.get_mut(&id).expect("country");
                c.committed_cash.0 += reserved.0;
                c.budget.capital.0 += preview.upfront_payment.0;
                c.projects.push(PolicyInstance {
                    id: self.next_project,
                    policy: policy.clone(),
                    value: *value,
                    start_day: self.day + preview.consent_delay,
                    consent_day: self.day + preview.consent_delay,
                    due_day: self.day + preview.delay,
                    cost: preview.cost,
                    reserved,
                    stage: if preview.consent_delay == 0 {
                        PolicyStage::Implementing
                    } else {
                        PolicyStage::AwaitingConsent
                    },
                });
                self.next_project = next_project;
                self.event(&id,true,format!("Proposed {} via {}. Paid {}m upfront, reserved {}m; consent {} and operating due {}.",content.policy(policy).expect("policy").name,preview.route,preview.upfront_payment.0,reserved.0,date(self.day+preview.consent_delay),date(self.day+preview.delay)));
            }
            Action::Borrow { amount } => {
                if content.finance.is_some() {
                    self.request_borrowing(content, &id, LoanId(command.id), *amount)?;
                } else {
                    if amount.0 <= 0 || amount.0 > d.gdp / 10 {
                        return Err(
                            "Borrowing must be positive and at most 10% of opening GDP per issue"
                                .into(),
                        );
                    }
                    if c.debt.0 + amount.0 > d.gdp * 3 / 2 {
                        return Err("Creditors refuse: debt ceiling is 150% of opening GDP".into());
                    }
                    if amount.0 > c.accounts.external.0 {
                        return Err("External creditor lacks available funds".into());
                    }
                    self.pay(
                        &id,
                        External,
                        Treasury,
                        amount.0,
                        "External loan principal received",
                    );
                    let c = self.countries.get_mut(&id).expect("country");
                    c.debt.0 += amount.0;
                    c.budget.borrowing.0 += amount.0;
                    self.event(&id,true,format!("Borrowed {}m from external creditors at 4% annual interest. Principal is due on voluntary repayment.",amount.0));
                }
            }
            Action::EmergencyBorrow { amount } => {
                self.emergency_borrowing(content, &id, LoanId(command.id), *amount)?
            }
            Action::CancelBorrow { request } => self.cancel_borrowing(&id, *request)?,
            Action::Repay { amount } => {
                if amount.0 <= 0
                    || amount.0 > c.debt.0
                    || amount.0 > c.accounts.treasury.0 - c.committed_cash.0
                {
                    return Err("Repayment must be positive and covered by uncommitted cash and debt principal".into());
                }
                self.pay(
                    &id,
                    Treasury,
                    External,
                    amount.0,
                    "External debt principal repaid",
                );
                let c = self.countries.get_mut(&id).expect("country");
                c.debt.0 -= amount.0;
                if let Some(financing) = &mut c.financing {
                    financing.repay(*amount);
                }
                c.budget.principal_repaid.0 += amount.0;
                self.event(
                    &id,
                    false,
                    format!("Repaid {}m of debt principal.", amount.0),
                );
            }
            Action::Developer { target, edit } => {
                edit.validate()?;
                let country = self
                    .countries
                    .get_mut(target)
                    .ok_or("Unknown developer target country")?;
                let before = edit.field.read(country);
                edit.apply(country);
                self.event(target, true, format!("DEVELOPER EDIT: {} changed from {before} to {}. Policy costs and delays were bypassed; this campaign is marked developer-modified.", edit.field.name(), edit.value));
            }
        }
        self.journal.push(JournalEntry {
            command,
            ai,
            developer,
        });
        self.next_command = next_command;
        Ok(())
    }
    /// Consent → delivery → production/households → fiscal close → politics → AI.
    pub fn advance(&mut self, content: &Content, days: u32) -> Result<(), String> {
        if self.content_hash != content.hash() {
            return Err("Campaign content differs from loaded pack".into());
        }
        if days > 3660 || self.day.saturating_add(days) > MAX_DAY {
            return Err("Advance 0..3660 days within the 100-year campaign limit".into());
        }
        let review_day = content.review_day()?;
        let ids: Vec<_> = self.countries.keys().cloned().collect();
        for _ in 0..days {
            self.day += 1;
            for id in &ids {
                self.resolve_consents(content, id);
                self.resolve_borrowing(content, id);
                self.complete_projects(id);
                self.settle(content, id);
                self.review_budget(content, id);
                if self.day.is_multiple_of(30)
                    || self.countries[id].next_election == Some(self.day)
                    || self.countries[id].next_leadership_review == Some(self.day)
                {
                    self.politics(content, id);
                }
            }
            if self.day.is_multiple_of(30) {
                for id in &ids {
                    if id != &self.player {
                        self.ai_turn(content, id);
                    }
                }
            }
            self.ledger.retain(|e| e.day + 30 >= self.day);
            if self.day >= review_day && !self.milestone_reported {
                let c = self.player_country();
                let won = c.report.food_coverage >= 90
                    && c.report.energy_coverage >= 90
                    && c.approval >= 50
                    && c.arrears.0 == 0
                    && c.debt.0 <= content.country(&self.player).expect("player").gdp;
                let message = if self.is_developer_modified() {
                    if won {
                        "Five-year review: targets met in a developer-modified campaign. Country play continues."
                    } else {
                        "Five-year review: targets missed in a developer-modified campaign. Country play continues."
                    }
                } else if won {
                    "Five-year review: mandate achieved. Continue the country campaign."
                } else {
                    "Five-year review: mandate missed. Inspect household needs, finances and authority; country play continues."
                };
                self.event(&self.player.clone(), true, message);
                self.milestone_reported = true;
            }
        }
        Ok(())
    }
    pub fn advance_to_event(&mut self, content: &Content) -> Result<u32, String> {
        self.advance(content, 0)?;
        let start = self.day;
        for _ in 0..365.min(MAX_DAY.saturating_sub(self.day)) {
            self.advance(content, 1)?;
            if self
                .events
                .iter()
                .any(|e| e.day == self.day && e.country == self.player && e.important)
            {
                break;
            }
        }
        Ok(self.day - start)
    }
    fn complete_projects(&mut self, id: &CountryId) {
        let c = self.countries.get_mut(id).expect("country");
        let mut completed = Vec::new();
        c.projects.retain(|p| {
            if p.stage == PolicyStage::Implementing && p.due_day <= self.day {
                completed.push(p.clone());
                false
            } else {
                true
            }
        });
        for p in completed {
            let c = self.countries.get_mut(id).expect("country");
            match p.policy.as_str() {
                "income_tax" => c.tax = p.value,
                "services" => c.services = p.value,
                "welfare" => c.welfare = p.value,
                "energy" => c.energy_capacity = (c.energy_capacity + 15).min(200),
                "agriculture" => c.food_capacity = (c.food_capacity + 15).min(200),
                "administration" => c.administration = (c.administration + 8).min(100),
                "coalition" => {
                    c.legislative_support = (c.legislative_support + 12).min(100);
                    c.elite_support = (c.elite_support + 12).min(100);
                    c.council_support = (c.council_support + 12).min(100);
                }
                _ => unreachable!("validated policy"),
            }
            self.event(
                id,
                true,
                format!(
                    "{} is operating. Inspect household and fiscal execution for realized effects.",
                    p.policy
                ),
            );
        }
    }
    fn settle(&mut self, content: &Content, id: &CountryId) {
        let d = content.country(id).expect("definition");
        for h in &mut self.countries.get_mut(id).expect("country").households {
            h.income = Money(0);
            h.tax = Money(0);
            h.consumption = Money(0);
        }
        let quarter = (self.day - 1) / 90;
        let sample =
            crate::stable_hash(format!("{}:{}:{quarter}:supply-v2", self.seed, id.0).as_bytes());
        let shock = if (self.day - 1) % 90 < 20 && sample.is_multiple_of(3) {
            10 + (sample % 16) as i32
        } else {
            0
        };
        if (self.day - 1).is_multiple_of(90) && shock > 0 {
            self.event(id,true,format!("Supply disruption: domestic food and energy capacity down {shock} points for 20 days. Imports require cash and have quantity limits."));
        }
        let c = &self.countries[id];
        let demand = (c.population.0 / 1000) as i64;
        let energy_demand = Energy::new(demand).expect("validated population");
        let food_demand = Food::new(demand).expect("validated population");
        let daily = (d.gdp / 365).max(1);
        let market_rules = content.markets.as_ref();
        let energy_rule = market_rules.map(|rules| rules.good(GoodKind::Energy));
        let food_rule = market_rules.map(|rules| rules.good(GoodKind::Food));
        let energy_price = c
            .markets
            .as_ref()
            .map_or(10000, |markets| markets.energy.next_price_bps);
        let food_price = c
            .markets
            .as_ref()
            .map_or(10000, |markets| markets.food.next_price_bps);
        let opening_energy = c.energy_stock;
        let opening_food = c.food_stock;
        let energy_produced = energy_demand
            .scaled(i64::from((c.energy_capacity - shock).max(0)), 100)
            .expect("bounded capacity")
            .min(
                energy_demand
                    .scaled(
                        i64::from(energy_rule.map_or(3, |rule| rule.storage_days)),
                        1,
                    )
                    .expect("bounded demand")
                    .saturating_sub(c.energy_stock),
            );
        let food_produced = food_demand
            .scaled(i64::from((c.food_capacity - shock).max(0)), 100)
            .expect("bounded capacity")
            .min(
                food_demand
                    .scaled(i64::from(food_rule.map_or(11, |rule| rule.storage_days)), 1)
                    .expect("bounded demand")
                    .saturating_sub(c.food_stock),
            );
        let energy_available = c
            .energy_stock
            .checked_add(energy_produced)
            .expect("bounded energy storage");
        let food_available = c
            .food_stock
            .checked_add(food_produced)
            .expect("bounded food storage");
        let buy_import = |world: &mut Self, quantity: i64, price: u32, reason: &str| {
            if quantity == 0 {
                return 0;
            }
            let base_cost = (daily * quantity / demand / 5).max(1);
            let cost = (base_cost * i64::from(price) / 10000).max(1);
            let paid = world.pay(id, Firms, External, cost, reason);
            quantity * paid / cost
        };
        let energy_imported = Energy::new(buy_import(
            self,
            energy_demand
                .saturating_sub(energy_available)
                .min(
                    energy_demand
                        .scaled(
                            i64::from(energy_rule.map_or(2500, |rule| rule.import_limit_bps)),
                            10000,
                        )
                        .expect("positive ratio"),
                )
                .units(),
            energy_price,
            "Bounded energy imports",
        ))
        .expect("nonnegative imports");
        let food_imported = Food::new(buy_import(
            self,
            food_demand
                .saturating_sub(food_available)
                .min(
                    food_demand
                        .scaled(
                            i64::from(food_rule.map_or(3500, |rule| rule.import_limit_bps)),
                            10000,
                        )
                        .expect("positive ratio"),
                )
                .units(),
            food_price,
            "Bounded food imports",
        ))
        .expect("nonnegative imports");
        let energy_supply = energy_available
            .checked_add(energy_imported)
            .expect("bounded energy imports");
        let energy_used = energy_supply.min(energy_demand);
        let energy_coverage = energy_used.coverage(energy_demand);
        let output = daily * i64::from(energy_coverage) / 100;
        self.pay(
            id,
            External,
            Firms,
            output * 15 / 100,
            "Service exports: external demand capped at 15% of realized output",
        );
        let income_weights: Vec<_> = d.households.iter().map(|h| h.income_share).collect();
        let wages = self.pay_income(
            id,
            Firms,
            output * 70 / 100,
            &income_weights,
            "Private household income",
        );
        let c = &self.countries[id];
        let effective_tax = i64::from(c.tax) * i64::from(c.administration);
        let mut revenue = 0;
        for index in 0..d.households.len() {
            let due = self.countries[id].households[index].income.0 * effective_tax / 10000;
            let paid = self.pay_group(id, index, Treasury, due, "Income tax on private income");
            self.countries.get_mut(id).expect("country").households[index].tax = Money(paid);
            revenue += paid;
        }
        let c = &self.countries[id];
        let services_due = daily * i64::from(c.services) / 100;
        let welfare_due = daily * i64::from(c.welfare) / 100;
        let interest_due = (c.debt.0 * 4 / 100 / 365).max(i64::from(c.debt.0 > 0));
        let maintenance_due = daily
            * i64::from(
                (c.energy_capacity - c.opening_energy_capacity).max(0)
                    + (c.food_capacity - c.opening_food_capacity).max(0),
            )
            / 15
            / 1000;
        let interest = self.pay(
            id,
            Treasury,
            External,
            interest_due,
            "Debt interest at 4% annual rate",
        );
        let public_weights: Vec<_> = d
            .households
            .iter()
            .map(|h| i64::from(h.id == "public"))
            .collect();
        let service_pay = self.pay_income(
            id,
            Treasury,
            services_due,
            &public_weights,
            "Public service household wages/procurement",
        );
        let welfare_weights: Vec<_> = d
            .households
            .iter()
            .map(|h| {
                if h.id == "vulnerable" {
                    80
                } else if h.id == "workers" {
                    20
                } else {
                    0
                }
            })
            .collect();
        let welfare_pay = self.pay_income(
            id,
            Treasury,
            welfare_due,
            &welfare_weights,
            "Targeted welfare transfers",
        );
        let maintenance = self.pay(
            id,
            Treasury,
            Firms,
            maintenance_due,
            "Added supply capacity maintenance",
        );
        let unpaid = interest_due - interest + services_due - service_pay + welfare_due
            - welfare_pay
            + maintenance_due
            - maintenance;
        let c = &self.countries[id];
        let previous_arrears = c.arrears.0;
        let external_arrears = c.arrears_external.0;
        let household_arrears = c.arrears_households.0;
        let firm_arrears = c.arrears_firms.0;
        let paid_external = self.pay(
            id,
            Treasury,
            External,
            external_arrears.min((daily / 30).max(1)),
            "Interest arrears paid to original creditors",
        );
        let claims: Vec<_> = self.countries[id]
            .households
            .iter()
            .map(|h| h.arrears.0)
            .collect();
        let paid_households = if household_arrears > 0 {
            self.pay_income(
                id,
                Treasury,
                household_arrears.min((daily / 30).max(1)),
                &claims,
                "Household arrears paid to original groups",
            )
        } else {
            0
        };
        let claims_paid = if household_arrears > 0 {
            apportion(paid_households, &claims)
        } else {
            vec![0; claims.len()]
        };
        let service_claims = apportion(services_due - service_pay, &public_weights);
        let welfare_claims = apportion(welfare_due - welfare_pay, &welfare_weights);
        for (index, h) in self
            .countries
            .get_mut(id)
            .expect("country")
            .households
            .iter_mut()
            .enumerate()
        {
            h.arrears.0 += service_claims[index] + welfare_claims[index] - claims_paid[index];
        }
        let paid_firms = self.pay(
            id,
            Treasury,
            Firms,
            firm_arrears.min((daily / 30).max(1)),
            "Maintenance arrears paid to original firms",
        );
        let arrears_paid = paid_external + paid_households + paid_firms;
        let food_supply = food_available
            .checked_add(food_imported)
            .expect("bounded food imports");
        let (consumption, food_consumed, food_requested) =
            self.settle_households(content, id, food_supply, food_demand);
        let food_left = food_supply
            .checked_sub(food_consumed)
            .expect("delivery limited by supply");
        let food_lost = food_left
            .scaled(
                i64::from(food_rule.map_or(100, |rule| rule.loss_bps)),
                10000,
            )
            .expect("positive loss ratio");
        let c = self.countries.get_mut(id).expect("country");
        c.energy_stock = energy_supply
            .checked_sub(energy_used)
            .expect("usage limited by supply");
        c.food_stock = food_left
            .checked_sub(food_lost)
            .expect("loss limited by stock");
        c.arrears_households.0 +=
            services_due - service_pay + welfare_due - welfare_pay - paid_households;
        c.arrears_external.0 += interest_due - interest - paid_external;
        c.arrears_firms.0 += maintenance_due - maintenance - paid_firms;
        c.arrears.0 += unpaid - arrears_paid;
        let service_delivery = if services_due == 0 {
            0
        } else {
            (service_pay * i64::from(c.administration) / services_due) as i32
        };
        for (h, definition) in c.households.iter_mut().zip(&d.households) {
            let (paid, planned) = if h.id == "public" {
                (
                    wages * definition.income_share / 100 + service_pay,
                    daily * 70 / 100 * definition.income_share / 100 + services_due,
                )
            } else {
                (wages, daily * 70 / 100)
            };
            h.employed.0 = h.working_age.0 * 90 / 100 * paid as u64 / planned.max(1) as u64;
        }
        c.budget.revenue.0 += revenue;
        c.budget.payments.0 += service_pay + welfare_pay + maintenance + arrears_paid + interest;
        c.report = DailyReport {
            revenue: Money(revenue),
            spending: Money(service_pay + welfare_pay + maintenance + arrears_paid),
            interest: Money(interest),
            unpaid: Money(unpaid),
            wages: Money(wages),
            consumption,
            energy_coverage,
            food_coverage: food_consumed.coverage(food_demand),
            service_delivery,
            output: Money(output),
            shock,
            energy_produced,
            energy_imported,
            energy_used,
            food_produced,
            food_imported,
            food_consumed,
            food_lost,
        };
        if let Some(markets) = &mut c.markets {
            markets.energy.last = Some(MarketSettlement {
                day: self.day,
                price_bps: energy_price,
                physical_need: energy_demand,
                requested_use: energy_demand,
                opening_stock: opening_energy,
                production: energy_produced,
                imports: energy_imported,
                supply: energy_supply,
                delivered: energy_used,
                loss: Energy::ZERO,
                closing_stock: c.energy_stock,
            });
            markets.food.last = Some(MarketSettlement {
                day: self.day,
                price_bps: food_price,
                physical_need: food_demand,
                requested_use: food_requested,
                opening_stock: opening_food,
                production: food_produced,
                imports: food_imported,
                supply: food_supply,
                delivered: food_consumed,
                loss: food_lost,
                closing_stock: c.food_stock,
            });
            markets.energy.next_price_bps = next_market_price(
                energy_price,
                energy_supply.units(),
                energy_demand.units(),
                energy_rule.expect("configured energy rule"),
            )
            .expect("validated price quantities");
            markets.food.next_price_bps = next_market_price(
                food_price,
                food_supply.units(),
                food_requested.units(),
                food_rule.expect("configured food rule"),
            )
            .expect("validated price quantities");
        }
        if unpaid > 0 && previous_arrears == 0 {
            self.event(id,true,"Treasury cannot meet unreserved obligations: reduced payments and recorded arrears. Borrow or adjust spending; reserved policy capital is protected.");
        }
    }
    fn politics(&mut self, content: &Content, id: &CountryId) {
        let d = content.country(id).expect("definition");
        let c = self.countries.get_mut(id).expect("country");
        let needs = (c.report.energy_coverage + c.report.food_coverage) / 2;
        let service_score = (c.services * 5).min(90) * c.report.service_delivery / 100;
        for h in &mut c.households {
            let purchasing = if h.basket_cost.0 == 0 {
                100
            } else {
                ((h.income.0 - h.tax.0) * 100 / h.basket_cost.0).clamp(0, 150) as i32
            };
            let tax_burden = if h.income.0 == 0 {
                0
            } else {
                (h.tax.0 * 100 / h.income.0) as i32
            };
            let unemployment = 100
                - (h.employed.0 * 100)
                    .checked_div(h.working_age.0)
                    .unwrap_or(100) as i32;
            let target = (20
                + (h.food_coverage - 70)
                + (c.report.energy_coverage - 90) / 2
                + service_score / 3
                + purchasing / 5
                - tax_burden / 2
                - unemployment / 3
                - if c.arrears.0 > 0 { 20 } else { 0 })
            .clamp(5, 95);
            h.approval += (target - h.approval).signum() * ((target - h.approval).abs() / 4).max(1);
        }
        c.approval = (c
            .households
            .iter()
            .map(|h| h.approval as u64 * h.population.0)
            .sum::<u64>()
            / c.population.0) as i32;
        let owners = c
            .households
            .iter()
            .find(|h| h.id == "owners")
            .expect("owners")
            .approval;
        let public = c
            .households
            .iter()
            .find(|h| h.id == "public")
            .expect("public")
            .approval;
        let council_target = match d.government.route {
            DecisionRoute::PartyCouncil => (public + c.administration) / 2,
            DecisionRoute::RoyalDecree => (owners + public) / 2,
            _ => (c.approval + c.administration) / 2,
        } - if c.arrears.0 > 0 { 20 } else { 0 };
        c.council_support =
            (c.council_support + (council_target - c.council_support).signum() * 2).clamp(0, 100);
        let elite_target =
            (40 + owners / 3 + public / 4 - if c.arrears.0 > 0 { 30 } else { 0 }).clamp(5, 95);
        c.elite_support += (elite_target - c.elite_support).signum();
        if d.regime.competitive() && self.day.is_multiple_of(30) {
            let base: i32 = c.coalition.iter().map(|id| i32::from(c.seats[id])).sum();
            let target = (base + (c.approval - 50) / 5).clamp(0, 100);
            c.legislative_support += (target - c.legislative_support).signum();
            c.confidence_crises = if c.legislative_support < d.government.threshold {
                c.confidence_crises + 1
            } else {
                0
            };
            if c.confidence_crises >= 3 && d.regime == Regime::Parliamentary {
                c.next_election = Some(self.day + 1);
                c.confidence_crises = 0;
            }
        }
        let leadership_review = c.next_leadership_review == Some(self.day);
        if leadership_review {
            c.next_leadership_review = d
                .government
                .leadership_review_days
                .map(|days| self.day + days);
        }
        let mut transition = None;
        if c.next_election.is_some_and(|day| day <= self.day) {
            let current = c.ruling_party.clone();
            let opposition: Vec<_> = d.parties.iter().filter(|p| p.id != current).collect();
            if !opposition.is_empty() {
                let votes = c.approval.clamp(15, 80) as u16;
                c.seats.insert(current.clone(), votes);
                let rest = 100 - votes;
                let opposition_count = opposition.len() as u16;
                for (i, p) in opposition.iter().enumerate() {
                    c.seats.insert(
                        p.id.clone(),
                        rest / opposition_count + u16::from(i < (rest % opposition_count) as usize),
                    );
                }
                if votes < 50 {
                    let successor = opposition
                        .iter()
                        .max_by_key(|p| c.seats[&p.id])
                        .expect("opposition");
                    c.ruling_party = successor.id.clone();
                    c.coalition = opposition.iter().map(|p| p.id.clone()).collect();
                    c.executive = format!("{} cabinet {}", successor.name, c.government_number + 1);
                    // Opposition parties agree a government; named historical leaders are not kept alive forever.
                    c.legislative_support = i32::from(rest);
                    c.approval = 55;
                    for h in &mut c.households {
                        h.approval = 55;
                    }
                    c.government_number += 1;
                    transition = Some(format!(
                        "Election: {votes}% for the outgoing bloc. {} forms a generated successor cabinet. Country play continues under its mandate.",
                        successor.name
                    ));
                } else {
                    c.legislative_support = i32::from(votes);
                    transition = Some(format!(
                        "Election: governing bloc returned with {votes}% of the modeled vote. Seat allocation uses the simplified 100-seat scenario rule."
                    ));
                }
            }
            c.next_election = Some(self.day + 1460);
        } else if !d.regime.competitive()
            && (c.elite_support < 40
                || leadership_review && c.council_support < d.government.council_threshold.max(45))
        {
            c.government_number += 1;
            c.executive = format!("Generated leadership council {}", c.government_number);
            c.elite_support = 65;
            c.council_support = 65;
            transition=Some("Elite coalition replaced the leadership after sustained loss of backing. Institutional route remains in force; country play continues.".into());
        }
        c.history.push(Observation {
            day: self.day,
            treasury: c.accounts.treasury,
            debt: c.debt,
            approval: c.approval,
            needs,
            output: c.report.output,
        });
        if c.history.len() > 120 {
            c.history.remove(0);
        }
        let summary = format!(
            "Monthly review: needs {needs}%, approval {}%, revenue {}m/day, payments {}m/day, arrears {}m. Support follows household needs, funded service delivery and disposable income.",
            c.approval,
            c.report.revenue.0,
            c.report.spending.0 + c.report.interest.0,
            c.arrears.0
        );
        let confidence_warning = c.confidence_crises == 1 || c.confidence_crises == 2;
        let warning_count = c.confidence_crises;
        if confidence_warning {
            self.event(id,true,format!("Confidence warning {warning_count}/3: parliamentary backing is below the ordinary-policy threshold. Negotiate support before the third weak review triggers an early election in parliamentary systems."));
        }
        if let Some(message) = transition {
            self.event(id, true, message);
        }
        self.event(id, false, summary);
    }
    fn ai_turn(&mut self, content: &Content, id: &CountryId) {
        let c = &self.countries[id];
        let d = content.country(id).expect("country");
        if c.financing
            .as_ref()
            .is_some_and(|state| state.pending.is_some())
        {
            self.event(id, false, "AI review: awaiting institutional borrowing decision; pending principal is unavailable cash.");
            return;
        }
        let candidate = if c.accounts.treasury.0 - c.committed_cash.0 < d.gdp / 100
            || c.arrears.0 > 0
        {
            Action::Borrow {
                amount: Money(d.gdp / 20),
            }
        } else {
            let policy = if d.regime.competitive() && c.legislative_support < d.government.threshold
                || !d.regime.competitive()
                    && (c.elite_support < d.government.threshold + 5
                        || c.council_support < d.government.council_threshold + 10)
            {
                "coalition"
            } else if c.energy_capacity < 110 {
                "energy"
            } else if c.food_capacity < 100 {
                "agriculture"
            } else if c.administration < 85 {
                "administration"
            } else if c.debt.0 > d.gdp / 5 && c.accounts.treasury.0 > d.gdp / 10 {
                let command = self.command(
                    id.clone(),
                    Action::Repay {
                        amount: Money(d.gdp / 100),
                    },
                );
                let _ = self.apply(content, command, CommandOrigin::Ai);
                return;
            } else {
                return;
            };
            Action::Enact {
                policy: policy.into(),
                value: 1,
            }
        };
        let reason = format!(
            "AI review: chose {candidate:?} to address the highest-ranked financing, support or supply constraint."
        );
        let command = self.command(id.clone(), candidate);
        match self.apply(content, command, CommandOrigin::Ai) {
            Ok(()) => self.event(id, false, reason),
            Err(error) => self.event(
                id,
                false,
                format!("AI candidate rejected by shared rules: {error}"),
            ),
        }
    }
}
