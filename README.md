# Geopolitical Simulator

A proposed Rust geopolitical strategy game for macOS, Linux and Windows, beginning in 2000. The player governs a country through its institutions, including democracies, dictatorships, single-party states, monarchies, military governments and hybrid regimes.

This repository contains research, an ordered implementation specification and a minimal Rust workspace. No gameplay is implemented yet. Multiplayer readiness is part of the architecture; networked play comes later.

## Documents

- [Implementation order](docs/implementation-order.md): 341 proposed requirements across 42 ordered systems, without schedules or workload estimates.
- [Implementation register](docs/implementation-register.csv): trackable requirements with stable IDs.
- [Technical overview](docs/technical-research.md): Rust simulation, UI, graphics, storage, AI, persistence and multiplayer architecture.
- [World-data specification](docs/world-data-specification.md): historical entities, institutions, ideologies, indicators, policies and assets.
- [Eversim research](docs/eversim-research.md): Power & Revolution and Geo-Political Simulator editions, features, DLC and publication coverage.
- [Comparator research](docs/comparators-research.md): Geoplanetical and Millennium Dawn mechanics and roadmaps.
- [Source register](docs/source-register.csv): references, evidence status and research gaps.

Research snapshot: 2 November 2026. Advertised capabilities and planned features are distinguished from verified behavior. The source inventories identify incomplete announcement and in-game coverage.

## Repository structure

- `crates/sim-core/`: independent simulation library boundary; game systems are not implemented yet.
- `crates/headless/`: executable setup smoke test, ready for a future scenario runner.
- `docs/`: research documents and CSV registers.
- `scripts/validate.py`: shared local and CI validation, using only Python's standard library.
- `.github/workflows/ci.yml`: full validation on macOS, Linux and Windows.

There are no third-party Rust dependencies, environment variables, services or deployments to configure. The planned desktop UI and world-data compiler have not been scaffolded.

## Local setup

Install Rust through [rustup](https://rustup.rs/) and the native platform linker: Xcode Command Line Tools on macOS (`xcode-select --install`), a C compiler/linker on Linux, or Visual Studio Build Tools with the Desktop development with C++ workload on Windows.

The committed `rust-toolchain.toml` selects Rust 1.99.1 with rustfmt and Clippy. From the repository root:

```sh
git clone https://github.com/timborovkov/geopolitical-simulator.git
cd geopolitical-simulator
rustup show
cargo run --locked
```

The executable prints a setup confirmation and exits. It does not load a world or simulate time. The default start year of 2000 remains a design requirement.

## Validation

Install Python 3.11 or newer for the repository validation command (CI uses Python 3.12). Python is development tooling; the Rust executable does not require it. Run the same checks as CI:

```sh
python3 scripts/validate.py
```

On Windows, use `python` if `python3` is unavailable. The command checks CSV structure and IDs, requirement text/order/counts, local Markdown file links, diff whitespace, Rust formatting, Clippy with warnings denied, tests, Rust documentation with warnings denied, release builds and the headless runner. It stops on the first failure.

For document-only edits, run `python3 scripts/validate.py --docs-only`. To run individual Rust checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
```

Keep `Cargo.lock` committed. See the contribution guide for validation expectations; no behavioral tests exist at this setup stage.

Build output lives in `target/` and is ignored, along with environment secrets, Rust backup files, Python caches and local agent state. `.editorconfig` supplies shared whitespace defaults; rustfmt handles Rust formatting. No Git hooks are installed. CI is configured, and its remote results can be confirmed after pushing.

See [CONTRIBUTING.md](CONTRIBUTING.md) for document maintenance and validation, and [AGENTS.md](AGENTS.md) for project constraints.
