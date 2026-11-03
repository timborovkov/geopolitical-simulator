# Geopolitical Simulator

An initial playable Rust geopolitical strategy game for macOS, Linux and Windows, starting on **1 January 2000**. Govern one of 15 countries through its institutions, fund public services, expand energy and food supply, manage debt and survive political transitions. Other governments act through the same command rules.

This is the first connected game slice of the research roadmap: **341 proposed requirements across 42 ordered systems**. It does not implement the full catalogue. See the [initial-game scope](docs/initial-game.md) for implemented loops, simplifications and remaining work.

## Play

Install [Rust through rustup](https://rustup.rs/) and a native linker: Xcode Command Line Tools on macOS, a C compiler/linker on Linux, or Visual Studio Build Tools with Desktop development with C++ on Windows. The committed toolchain selects Rust 1.99.1, rustfmt and Clippy. Linux desktop play needs an X11/Wayland session, libxkbcommon and a Vulkan or OpenGL-capable graphics driver; the simulation and terminal game need no graphics stack.

Launch the native desktop game:

```sh
cargo run -p desktop --release --locked
```

Choose a country, select **Sandbox** or the authored **Fiscal and supply crisis**, and click **Begin**. Advance a day, inspect households and accounts, preview a policy, secure consent, fund it and watch delivery. Space pauses/plays; F5 saves and F9 loads. The interface includes a guide and a five-year objective. Elections and leadership changes continue country play.

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

Both clients load the same bundled content. Supply `--content path/to/pack.json` to either executable to use a validated local JSON pack. Unknown schema, broken references, invalid ranges and incompatible saves produce explicit errors.

## World and game data

The [initial content pack](data/initial-2000.json) seeds 15 countries, 34 parties/governing bodies, eight institutional configurations and seven policy programs. The roster includes the UK, US, France, Germany, Russia, China, India, Japan, Brazil, South Africa, Saudi Arabia, Iran, Cuba, North Korea and Myanmar.

**Six reference-country population/GDP fields have pinned World Bank-mirror observations; other economic values, household profiles, seats, ideology profiles and political rules are authored gameplay approximations.** The source inspector distinguishes the two; the primary data API has not been independently reconciled. Historical identity names form a limited starter curation. Royal, military and religious governing bodies are explicitly scenario abstractions. All future decisions and successor cabinets are simulated. See [data provenance and coverage](data/README.md). The map uses Natural Earth public-domain land outlines and selectable country markers; political borders and subnational regions are not yet represented.

Money is expressed in millions of constant-2000 USD-equivalent game credits. Displayed local currency codes are identity metadata; there is no currency exchange model yet. Stocks and realized daily flows are shown separately. Policy forecasts state their assumptions.

## Save, recover and replay

The desktop writes manual saves to the path in its sidebar, initially `saves/campaign-v2.json`. Every crossed 30-day reporting boundary writes `saves/autosave-COUNTRY-SCENARIO-v2.json`. A completed save replaces the primary file using a temporary file and retains the previous snapshot as `.json.bak`. To recover, load that backup path explicitly. An interrupted `.json.tmp` is preserved for inspection and blocks replacement until resolved. Saves include content/rules hashes, commands, projects and deterministic shock state; incompatible or damaged files fail without replacing the active campaign.

The terminal uses `households`, `budget`, `data`, `save PATH`, `load PATH` and `replay`. Paths in terminal commands cannot contain spaces. `--save FILE` and `--load FILE` work in the runner. Save, rules and content formats are now version 2. Version-1 files are rejected without conversion or modification; the default v2 save paths keep them separate. Replay re-executes human commands and deterministic AI and checks the canonical final state hash. Hashes detect accidental corruption; they are not cryptographic authentication.

## Validation

Python 3.11+ is development tooling only. Run the same checks as CI:

```sh
python3 scripts/validate.py
```

This checks document/register consistency, local Markdown links, whitespace, Rust formatting, Clippy with warnings denied, tests, Rust docs with warnings denied, workspace release builds (including desktop) and the headless replay runner. CI is configured on macOS, Linux and Windows; actual platform validation requires those jobs to run. Desktop visual/interaction checks require a display and are separate from the headless checks.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
```

## Repository and research

- `crates/sim-core/`: content schema, pure daily simulation, account settlement, policies, politics, AI, saves and replay.
- `crates/headless/`: scenario runner and terminal game.
- `crates/desktop/`: native egui/eframe client with wgpu rendering.
- `data/`: versioned original game content and land geometry with provenance.
- [Implementation order](docs/implementation-order.md) and [requirement register](docs/implementation-register.csv): dependency-ordered roadmap with stable IDs.
- [Technical overview](docs/technical-research.md) and [world-data specification](docs/world-data-specification.md): intended architecture and content standards.
- [Eversim research](docs/eversim-research.md), [comparator research](docs/comparators-research.md) and [source register](docs/source-register.csv): references and coverage limitations.

Research snapshot: 2 November 2026. Advertised capabilities and planned features are distinguished from verified behavior. The inventories identify incomplete announcement and in-game coverage. See [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md) for project constraints.
