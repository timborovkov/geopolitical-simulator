use eframe::egui;
use sim_core::*;
use std::fmt::Display;

pub fn show(ui: &mut egui::Ui, content: &Content, world: &World, country: &CountryId) {
    if country != &world.player {
        return;
    }
    let (Some(definition), Some(markets)) = (&content.markets, &world.countries[country].markets)
    else {
        return;
    };
    egui::CollapsingHeader::new("Food and energy markets · prices, demand and stocks").show(ui, |ui| {
        ui.label("Relative price indices use the opening price as 1.00. Food reprices the configured part of essential baskets; food/energy prices also change import bills. These are authored model prices, not historical currency prices or a complete inflation index.");
        ui.label(format!("Food price affects {:.0}% of the essential basket. Prices respond after settlement and apply on the following day.", f64::from(definition.food_budget_share_bps.units()) / 100.0));
        show_quote(ui, &markets.energy, definition.good(GoodKind::Energy));
        show_quote(ui, &markets.food, definition.good(GoodKind::Food));
        if let Some(source) = content.sources.iter().find(|source| source.id == definition.source) {
            ui.collapsing("Market assumptions and source", |ui| {
                ui.label(format!("{} · revision {} · {}", source.id, definition.source_version, source.quality));
                ui.label(&source.description);
            });
        }
    });
}
fn show_quote<Q: Copy + Display>(ui: &mut egui::Ui, quote: &MarketQuote<Q>, rule: &GoodDefinition) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.strong(format!("{} · {}", rule.name, rule.unit));
        if let Some(last) = &quote.last {
            ui.label(format!("{} settlement price {:.4} · next settlement price {:.4}", date(last.day.units()), f64::from(last.price_bps.units()) / 10000.0, f64::from(quote.next_price_bps.units()) / 10000.0));
            let demand_label = if rule.kind == GoodKind::Food { "affordable food demand" } else { "requested industrial use" };
            ui.label(format!("Physical need {} · {demand_label} {} · supply {} · delivered {}", last.physical_need, last.requested_use, last.supply, last.delivered));
            if let Some(released)=last.reserve_release {
                ui.label(format!("Opening working stock {} + production {} + civilian imports {} + national reserve release {} = supply {}",last.opening_stock,last.production,last.imports,released,last.supply));
            } else {
            ui.label(format!("Opening stock {} + production {} + imports {} = supply {}", last.opening_stock, last.production, last.imports, last.supply));
            }
            ui.label(format!("Supply {} - delivered {} - loss {} = closing stock {}", last.supply, last.delivered, last.loss, last.closing_stock));
        } else {
            ui.label(format!("Opening reference price {:.4}. Awaiting first market settlement.", f64::from(quote.next_price_bps.units()) / 10000.0));
        }
        ui.small(format!("Price bounds {:.2}–{:.2}; maximum relative daily change {:.2}%. Import cap {:.0}% of physical need; storage ceiling {} need-days; daily remaining-stock loss {:.2}%.", f64::from(rule.minimum_price_bps.units()) / 10000.0, f64::from(rule.maximum_price_bps.units()) / 10000.0, f64::from(rule.adjustment_bps.units()) / 100.0, f64::from(rule.import_limit_bps.units()) / 100.0, rule.storage_days, f64::from(rule.loss_bps.units()) / 100.0));
    });
}
