# Contributing

This repository contains research documents and an initial playable Rust game. `sim-core` owns deterministic rules and content validation, `headless` provides the scenario runner and terminal game, and `desktop` presents the native egui/wgpu client. Behavioral tests cover accounting, policy implementation, commands, political transitions, persistence and replay.

## Rust development

Install [rustup](https://rustup.rs/) and your platform's native linker as described in the [README](README.md). The repository pins the Rust toolchain, rustfmt and Clippy in `rust-toolchain.toml`. Keep `Cargo.lock` committed, including when only local workspace packages are present.

Run from the repository root:

```sh
python3 scripts/validate.py
```

This requires Python 3.11+ (use `python` on Windows). It runs the same checks as CI, including document consistency, Rust documentation and the release runner. Use `--docs-only` for document-only changes. It has no third-party Python dependencies and works from any current directory.

Individual Rust commands:

```sh
cargo run --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
```

The test profile compiles `sim-core` at optimization level 1 for long campaign/replay checks, retaining debug assertions and overflow checks. The other workspace packages keep their default test settings, and the release runner separately validates the production build. Hosted validation allows 45 minutes per operating system for cold builds and the complete recovery suite.

Use `cargo fmt --all` to apply formatting. Add meaningful behavioral tests as game systems are implemented. A passing empty test harness only verifies the initial workspace compiles. GitHub Actions runs these checks and the release runner on macOS, Linux and Windows; platform checks are only confirmed after those jobs run. CPU UI rendering tests cover the current screens without a GPU. Native interaction checks require a display; packaging and installer validation remain future work.

Keep the simulation independent of graphics, databases, networking and wall-clock time. Follow the boundaries in the [technical overview](docs/technical-research.md); introduce additional crates and dependencies as implemented features require them.

## Documents and review

Keep requirements in `docs/implementation-order.md` synchronized with `docs/implementation-register.csv`. Preserve stable IDs and update the requirement count when adding or removing requirements. Keep source citations and evidence limitations explicit.

Before committing, run `python3 scripts/validate.py` for code, manifests, toolchain, validation or CI changes; use `--docs-only` for document-only edits. The document checks compare each requirement's ID, priority, subsystem, order and text between Markdown and CSV, summary counts, source counts, CSV structure and local Markdown file links. They do not verify remote URLs or Markdown fragment anchors. Review `git status --short` and the full diff, including new files, for unintended files or credentials. Keep build output and secrets ignored; keep `Cargo.lock` tracked.

Use descriptive commits. For ongoing work, use focused branches and pull requests when requested or appropriate; do not rewrite shared history without authorization. After an authorized push, verify the remote commit and inspect the configured CI checks. There is no deployment or release process yet.
