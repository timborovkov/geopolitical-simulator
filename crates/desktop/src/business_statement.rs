//! Government-only business cash inspection. Every number comes from retained settlements.
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use sim_core::*;

#[derive(Clone)]
struct Inspection {
    days: u32,
    cursor: Option<CampaignDay>,
    hover: Option<Pos2>,
    only_day: bool,
}
impl Default for Inspection {
    fn default() -> Self {
        Self {
            days: 7,
            cursor: None,
            hover: None,
            only_day: false,
        }
    }
}
fn total(values: &[Money; 3]) -> i128 {
    values.iter().map(|v| i128::from(v.0)).sum()
}
fn counterpart(account: Account) -> &'static str {
    match account {
        Account::Households => "Households",
        Account::Treasury => "Government",
        Account::External => "External counterpart",
        Account::Firms => "Businesses",
    }
}

pub fn show(ui: &mut egui::Ui, content: &Content, world: &World, country: &CountryId) {
    egui::CollapsingHeader::new("Business receipts, wages & cash").show(ui, |ui| {
        body(ui, content, world, country);
    });
}
fn body(ui: &mut egui::Ui, content: &Content, world: &World, country: &CountryId) {
    let key = egui::Id::new(("business_statement", content.hash(), country.0.as_str()));
    let mut inspection = ui.data_mut(|d| d.get_temp::<Inspection>(key).unwrap_or_default());
    let report = match world.business_statement(
        content,
        Viewer::Government(world.player.clone()),
        country,
        DaySpan::new(inspection.days),
    ) {
        Ok(report) => report,
        Err(reason) => {
            ui.label(reason);
            return;
        }
    };
    let Some(statement) = report.owned else {
        ui.label("Business accounts are not published for this government.");
        return;
    };
    show_ownership(ui, content, world, country);
    let previous = inspection.days;
    ui.horizontal_wrapped(|ui| {
        for (days, label) in [(1, "Current day"), (7, "Last 7 days"), (30, "Last 30 days")] {
            ui.selectable_value(&mut inspection.days, days, label);
        }
    });
    // Apply a newly selected period on the following frame; the labelled report
    // and all chart/table rows always belong to the same completed query.
    if previous != inspection.days {
        ui.ctx().request_repaint();
    }
    ui.label(format!(
        "{} through {} inclusive · millions of constant-2000 USD-equivalent game credits",
        date(report.first_day),
        date(report.last_day)
    ));
    let receipts = total(&statement.receipts);
    let payments = total(&statement.payments);
    ui.strong(format!(
        "Business cash: {}m {:+}m = {}m",
        statement.opening_cash.0,
        receipts - payments,
        statement.closing_cash.0
    ));
    ui.label(format!(
        "Realized receipts {receipts}m · realized payments {payments}m"
    ));
    ui.small("Opening cash is reconstructed from current cash and this period's recorded transfers; daily balances use the same reconciliation.");
    ui.small("Cash movement includes government capital and administration payments. It is not profit or GDP: inventory valuation, depreciation and individual firms are not represented here. Reservations and unpaid obligations are separate.");
    egui::Grid::new(key.with("counterparties"))
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Counterparty");
            ui.strong("Receipts");
            ui.strong("Payments");
            ui.end_row();
            for (index, account) in BUSINESS_COUNTERPARTIES.iter().enumerate() {
                ui.label(counterpart(*account));
                ui.label(format!("{}m", statement.receipts[index].0));
                ui.label(format!("{}m", statement.payments[index].0));
                ui.end_row();
            }
        });
    ui.small("Payments to households are private income in current rules; receipts from households are consumption. External payments include civilian food/energy imports. Public industrial purchases settle directly with the external counterpart and do not enter business import costs; their administration fees do enter business receipts.");
    chart(ui, &statement.days, &mut inspection);
    let selected = inspection
        .cursor
        .unwrap_or(report.last_day)
        .max(report.first_day)
        .min(report.last_day);
    inspection.cursor = Some(selected);
    if let Some(day) = statement.days.iter().find(|d| d.day == selected) {
        ui.label(format!(
            "Inspected {} · receipts {}m · payments {}m · opening cash {}m · closing cash {}m",
            date(day.day),
            total(&day.receipts),
            total(&day.payments),
            day.opening_cash.0,
            day.closing_cash.0
        ));
    }
    ui.checkbox(
        &mut inspection.only_day,
        "Only transactions on inspected day",
    );
    let entries: Vec<_> = statement
        .transactions
        .iter()
        .rev()
        .filter(|e| !inspection.only_day || e.day == selected)
        .collect();
    ui.label(format!(
        "{} recorded transactions · latest 80 shown",
        entries.len()
    ));
    for e in entries.into_iter().take(80) {
        ui.label(format!(
            "{} · {} to {} · {}m · {}",
            date(e.day),
            counterpart(e.from),
            counterpart(e.to),
            e.amount.0,
            e.reason
        ));
    }
    ui.small("The ledger retains recent transfers. Older cash flows are unavailable; no earlier zero flows or annual totals are inferred.");
    ui.data_mut(|d| d.insert_temp(key, inspection));
}

fn chart(ui: &mut egui::Ui, days: &[BusinessCashDay], inspection: &mut Inspection) {
    let Some(first) = days.first() else {
        return;
    };
    let last = days.last().expect("nonempty days").day;
    let span = (last.units() - first.day.units()).max(1);
    let ceiling = days
        .iter()
        .map(|d| total(&d.receipts).max(total(&d.payments)))
        .max()
        .unwrap_or(0)
        .max(1) as f64
        * 1.1;
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().max(160.0), 180.0),
        Sense::click(),
    );
    if response.clicked() {
        response.request_focus();
    }
    let plot = Rect::from_min_max(
        rect.min + Vec2::new(65.0, 12.0),
        rect.max - Vec2::new(12.0, 28.0),
    );
    let painter = ui.painter_at(rect);
    let position = |day: CampaignDay, value: i128| {
        Pos2::new(
            plot.left() + (day.units() - first.day.units()) as f32 / span as f32 * plot.width(),
            plot.bottom() - (value as f64 / ceiling) as f32 * plot.height(),
        )
    };
    let steps = (ceiling.floor() as u32).clamp(1, 3);
    for tick in 0..=steps {
        let value = ceiling * f64::from(tick) / f64::from(steps);
        let y = plot.bottom() - plot.height() * tick as f32 / steps as f32;
        painter.line_segment(
            [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
            Stroke::new(0.5_f32, Color32::from_gray(65)),
        );
        painter.text(
            Pos2::new(plot.left() - 6.0, y),
            egui::Align2::RIGHT_CENTER,
            format!("{value:.0}"),
            egui::FontId::monospace(10.0),
            Color32::LIGHT_GRAY,
        );
    }
    for (received, color) in [
        (true, Color32::from_rgb(104, 224, 186)),
        (false, Color32::from_rgb(245, 181, 105)),
    ] {
        let points: Vec<_> = days
            .iter()
            .map(|d| {
                position(
                    d.day,
                    total(if received { &d.receipts } else { &d.payments }),
                )
            })
            .collect();
        if points.len() > 1 {
            painter.add(egui::Shape::line(
                points.clone(),
                Stroke::new(1.8_f32, color),
            ));
        }
        for point in points {
            if received {
                painter.circle_filled(point, 3.0, color);
            } else {
                painter.rect_filled(Rect::from_center_size(point, Vec2::splat(5.0)), 0.0, color);
            }
        }
    }
    for (day, align) in [
        (first.day, egui::Align2::LEFT_TOP),
        (last, egui::Align2::RIGHT_TOP),
    ] {
        if day == first.day && day == last {
            continue;
        }
        painter.text(
            position(day, 0) + Vec2::new(0.0, 8.0),
            align,
            date(day),
            egui::FontId::monospace(10.0),
            Color32::LIGHT_GRAY,
        );
    }
    if response.hovered()
        && let Some(pointer) = response.hover_pos()
        && (inspection.hover != Some(pointer) || response.clicked())
    {
        inspection.cursor = Some(CampaignDay::new(
            first.day.units()
                + (((pointer.x - plot.left()) / plot.width()).clamp(0.0, 1.0) * span as f32).round()
                    as u32,
        ));
    }
    inspection.hover = response.hover_pos();
    if response.has_focus() {
        let delta = ui.input(|i| {
            i32::from(i.key_pressed(egui::Key::ArrowRight))
                - i32::from(i.key_pressed(egui::Key::ArrowLeft))
        });
        if delta != 0 {
            inspection.cursor = Some(CampaignDay::new(
                (i64::from(inspection.cursor.unwrap_or(last).units()) + i64::from(delta))
                    .clamp(i64::from(first.day.units()), i64::from(last.units()))
                    as u32,
            ));
        }
    }
    let selected = inspection.cursor.unwrap_or(last).max(first.day).min(last);
    let x = position(selected, 0).x;
    painter.line_segment(
        [Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())],
        Stroke::new(1.0_f32, Color32::WHITE),
    );
    ui.label("Receipts (circles) · Payments (squares) · daily settled flows. Point at a day, or click and use Left/Right.");
}

fn show_ownership(ui: &mut egui::Ui, content: &Content, world: &World, country: &CountryId) {
    let Some(definition) = &content.businesses else {
        return;
    };
    let Ok(report) =
        world.business_ownership_report(content, Viewer::Government(world.player.clone()), country)
    else {
        return;
    };
    let Some(business) = report.owned else {
        return;
    };
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.strong("Household-owned civilian businesses");
        ui.label(format!("{} · operating receipts {}m · business payments {}m · government receipts {}m", business.id.0, business.operating_receipts.0, business.payments.0, business.public_receipts.0));
        ui.label(format!("Undistributed operating cash {:+}m · owner distributions to date {}m",business.undistributed_cash().0,business.distributions.0));
        ui.small("Operating cash is household/external receipts less all payments, including distributions. Opening cash and every government receipt are excluded. This is not accrual profit. Public industrial assets remain public.");
        let elapsed = world.day.units()-content.start_day().expect("bookmark").units();
        if content.business_finance.is_some() {
            ui.label(format!("Autonomous review every 30 elapsed days · next {} · at most {:.0}% of prior closed-day retained modeled earnings",date(CampaignDay::new(world.day.units()+30-elapsed%30)),f64::from(definition.distribution_bps.units())/100.));
            ui.small("Cash also retains the trailing payment buffer plus all undelivered client capital. The buffer is a cash policy, not an insolvency test. Private investment and insolvency remain unfinished.");
        } else {
            ui.label(format!("Autonomous review every 30 elapsed days · next {} · share at most {:.0}% of positive earned cash, while retaining trailing 30-day payments",date(CampaignDay::new(world.day.units()+30-elapsed%30)), f64::from(definition.distribution_bps.units())/100.));
        }
        if let Some(decision) = &business.last_decision {
            ui.label(format!("Last decision {}: cash {}m · protected payment buffer {}m · earned cash {:+}m · paid {}m",date(decision.day),decision.opening_cash.0,decision.protected_cash.0,decision.undistributed_cash.0,decision.paid.0));
            if let Some(p) = &decision.earnings_protection {
                ui.label(format!("Earnings closed {} · retained result {:+}m · unfinished client advances {}m · available cash {}m", date(p.closed_day),p.retained_earnings.0,p.client_advances.0,p.available_cash.0));
            }
            ui.label(decision.reason.explanation());
        } else {ui.label("Awaiting the first autonomous business review.");}
        egui::ScrollArea::horizontal().id_salt("business_owners").show(ui, |ui| {
            egui::Grid::new("business_owners_table").striped(true).show(ui, |ui| {
                for label in ["Owning household group", "Ownership weight", "Last payment m"] {ui.strong(label);} ui.end_row();
                for (index,h) in content.country(country).expect("country").households.iter().enumerate() {
                    ui.label(&h.name); ui.label(h.savings_share.to_string());
                    ui.label(business.last_decision.as_ref().map_or_else(|| "Awaiting review".into(),|d|d.owners[index].amount.0.to_string())); ui.end_row();
                }
            });
        });
        ui.small("Fixed ownership uses the authored opening savings weights. Distributions stop if an owning cohort has no residents; ownership is not reassigned. Owner payments enter ordinary household income withholding. Government has no payout command.");
        ui.collapsing("Business ownership assumptions", |ui|{ui.label(format!("{} · {}",definition.source,definition.source_version));ui.label(if content.business_finance.is_some() { "Inherited ownership assumptions (the current payout policy below supersedes their operating-cash payout base)." } else { "Ownership and payout assumptions" }); ui.label(&definition.description); if let Some(finance) = &content.business_finance { ui.separator(); ui.label(format!("Current payout policy: {} · {}",finance.source,finance.source_version)); ui.label(&finance.description); }});
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn render(
        ctx: &egui::Context,
        content: &Content,
        world: &World,
        id: &CountryId,
        events: Vec<egui::Event>,
    ) -> (String, Option<Pos2>) {
        let output = ctx.run(
            egui::RawInput {
                focused: true,
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 1800.0))),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| body(ui, content, world, id));
            },
        );
        let mut text = String::new();
        let mut button = None;
        for shape in output.shapes {
            if let egui::Shape::Text(t) = shape.shape {
                text.push_str(t.galley.text());
                text.push('\n');
                if t.galley.text() == "Last 30 days" {
                    button = Some(t.pos + t.galley.size() / 2.0);
                }
            }
        }
        (text, button)
    }
    #[test]
    fn ownership_and_autonomous_decision_are_private_and_read_only_at_both_scales() {
        let content = Content::businesses_bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        world.advance(&content, DaySpan::new(30)).unwrap();
        let before = persistence::encode(&world, &content).unwrap();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut text = String::new();
            for _ in 0..3 {
                text = render(&ctx, &content, &world, &world.player, vec![]).0;
            }
            assert!(text.contains("Household-owned civilian businesses"));
            assert!(text.contains("Last decision 2000-01-31"));
            assert!(text.contains("Undistributed operating cash"));
            assert!(text.contains("This is not accrual profit"));
            let (foreign, _) = render(&ctx, &content, &world, &CountryId("USA".into()), vec![]);
            assert!(!foreign.contains("Undistributed operating cash"));
            assert!(!foreign.contains("Ownership weight"));
        }
        assert_eq!(persistence::encode(&world, &content).unwrap(), before);
    }
    #[test]
    fn finance_decision_explains_earnings_and_both_cash_reserves_at_both_scales() {
        let content = Content::business_finance_bundled().unwrap();
        let mut world = World::new(&content, CountryId("GBR".into()), 17).unwrap();
        world
            .issue(
                &content,
                Action::Enact {
                    policy: "welfare".into(),
                    value: 6,
                },
            )
            .unwrap();
        world.advance(&content, DaySpan::new(30)).unwrap();
        let before = persistence::encode(&world, &content).unwrap();
        for scale in [1., 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut text = String::new();
            for _ in 0..3 {
                text = render(&ctx, &content, &world, &world.player, vec![]).0;
            }
            assert!(text.contains("prior closed-day retained modeled earnings"));
            assert!(text.contains("Earnings closed 2000-01-30"));
            assert!(text.contains("unfinished client advances"));
            assert!(text.contains("available cash"));
            let (foreign, _) = render(&ctx, &content, &world, &CountryId("USA".into()), vec![]);
            assert!(!foreign.contains("unfinished client advances"));
        }
        assert_eq!(before, persistence::encode(&world, &content).unwrap());
    }
    #[test]
    fn actual_period_controls_and_foreign_inspection_preserve_campaign_bytes_at_both_scales() {
        let content =
            Content::from_json(include_str!("../../../data/synthetic-test.json")).unwrap();
        let mut world = World::new(&content, CountryId("AUR".into()), 17).unwrap();
        world.advance(&content, DaySpan::new(35)).unwrap();
        let before = persistence::encode(&world, &content).unwrap();
        for scale in [1.0, 1.4] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let mut button = None;
            for _ in 0..4 {
                let (text, b) = render(&ctx, &content, &world, &world.player, vec![]);
                assert!(text.contains("It is not profit or GDP"));
                button = b;
            }
            let pos = button.unwrap();
            render(
                &ctx,
                &content,
                &world,
                &world.player,
                vec![egui::Event::PointerMoved(pos)],
            );
            for pressed in [true, false] {
                render(
                    &ctx,
                    &content,
                    &world,
                    &world.player,
                    vec![egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: Default::default(),
                    }],
                );
            }
            let (text, _) = render(&ctx, &content, &world, &world.player, vec![]);
            assert!(text.contains(&format!(
                "{} through {} inclusive",
                date(CampaignDay::new(6)),
                date(world.day)
            )));
            let (foreign, _) = render(&ctx, &content, &world, &CountryId("COR".into()), vec![]);
            assert!(foreign.contains("not published"));
            assert!(
                !foreign.contains("Business cash:") && !foreign.contains("recorded transactions")
            );
            assert_eq!(before, persistence::encode(&world, &content).unwrap());
        }
    }
    #[test]
    fn chart_keyboard_moves_one_day_without_a_stationary_pointer_overriding_it() {
        let ctx = egui::Context::default();
        let mut inspection = Inspection::default();
        let days: Vec<_> = (10..=16)
            .map(|day| BusinessCashDay {
                day: CampaignDay::new(day),
                opening_cash: Money(100),
                receipts: [Money(10); 3],
                payments: [Money(11); 3],
                closing_cash: Money(97),
            })
            .collect();
        let input = egui::RawInput {
            focused: true,
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 500.0))),
            ..Default::default()
        };
        let draw = |ctx: &egui::Context, inspection: &mut Inspection| {
            egui::CentralPanel::default().show(ctx, |ui| chart(ui, &days, inspection));
        };
        let _ = ctx.run(input.clone(), |ctx| draw(ctx, &mut inspection));
        let pos = Pos2::new(420.0, 80.0);
        let mut click = input.clone();
        click.events = vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            },
        ];
        let _ = ctx.run(click, |ctx| draw(ctx, &mut inspection));
        let before = inspection.cursor.unwrap();
        let mut key = input.clone();
        key.events = vec![egui::Event::Key {
            key: egui::Key::ArrowRight,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }];
        let _ = ctx.run(key, |ctx| draw(ctx, &mut inspection));
        assert_eq!(inspection.cursor.unwrap().units(), before.units() + 1);
        let selected = inspection.cursor;
        let _ = ctx.run(input, |ctx| draw(ctx, &mut inspection));
        assert_eq!(inspection.cursor, selected);
    }
}
