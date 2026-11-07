//! Read-only aggregation of realized transfers, never spending authorization.
use eframe::egui;
use sim_core::{Account, Content, CountryId, World, date};

const ACCOUNTS: [Account; 4] = [
    Account::Treasury,
    Account::Households,
    Account::Firms,
    Account::External,
];
const NAMES: [&str; 4] = ["Treasury", "Households", "Firms", "External"];
fn index(account: Account) -> usize {
    match account {
        Account::Treasury => 0,
        Account::Households => 1,
        Account::Firms => 2,
        Account::External => 3,
    }
}
struct Summary {
    transfers: [[i64; 4]; 4],
    opening: [i64; 4],
    closing: [i64; 4],
}
fn summarize(world: &World, country: &CountryId, first: u32) -> Result<Summary, String> {
    let state = world
        .countries
        .get(country)
        .ok_or("Unknown cash-flow country")?;
    let closing = ACCOUNTS.map(|account| state.accounts.balance(account));
    let mut amounts = [[0_i128; 4]; 4];
    for entry in world.ledger.iter().filter(|entry| {
        &entry.country == country
            && entry.day >= sim_core::CampaignDay::new(first)
            && entry.day <= world.day
    }) {
        if entry.amount.0 < 0 {
            return Err("Invalid negative cash transfer".into());
        }
        amounts[index(entry.from)][index(entry.to)] += i128::from(entry.amount.0);
    }
    let mut transfers = [[0; 4]; 4];
    let mut opening = closing.map(i128::from);
    for from in 0..4 {
        for to in 0..4 {
            transfers[from][to] =
                i64::try_from(amounts[from][to]).map_err(|_| "Cash-flow total overflow")?;
            opening[from] += amounts[from][to];
            opening[to] -= amounts[from][to];
        }
    }
    let mut checked_opening = [0; 4];
    for (i, value) in opening.into_iter().enumerate() {
        checked_opening[i] = i64::try_from(value).map_err(|_| "Opening cash overflow")?;
        if checked_opening[i] < 0 {
            return Err("Recorded transfers do not reconcile to nonnegative opening cash".into());
        }
    }
    Ok(Summary {
        transfers,
        opening: checked_opening,
        closing,
    })
}

pub struct CashFlows {
    days: u32,
    route: Option<(usize, usize)>,
}
impl Default for CashFlows {
    fn default() -> Self {
        Self {
            days: 1,
            route: None,
        }
    }
}
impl CashFlows {
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        content: &Content,
        world: &World,
        country: &CountryId,
    ) {
        // This is a normal government view, never the developer truth view.
        if country != &world.player {
            return;
        }
        egui::CollapsingHeader::new("Trace realized cash transfers").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut self.days, 1, "Current day");
                ui.selectable_value(&mut self.days, 30, "Last 30 days");
                if ui.button("All routes").clicked() { self.route = None; }
            });
            let first = world.day.units().saturating_sub(self.days - 1).max(content.start_day().expect("validated bookmark"));
            ui.label(format!("{} through {} inclusive · millions of constant-2000 USD-equivalent game credits", date(first), date(world.day.units())));
            ui.label("Rows pay columns. Select an amount to inspect its recorded transactions. Reservations and unpaid obligations are separate from these cash transfers; transfer totals are not GDP or net public expenditure.");
            let summary = match summarize(world, country, first) { Ok(summary) => summary, Err(error) => { ui.colored_label(egui::Color32::LIGHT_YELLOW, error); return; } };
            let max = summary.transfers.iter().flatten().copied().max().unwrap_or(0).max(1) as f32;
            egui::ScrollArea::horizontal().id_salt("cash_flow_matrix_scroll").show(ui, |ui| {
                egui::Grid::new("cash_flow_matrix").striped(true).min_col_width(95.0).show(ui, |ui| {
                    ui.strong("Payer to recipient");
                    for name in NAMES { ui.strong(name); }
                    ui.end_row();
                    for (from, name) in NAMES.iter().enumerate() {
                        ui.strong(*name);
                        for (to, recipient) in NAMES.iter().enumerate() {
                            let amount = summary.transfers[from][to];
                            if amount == 0 { ui.label("—"); }
                            else {
                                let intensity = (amount as f32 / max).sqrt();
                                let fill = egui::Color32::from_rgb(24, (55.0 + 65.0 * intensity) as u8, (48.0 + 50.0 * intensity) as u8);
                                if ui.add(egui::Button::new(format!("{amount}m")).fill(fill).selected(self.route == Some((from, to))))
                                    .on_hover_text(format!("{name} paid {recipient} {amount}m during the selected period. Select to inspect reasons and dates.")).clicked() { self.route = Some((from, to)); }
                            }
                        }
                        ui.end_row();
                    }
                });
            });
            ui.strong("Cash reconciliation · opening + net receipts = closing");
            for (i, name) in NAMES.iter().enumerate() {
                ui.monospace(format!("{name}: {}m {:+}m = {}m", summary.opening[i], summary.closing[i] - summary.opening[i], summary.closing[i]));
            }
            ui.small("Opening cash is reconstructed immediately before this period's recorded transfers. Sector totals include only this country's finite external counterpart; the model does not claim a full global financial network.");
            let entries: Vec<_> = world.ledger.iter().rev().filter(|entry| &entry.country == country && entry.day >= sim_core::CampaignDay::new(first) && entry.day <= world.day
                && self.route.is_none_or(|(from, to)| index(entry.from) == from && index(entry.to) == to)).collect();
            let route = self.route.map_or_else(|| "All routes".into(), |(from, to)| format!("{} to {}", NAMES[from], NAMES[to]));
            ui.strong(format!("{route} · {} transactions · latest 80 shown", entries.len()));
            if entries.is_empty() { ui.label("No recorded transfers on this route in the selected period."); }
            for entry in entries.into_iter().take(80) {
                ui.label(format!("{} · {} to {} · {}m · {}", date(entry.day.units()), NAMES[index(entry.from)], NAMES[index(entry.to)], entry.amount.0, entry.reason));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::{Action, Money};
    #[test]
    fn actual_cash_flows_reconstruct_period_opening_after_earlier_capital_and_financing() {
        for fixture in [
            include_str!("../../../data/synthetic-test.json"),
            include_str!("../../../data/synthetic-dated-test.json"),
        ] {
            let content = Content::from_json(fixture).unwrap();
            let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
            let start = world.day.units();
            let initial = ACCOUNTS.map(|account| world.player_country().accounts.balance(account));
            world
                .issue(
                    &content,
                    Action::Borrow {
                        amount: Money(1000),
                    },
                )
                .unwrap();
            world
                .issue(
                    &content,
                    Action::Enact {
                        policy: "energy".into(),
                        value: 1,
                    },
                )
                .unwrap();
            let after_orders =
                ACCOUNTS.map(|account| world.player_country().accounts.balance(account));
            let summary = summarize(&world, &world.player, start).unwrap();
            assert_eq!(summary.opening, initial);
            assert_eq!(
                summary.transfers[index(Account::External)][index(Account::Treasury)],
                1000
            );
            world.advance(&content, sim_core::DaySpan::new(30)).unwrap();
            let summary = summarize(&world, &world.player, start + 1).unwrap();
            assert_eq!(summary.opening, after_orders);
            assert_eq!(
                summary.opening.iter().sum::<i64>(),
                summary.closing.iter().sum::<i64>()
            );
            assert!(summary.transfers[index(Account::Households)][index(Account::Treasury)] > 0);
            assert!(summary.transfers[index(Account::Treasury)][index(Account::Households)] > 0);
            for i in 0..4 {
                let paid: i64 = summary.transfers[i].iter().sum();
                let received: i64 = summary.transfers.iter().map(|row| row[i]).sum();
                assert_eq!(summary.opening[i] + received - paid, summary.closing[i]);
            }
            let encoded = sim_core::persistence::encode(&world, &content).unwrap();
            let loaded = sim_core::persistence::decode(&encoded, &content).unwrap();
            assert_eq!(
                summary.transfers,
                summarize(&loaded, &world.player, start + 1)
                    .unwrap()
                    .transfers
            );
            assert!(summarize(&world, &CountryId("missing".into()), start + 1).is_err());
        }
    }
}
