# Geopolitical Simulator

An initial playable Rust geopolitical strategy game for macOS, Linux and Windows, starting on **1 January 2000**. Govern one of 15 countries through its institutions, fund public services, expand energy and food supply, manage debt and survive political transitions. Other governments act through the same command rules.

This is the first connected game slice of the research roadmap: **341 proposed requirements across 42 ordered systems**. It does not implement the full catalogue. See the [initial-game scope](docs/initial-game.md) for implemented loops, simplifications and remaining work.

## Play

Install [Rust through rustup](https://rustup.rs/) and a native linker: Xcode Command Line Tools on macOS, a C compiler/linker on Linux, or Visual Studio Build Tools with Desktop development with C++ on Windows. The committed toolchain selects Rust 1.99.1, rustfmt and Clippy. Linux desktop play needs an X11/Wayland session, libxkbcommon and a Vulkan or OpenGL-capable graphics driver; the simulation and terminal game need no graphics stack.

Launch the native desktop game:

```sh
cargo run -p desktop --release --locked
```

Choose a country, select **Sandbox** or the authored **Fiscal and supply crisis**, and click **Begin**. Advance a day, inspect households and accounts, preview a policy, secure consent, fund it and watch delivery. Space pauses/plays; F5 saves, F9 loads and N advances to the next important event. F1 opens rebindable controls and interface scaling; Escape pauses and returns to the map. The interface includes a situation brief, charts, pinned country comparisons, a guide and a five-year objective. Long advances run off the UI thread, can stop at day boundaries and respect important-event pauses. Elections and leadership changes continue country play.

Play in a terminal:

```sh
cargo run -p headless --release --locked -- --interactive --country GBR --seed 2000
```

Type `help`. For example:

```text
advance 1
households
budget
preview energy 1
enact energy 1
advance 100
status
ledger
save campaign.json
replay
quit
```

Run a reproducible scenario without interaction (also the default `cargo run` behavior):

```sh
cargo run --release --locked -- --country CHN --seed 2000 --days 1827 --verify
```

Add `--scenario crisis` to the headless command for the authored fiscal/supply-crisis campaign.

The desktop map supports country-shape selection, zoom at the pointer, focus-on-country, keyboard panning and dated approval/needs/debt overlays. Islands and interior holes are preserved in the supplied reference shapes; markers and search cover countries without shapes. Charts use actual release dates and provide hover, keyboard inspection and text values. Foreign reports expose published aggregates; private household accounts, treasury and AI traces stay within the controlled government.

Both clients load the same bundled content. Supply `--content path/to/pack.json` to either executable to use a validated local JSON pack. Unknown schema, broken references, invalid ranges and incompatible saves produce explicit errors.

## World and game data

The [initial content pack](data/initial-2000-observed.json) seeds 15 countries, 34 parties/governing bodies, eight institutional configurations and seven policy programs. The roster includes the UK, US, France, Germany, Russia, China, India, Japan, Brazil, South Africa, Saudi Arabia, Iran, Cuba, North Korea and Myanmar.

**The default starter has 29 pinned World Bank-mirror observations: population for all 15 countries and GDP for 14. North Korean GDP and other economic values, household profiles, seats, ideology profiles and political rules remain explicitly authored.** The source inspector distinguishes the two; the primary data API has not been independently reconciled. Historical identity names form a limited starter curation. Royal, military and religious governing bodies are explicitly scenario abstractions. All future decisions and successor cabinets are simulated. See [data provenance and coverage](data/README.md). The map combines Natural Earth public-domain land with reference country outlines for the 15 starters. Their source revision and transformations are pinned; they are not a certified 2000 sovereignty/control layer. Administrative regions remain absent.

Money is expressed in millions of constant-2000 USD-equivalent game credits. Displayed local currency codes are identity metadata; there is no currency exchange model yet. Stocks and realized daily flows are shown separately. Policy forecasts state their assumptions.

An entirely fictional [three-government fixture](data/synthetic-test.json) offers identical opening economies under parliamentary, party-council and royal authority. It has no historical-data dependencies. Launch it with `cargo run -p headless --release --locked -- --content data/synthetic-test.json --country AUR --interactive`, or pass the same `--content` option to the desktop. Rebuild it offline with `python3 scripts/build_synthetic_world.py`; `--check` verifies reproducibility. Its map markers are arbitrary test locations, not territorial claims.

The [dated fictional fixture](data/synthetic-dated-test.json) starts on 2001-07-15 and demonstrates people, separate office roles, collective leadership, explicit vacancy, unknown incumbency and primary-currency records. Use its path with the same `--content` command. Records initialize the selected bookmark; future reference handovers do not force campaign outcomes. Custom starts must allow a five-calendar-year review by 2100-01-01. The default historical starter remains 2000-01-01; historical office-term coverage has not been completed.

The default [15-country starter](data/initial-2000-fiscal-plans.json) uses content/rules 12 with 29 population/GDP observations, markets, borrowing, constitutional transitions, policy history, ministries, electoral mandates, territorial government, delivery undertakings and reviewed fiscal plans. **Treasury & accounts** combines income-tax, service and welfare changes into annual or supplementary proposals, with paid administration, protected implementation reservations, institutional review and actual dated programmes. Recurring purchases still need daily cash; these plans are not hard appropriation ceilings or historical budget laws. January transactions enter the new fiscal year, and the initial opening snapshot has no realized daily flow. Each country retains its two authored budget jurisdictions and three undertaking templates, including the full-year linked-project deadline. These are budget and commitment abstractions, not historical administrative units or electoral promises. The [undertaking roster](data/initial-2000-undertakings.json) retains rules 11; the [territorial roster](data/initial-2000-territorial.json) retains rules 10. The [expanded-observation roster](data/initial-2000-observed.json) and [earlier electoral roster](data/initial-2000-elections.json) retain rules 8 with 29 and six facts respectively. Cabinet, commitment, constitutional, financing, market and classic packs retain rules 7, 6, 5, 4, 3 and 2. All eleven share the political/policy roster, and saves resolve their exact original pack. Procedures and calibration have separate authored provenance.

Food and energy prices respond after daily settlement; food reprices part of household baskets and both indices affect import bills. Treasury separates physical need, affordable demand, delivery and reconciled stocks. Borrowing requests need the country's configured legal route and decision delay; pending funds are unavailable until approval rechecks backing, government continuity, credit headroom and the finite creditor's funds. Treasury shows cancellation, debt records and an explicit contested emergency route with political costs/cooldown. Loans remain external fixed-4% perpetual debt in reporting credits, with oldest-first voluntary redemption. This does not complete sectors, endogenous wages, inflation, banks, FX, defaults or constitutional institutions.

The optional [market fixture](data/synthetic-market-test.json) isolates rules 3. The [borrowing-authority fixture](data/synthetic-finance-test.json) isolates rules 4 across three fictional governments: Aurora needs 14-day legislative consent, Boreal 21-day party consent and Coral an immediate authorized royal decree. Launch either client with `--content` and the fixture path (and `--country AUR` for the terminal). The terminal exposes `finance`, `preview-loan MILLIONS`, `cancel-loan ID` and `emergency-borrow MILLIONS`. Fictional fixtures require their exact explicit content pack when reopening saves.

The optional [constitutional-transition fixture](data/synthetic-constitution-test.json) uses content/rules 5 and snapshot format 6. Launch with `--content data/synthetic-constitution-test.json` to isolate the same transition model used by the default starter. **Institutions & parties** offers lawful proposals, explicit contested attempts, withdrawal and enacted history. Changes update actual governing powers, policy/borrowing consent, elections/reviews and cabinet sponsorship; existing debt and funded programs continue. Both directions are playable across the eight configured forms. Upfront administration, protected reservations, rechecked backing, political costs and cooldown constrain the process. The terminal adds `constitutions`, `preview-constitution FORM lawful|contested`, `reconstitute FORM lawful|contested` and `withdraw-constitution ID`; the listing prints the form identifiers. These are authored political abstractions, not historical constitutional procedures, a complete military/faction model or full electoral law. Rules-2/3/4/5 campaigns retain their original packs, behavior and checksums.

The optional [policy-lifecycle fixture](data/synthetic-commitment-test.json) uses rules/content 6 and snapshot format 7; launch with `--content data/synthetic-commitment-test.json`. **Policies & implementation** adds paid amendments before consent, withdrawal before delivery, a persistent programme history and observed outcome reviews. Amendments preview household income and annual budget changes against currently operating levels, then restart consent/delivery under current authority; withdrawal releases only reserved cash, and payments already made remain spent. New operating tax/service/welfare levels replace the previous level; capacity additions remain physical assets. Reviews separate direct setting changes from observed settlement, other deliveries and external shocks; they do not estimate isolated causal effects. Terminal commands: `commitments`, `preview-amend ID LEVEL`, `amend ID LEVEL`, `preview-withdraw ID`, `withdraw ID`. The default 15-country starter also uses this lifecycle model, with separately attributed authored procedures. Staffing, materials, regional exemptions, promises/contracts and full expiry/repeal rules remain unfinished.

The optional [ministerial fixture](data/synthetic-cabinet-test.json) uses rules/content 7 and snapshot format 8 (`--content data/synthetic-cabinet-test.json`; terminal country `AUR`). **Institutions & parties** adds finance, supply and cabinet-administration portfolios with generated nominees. Skill shortens preparation for new proposals; political acceptability determines backing costs. Appointments pay administration, require current authority and recheck sponsorship at consent. Withdrawal/refusal keeps fees and cooldown; dismissal costs cash and restores civil-service preparation. Successor cabinets end terms and cancel pending appointments. Existing construction dates and productive capacity remain unchanged. The terminal adds `cabinet`, `preview-appointment OFFICE PERSON`, `appoint OFFICE PERSON`, `preview-dismissal OFFICE`, `dismiss OFFICE` and `cancel-appointment ID`. Names and profiles are authored, not historical ministerial data. The default starter uses the same model with 45 authored functional portfolios and 90 generated nominees; these are not historical ministerial rosters or vacancy claims. Separate head-of-state powers, courts, independent bodies and staffing networks remain unfinished.

The optional [electoral-authority fixture](data/synthetic-election-test.json) uses rules/content 8 and snapshot format 9 (`--content data/synthetic-election-test.json`; terminal country `AUR`). **Institutions & parties** adds a separate authority for early-election requests, fixed announced dates and recorded modeled polls. Current form, ordinary backing, a minimum interval, administration cash and review/campaign time constrain requests. The government cannot withdraw an announced poll or postpone an earlier mandatory confidence election. An explicit, costed constitutional change can supersede the mandate. Aurora can request an early poll; Boreal starts presidential in this fixture to demonstrate a government without that configured power; Coral starts royal. Results still use the simplified 100-seat model. No turnout, districts or integrity certification is modeled, and no historical past polls are invented. Terminal commands: `elections`, `preview-election`, `request-election`, `withdraw-election ID`. The default starter uses the same procedures across 15 countries with separate authored provenance. Early-poll requests are configured for parliamentary and semi-presidential forms; this is a gameplay rule, not a claim about each country’s historical law. Broader independent institutions and national electoral law remain unfinished.

The optional [subnational fixture](data/synthetic-subnational-test.json) uses rules/content 9 and snapshot format 10 (`--content data/synthetic-subnational-test.json`; terminal country `AUR`). **Institutions & parties** adds fictional provincial/municipal budget jurisdictions with fixed shares of collected income tax. Local funds and pending grant escrow remain inside consolidated public cash but outside national spending availability. Autonomous service levels cannot be ordered nationally; delegated orders use current consent. Costed grants pre-fund additional public-service purchases, subject to national backing, sponsor continuity, local absorption rules and calendar limits. Withdrawal/refusal releases only escrow; accepted funding survives national succession and cannot be reclaimed by ordinary national order. The terminal adds `regions`, `preview-region-grant REGION BOOST DAYS`, `region-grant REGION BOOST DAYS`, `preview-region-services REGION LEVEL`, `region-services REGION LEVEL` and `cancel-region ID`. These are funded household payments, not physical capacity or service-quality estimates. The default rules-12 roster includes the same local-funding mechanics with separately attributed authored budget jurisdictions. Historical regional geography, local tax rates, regional households, charter changes and contested seizure remain unfinished.

While paused, a brief below the time controls shows today’s important events for your government, including the reason an important-event advance stopped. **Open chronicle** returns to your government’s newest records; **Dismiss brief** hides the current notices until a new important event. Foreign private events stay outside the brief, and these controls issue no game orders.

The **How to play** screen includes a first-mandate walkthrough and **Search game reference**. Search mechanics, policies and countries from the loaded bookmark, then open the relevant screen. The command bar shows paused/running/advancing/stopping state on every page. Automatic progression selects days per step; single-step buttons pause automatic progression. Escape pauses and returns to the map while retaining the selected country and camera, or closes save recovery.

In **Treasury & accounts**, expand **Trace realized cash transfers** to compare today's transfers or the last 30 days. Rows pay columns; selecting an amount filters transactions by payer and recipient. Opening cash plus net receipts reconciles to closing cash for all four sectors. This uses recorded payments, with reservations and arrears kept separate.

The household screen reports dated income pressure, actual food delivery and a group-mean income Gini, alongside income/basket coverage and cash/basket buffers. These use modeled disjoint groups: they do not estimate individual poverty or inequality within groups. Opening-state indicators remain unavailable until the first settlement. The terminal's `households` command reports the same aggregate indicators.

Treasury's **Funding outlook** shows available cash and how many whole days it covers the current deficit. Expand its interactive chart to compare no new borrowing with conditional approval of the selected or pending ordinary loan, including extra interest. Negative forecast balances indicate funding gaps, not an overdraft. This unchanged-policy scenario excludes shocks, price changes, arrears repayments and pending program delivery. The terminal provides `outlook [DAYS]`. Blocked borrowing and policy previews link to a governing-support agreement; funding failures link to Treasury. Opening a preview issues no order.

Use **New campaign** in the command bar to choose a country, seed and scenario without restarting. Default launches can choose current, earlier electoral, earlier cabinet, earlier commitment, earlier constitutional, earlier financing, earlier market or classic packs; an explicit `--content` stays pinned. **Save and start** validates the new setup, saves the active game and retains a unique `.before-new-campaign-N` archive before replacing it. **Start without saving** explicitly discards unsaved progress. Cancel/Escape returns to the paused game; failed saves or invalid setup leave it active.

## Save, recover and replay

The optional `data/synthetic-territorial-test.json` adds rules 10 and snapshot format 11. **Institutions & parties → Change territorial settlement** offers lawful and contested reforms. Lawful loss of autonomy or tax rights needs an explicitly authored household-consent proxy and reserved local compensation. Administration, backing checks, review, withdrawal and cooldown apply; accepted grants and existing local cash survive. A separate contested requisition can take existing base funds only, with effective-power backing, fees and political costs; grant escrow and accepted grant balances remain protected. Terminal commands are `territory`, `preview-territory FORM lawful|contested`, `reform-territory FORM lawful|contested`, `withdraw-territory ID`, `preview-requisition REGION MILLIONS` and `requisition REGION MILLIONS`. Forms are `federation`, `decentralized_unitary` and `delegated_unitary`. These procedures are fictional model assumptions, not historical territorial law or regional elections. The default rules-12 roster includes these procedures with separately attributed authored assumptions.

The optional [delivery-undertaking fixture](data/synthetic-undertakings-test.json) uses rules 11 and snapshot format 12. **Policies & implementation → Delivery undertakings** adds public delivery pledges, governing support arrangements and fixed deadlines for existing policy projects. Declarations pay administration and undergo review; order/fund the policy separately. Actual policy delivery records determine fulfilment. Accepted targets stay fixed through amendments, and missing or withdrawing them has explicit political/credibility costs. These arrangements do not form a cabinet coalition or create physical resources. Terminal commands: `undertaking-templates`, `undertakings`, `preview-undertaking TEMPLATE [PROJECT]`, `declare-undertaking TEMPLATE [PROJECT]` and `withdraw-undertaking ID`. Project IDs are the existing policy commitment IDs. The default rules-12 roster includes the same procedures with separate authored provenance and a longer linked-project window; full electoral promises, coalition formation, commercial contracts and treaties remain unfinished.

The optional [fiscal-plan fixture](data/synthetic-fiscal-plans-test.json) uses rules/content 12 and snapshot format 13 (`--content data/synthetic-fiscal-plans-test.json`, country `AUR`). **Treasury & accounts → Annual & supplementary budget** combines income-tax, service and welfare changes in a reviewed package. Submission pays administration and reserves every programme's implementation cost. Review rechecks backing and sponsorship, then funds all included programmes or refuses the whole package. Annual changes cannot operate before January; each policy retains its actual delivery date. Pending withdrawal and succession release unspent implementation reservations while retaining administration/intervals; enacted projects keep their normal funded withdrawal rules. Forecasts are conditional, recurring spending still needs daily cash, and these plans are not hard appropriation ceilings. Commands: `fiscal-plans`, `preview-budget YEAR TAX SERVICES WELFARE`, `propose-budget YEAR TAX SERVICES WELFARE`, `withdraw-budget ID`. The default rules-12 roster includes the same procedures with separately attributed authored provenance; other tax bases, maturities/default and full appropriation law remain unfinished.

The desktop writes manual saves to the path in its sidebar, initially `saves/campaign-v13.json` for the current fiscal roster, `saves/campaign-v12.json` for the preserved undertaking roster, `saves/campaign-v11.json` for the preserved territorial roster, `saves/campaign-observed-v9.json` for the preserved expanded-observation roster, `saves/campaign-v9.json` for the earlier electoral roster, `saves/campaign-v8.json` for earlier cabinet rules 7, `saves/campaign-v7.json` for earlier commitment rules 6, `saves/campaign-v6.json` for earlier constitutional rules 5, `saves/campaign-v5.json` for earlier financing rules 4, `saves/campaign-v4.json` for earlier rules 3, or `saves/campaign-v3.json` for classic rules 2. Every crossed 30-day reporting boundary writes a country, scenario and content-specific filename in `saves`, ending in the campaign format (`v3.json` through `v13.json`, depending on the selected pack). A completed save replaces the primary file using a temporary file and retains the previous snapshot as `.json.bak`. Use **Inspect recovery snapshots** to review the primary, pending write, backup or interrupted backup/recovery stage before restoring. Originals are archived before recovery; a damaged primary never replaces the last validated backup. Interrupted writes block ordinary replacement until explicitly resolved. Saves include content/rules hashes, commands, projects and deterministic shock state; incompatible or damaged files fail without replacing the active campaign.

The terminal uses `until YYYY-MM-DD`, `households`, `budget`, `data`, `save PATH`, `load PATH` and `replay`. Paths in terminal commands cannot contain spaces. `--save FILE` and `--load FILE` work in the runner. Rules-2 campaigns write snapshot format 3; relative-price rules-3 campaigns write format 4 and require content schema 3. Current content/rules 12 requires snapshot format 13; preserved undertaking rules 11 requires format 12; preserved territorial rules 10 requires format 11; preserved observed/electoral rules 8 requires format 9; earlier cabinet content/rules 7 requires format 8; earlier commitment content/rules 6 requires format 7; earlier constitutional content/rules 5 requires format 6; earlier financing content/rules 4 requires format 5; the earlier market starter retains content/rules 3 and the classic starter retains content/rules 2. Without --content, loading resolves the exact saved hash against the current, earlier undertaking, earlier territorial, earlier observed, earlier electoral, earlier cabinet, earlier commitment, earlier constitutional, earlier financing, earlier market and classic bundled packs, retaining the saved rules. Explicit --content selections stay pinned. Unknown packs require their original file rather than a substitute. Compatible formats 2 and 3 load without changing their files or ordinary campaign checksums. Rules-3 through rules-12 campaigns cannot be relabeled as older formats. Saving over a validated format-2 snapshot first archives its exact bytes to a unique `.format-v2-N` file; subsequent saves retain that archive. Recovery also preserves originals, including a selected backup before format conversion, before writing the campaign's supported format. Version-1 files remain unsupported and are rejected without modification. Replay re-executes player and explicitly marked developer commands alongside deterministic AI and approved-budget programme dispatch, checks commands in sequence and verifies the final state hash. Diagnostics identify the first differing command or final-state field; a final snapshot alone cannot establish the earliest divergent tick. Hashes detect accidental corruption; they are not cryptographic authentication.

Developer inspection is explicitly enabled with `--developer` on either executable. The desktop adds a paused developer inspector for the selected country; the terminal adds `dev-fields`, `dev-inspect COUNTRY` and `dev COUNTRY FIELD VALUE`. Inspection exposes live world truth, including private foreign accounts, and belongs to the trusted host. Bounded direct field edits bypass policy costs and delays, consume ordinary command IDs, and permanently mark the journal/save as developer-modified. Normal player and AI command entry points reject these edits. Marked campaigns remain marked when reopened without the flag and cannot receive ordinary mandate certification. Merely opening the inspector does not modify a campaign. This is a limited scenario tool, not the full content editor.

## Validation

Python 3.11+ is development tooling only. Run the same checks as CI:

```sh
python3 scripts/validate.py
```

This checks document/register consistency, local Markdown links, whitespace, Rust formatting, Clippy with warnings denied, tests, Rust docs with warnings denied, workspace release builds (including desktop) and the headless replay runner and a worked recovery campaign that achieves the five-year mandate in all 15 seeded countries through ordinary validated commands. CI is configured on macOS, Linux and Windows; actual platform validation requires those jobs to run. Desktop visual/interaction checks require a display and are separate from the headless checks.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
```

A reproducible successful crisis strategy can also be inspected and run:

```sh
cargo run -p headless --example recovery --release --locked
cargo run -p headless --example recovery --release --locked -- --contested-transition
cargo run -p headless --example recovery --release --locked -- --content data/synthetic-test.json
cargo run -p headless --example recovery --release --locked -- --content data/synthetic-dated-test.json
cargo run -p headless --example recovery --release --locked -- --content data/synthetic-market-test.json
cargo run -p headless --example recovery --release --locked -- --content data/synthetic-finance-test.json
cargo run -p headless --example recovery --release --locked -- --content data/synthetic-commitment-test.json
cargo run -p headless --example recovery --release --locked -- --content data/synthetic-constitution-test.json
cargo run -p headless --example recovery --release --locked -- --content data/synthetic-constitution-test.json --contested-transition
```

This example borrows for runway, proposes tax/spending reforms, builds supply and administrative capacity, maintains support and repays debt when affordable. With `--contested-transition`, it first negotiates backing, secures approved loans, makes a costed contested change of governing form and recovers afterward. Minority cabinets negotiate before borrowing and wait for actual disbursement before funding more talks. It uses the same validators, costs and delays as a player. The seed-17 acceptance run establishes that every initial country has a recovery path; it is not a universal optimal AI or a guarantee of balance for every seed. See [development progress](docs/development-progress.md) for the full remaining scope.

## Repository and research

- `crates/sim-core/`: content schema, pure daily simulation, account settlement, policies, politics, AI, saves and replay.
- `crates/headless/`: scenario runner and terminal game.
- `crates/desktop/`: native egui/eframe client with wgpu rendering.
- `data/`: versioned original game content and land geometry with provenance.
- [Implementation order](docs/implementation-order.md) and [requirement register](docs/implementation-register.csv): dependency-ordered roadmap with stable IDs.
- [Technical overview](docs/technical-research.md) and [world-data specification](docs/world-data-specification.md): intended architecture and content standards.
- [Eversim research](docs/eversim-research.md), [comparator research](docs/comparators-research.md) and [source register](docs/source-register.csv): references and coverage limitations.

Research snapshot: 2 November 2026. Advertised capabilities and planned features are distinguished from verified behavior. The inventories identify incomplete announcement and in-game coverage. See [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md) for project constraints.
