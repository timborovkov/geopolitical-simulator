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

The [initial content pack](data/initial-2000.json) seeds 15 countries, 34 parties/governing bodies, eight institutional configurations and seven policy programs. The roster includes the UK, US, France, Germany, Russia, China, India, Japan, Brazil, South Africa, Saudi Arabia, Iran, Cuba, North Korea and Myanmar.

**Six reference-country population/GDP fields have pinned World Bank-mirror observations; other economic values, household profiles, seats, ideology profiles and political rules are authored gameplay approximations.** The source inspector distinguishes the two; the primary data API has not been independently reconciled. Historical identity names form a limited starter curation. Royal, military and religious governing bodies are explicitly scenario abstractions. All future decisions and successor cabinets are simulated. See [data provenance and coverage](data/README.md). The map combines Natural Earth public-domain land with reference country outlines for the 15 starters. Their source revision and transformations are pinned; they are not a certified 2000 sovereignty/control layer. Administrative regions remain absent.

Money is expressed in millions of constant-2000 USD-equivalent game credits. Displayed local currency codes are identity metadata; there is no currency exchange model yet. Stocks and realized daily flows are shown separately. Policy forecasts state their assumptions.

An entirely fictional [three-government fixture](data/synthetic-test.json) offers identical opening economies under parliamentary, party-council and royal authority. It has no historical-data dependencies. Launch it with `cargo run -p headless --release --locked -- --content data/synthetic-test.json --country AUR --interactive`, or pass the same `--content` option to the desktop. Rebuild it offline with `python3 scripts/build_synthetic_world.py`; `--check` verifies reproducibility. Its map markers are arbitrary test locations, not territorial claims.

The [dated fictional fixture](data/synthetic-dated-test.json) starts on 2001-07-15 and demonstrates people, separate office roles, collective leadership, explicit vacancy, unknown incumbency and primary-currency records. Use its path with the same `--content` command. Records initialize the selected bookmark; future reference handovers do not force campaign outcomes. Custom starts must allow a five-calendar-year review by 2100-01-01. The default historical starter remains 2000-01-01; historical office-term coverage has not been completed.

The default [15-country starter](data/initial-2000-markets.json) now uses relative-price rules 3. The [original pack](data/initial-2000.json) remains bundled for classic fixed-price campaigns. Both retain the same country, party, policy and reference-observation definitions. The optional [relative-price fixture](data/synthetic-market-test.json) uses content schema 3 and rules 3. Launch either client with `--content data/synthetic-market-test.json` (and `--country AUR` for the terminal). Food and energy prices respond after daily settlement; food reprices part of household baskets and both indices affect import bills. Treasury's market report separates physical need, affordable demand, delivery and reconciled stocks. Classic campaigns and earlier fictional packs retain rules 2 and fixed prices; their ordinary replay hashes are preserved. This authored fixture does not add full sector recipes, endogenous wages, inflation measurement or banking.

The **How to play** screen includes a first-mandate walkthrough and **Search game reference**. Search mechanics, policies and countries from the loaded bookmark, then open the relevant screen. The command bar shows paused/running/advancing/stopping state on every page. Automatic progression selects days per step; single-step buttons pause automatic progression. Escape pauses and returns to the map while retaining the selected country and camera, or closes save recovery.

In **Treasury & accounts**, expand **Trace realized cash transfers** to compare today's transfers or the last 30 days. Rows pay columns; selecting an amount filters transactions by payer and recipient. Opening cash plus net receipts reconciles to closing cash for all four sectors. This uses recorded payments, with reservations and arrears kept separate.

The household screen reports dated income pressure, actual food delivery and a group-mean income Gini, alongside income/basket coverage and cash/basket buffers. These use modeled disjoint groups: they do not estimate individual poverty or inequality within groups. Opening-state indicators remain unavailable until the first settlement. The terminal's `households` command reports the same aggregate indicators.

## Save, recover and replay

The desktop writes manual saves to the path in its sidebar, initially `saves/campaign-v3.json` for rules 2 or `saves/campaign-v4.json` for relative-price rules 3. Every crossed 30-day reporting boundary writes a country, scenario and content-specific filename in `saves`, ending in the campaign format (`v3.json` or `v4.json`). A completed save replaces the primary file using a temporary file and retains the previous snapshot as `.json.bak`. Use **Inspect recovery snapshots** to review the primary, pending write, backup or interrupted backup/recovery stage before restoring. Originals are archived before recovery; a damaged primary never replaces the last validated backup. Interrupted writes block ordinary replacement until explicitly resolved. Saves include content/rules hashes, commands, projects and deterministic shock state; incompatible or damaged files fail without replacing the active campaign.

The terminal uses `until YYYY-MM-DD`, `households`, `budget`, `data`, `save PATH`, `load PATH` and `replay`. Paths in terminal commands cannot contain spaces. `--save FILE` and `--load FILE` work in the runner. Rules-2 campaigns write snapshot format 3; relative-price rules-3 campaigns write format 4 and require content schema 3. The default starter uses content/rules 3; the classic starter retains content/rules 2. Without --content, loading resolves the exact saved hash against the current and classic bundled packs, retaining the saved rules. Explicit --content selections stay pinned. Unknown packs require their original file rather than a substitute. Compatible formats 2 and 3 load without changing their files or ordinary campaign checksums. A rules-3 campaign cannot be relabeled as an older format. Saving over a validated format-2 snapshot first archives its exact bytes to a unique `.format-v2-N` file; subsequent saves retain that archive. Recovery also preserves originals, including a selected backup before format conversion, before writing the campaign's supported format. Version-1 files remain unsupported and are rejected without modification. Replay re-executes player and explicitly marked developer commands alongside deterministic AI, checks commands in sequence and verifies the final state hash. Diagnostics identify the first differing command or final-state field; a final snapshot alone cannot establish the earliest divergent tick. Hashes detect accidental corruption; they are not cryptographic authentication.

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
cargo run -p headless --example recovery --release --locked -- --content data/synthetic-test.json
cargo run -p headless --example recovery --release --locked -- --content data/synthetic-dated-test.json
cargo run -p headless --example recovery --release --locked -- --content data/synthetic-market-test.json
```

This example borrows for runway, proposes tax/spending reforms, builds supply and administrative capacity, maintains support and repays debt when affordable. It uses the same validators, costs and delays as a player. The seed-17 acceptance run establishes that every initial country has a recovery path; it is not a universal optimal AI or a guarantee of balance for every seed. See [development progress](docs/development-progress.md) for the full remaining scope.

## Repository and research

- `crates/sim-core/`: content schema, pure daily simulation, account settlement, policies, politics, AI, saves and replay.
- `crates/headless/`: scenario runner and terminal game.
- `crates/desktop/`: native egui/eframe client with wgpu rendering.
- `data/`: versioned original game content and land geometry with provenance.
- [Implementation order](docs/implementation-order.md) and [requirement register](docs/implementation-register.csv): dependency-ordered roadmap with stable IDs.
- [Technical overview](docs/technical-research.md) and [world-data specification](docs/world-data-specification.md): intended architecture and content standards.
- [Eversim research](docs/eversim-research.md), [comparator research](docs/comparators-research.md) and [source register](docs/source-register.csv): references and coverage limitations.

Research snapshot: 2 November 2026. Advertised capabilities and planned features are distinguished from verified behavior. The inventories identify incomplete announcement and in-game coverage. See [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md) for project constraints.
