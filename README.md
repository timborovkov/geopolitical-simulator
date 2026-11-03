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

## Save, recover and replay

The desktop writes manual saves to the path in its sidebar, initially `saves/campaign-v3.json`. Every crossed 30-day reporting boundary writes a country, scenario and content-specific filename in `saves`, ending in `v3.json`. A completed save replaces the primary file using a temporary file and retains the previous snapshot as `.json.bak`. Use **Inspect recovery snapshots** to review the primary, pending write, backup or interrupted backup/recovery stage before restoring. Originals are archived before recovery; a damaged primary never replaces the last validated backup. Interrupted writes block ordinary replacement until explicitly resolved. Saves include content/rules hashes, commands, projects and deterministic shock state; incompatible or damaged files fail without replacing the active campaign.

The terminal uses `until YYYY-MM-DD`, `households`, `budget`, `data`, `save PATH`, `load PATH` and `replay`. Paths in terminal commands cannot contain spaces. `--save FILE` and `--load FILE` work in the runner. Snapshot writer format is 3; rules and content remain version 2. Compatible format-2 snapshots load without changing their files or ordinary campaign checksums. Saving over a validated format-2 snapshot first archives its exact bytes to a unique `.format-v2-N` file; subsequent saves retain that archive. Recovery also preserves originals before writing format 3. Version-1 files remain unsupported and are rejected without modification. Replay re-executes player and explicitly marked developer commands alongside deterministic AI, checks commands in sequence and verifies the final state hash. Diagnostics identify the first differing command or final-state field; a final snapshot alone cannot establish the earliest divergent tick. Hashes detect accidental corruption; they are not cryptographic authentication.

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
