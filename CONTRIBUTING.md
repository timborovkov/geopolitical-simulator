# Contributing

This repository contains research documents and an initial playable Rust game. `sim-core` owns deterministic rules and content validation, `headless` provides the scenario runner and terminal game, and `desktop` presents the native egui/wgpu client. Behavioral tests cover accounting, policy implementation, commands, political transitions, persistence and replay.

## Rust development

Install [rustup](https://rustup.rs/) and your platform's native linker as described in the [README](README.md). The repository pins the Rust toolchain, rustfmt and Clippy in `rust-toolchain.toml`. Keep `Cargo.lock` committed, including when only local workspace packages are present.

Run from the repository root:

```sh
python3 scripts/validate.py
```

This requires Python 3.11+ (use `python` on Windows). It runs the same checks as CI, including document consistency, Rust documentation and the release runner. Dependent build, lint and test commands complete first. Independent in-memory recovery examples then use four bounded workers; every original command and its complete output remain grouped in input order, and any nonzero result fails validation. Use `--recovery-workers 1` to run those examples serially when diagnosing a failure. Only the memory-only recovery example is eligible; future commands that write saves or depend on another case must remain serial. Use `--docs-only` for document-only changes. It has no third-party Python dependencies and works from any current directory.

Individual Rust commands:

```sh
cargo run --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
```

The test profile compiles `sim-core` at optimization level 1 for long campaign/replay checks, retaining debug assertions and overflow checks. The other workspace packages keep their default test settings, and the release runner separately validates the production build. Hosted validation allows 360 minutes per operating system for cold builds and the expanded transactional campaign/recovery suite.

Use `cargo fmt --all` to apply formatting. Add meaningful behavioral tests as game systems are implemented. A passing empty test harness only verifies the initial workspace compiles. GitHub Actions runs these checks and the release runner on macOS, Linux and Windows; platform checks are only confirmed after those jobs run. CPU UI rendering tests cover the current screens without a GPU. Native interaction checks require a display; packaging and installer validation remain future work.

Keep the simulation independent of graphics, databases, networking and wall-clock time. Follow the boundaries in the [technical overview](docs/technical-research.md); introduce additional crates and dependencies as implemented features require them.

## Native launch check

After the release build, run `python3 scripts/check_native_launch.py`. On Linux use a display or `xvfb-run -a -s "-screen 0 1600x1000x24" python3 scripts/check_native_launch.py` with Mesa/Vulkan and the X11 keyboard runtime (`libxkbcommon-x11-0` on Debian/Ubuntu) installed. The launcher uses an isolated temporary working directory, requires a nonuniform screenshot read back from the actual wgpu viewport and waits for a clean process exit. It refuses missing reports, failed exits, hangs and unexpected files. `--report NEWFILE.json` retains the platform, backend, adapter, dimensions and executable hash without overwriting existing evidence.

The underlying `desktop --developer --native-launch-check NEWFILE.json` mode opens the ordinary start screen with default preferences, requests renderer readback and closes without starting or advancing a campaign. It writes its report only after the native event loop returns successfully. The `frames` counter records app updates, including any extra egui layout passes; it is not a GPU presentation or performance counter. Readiness comes from actual image readback and a clean exit. This is a launch check, not full interaction, installer or platform certification. CI runs it on each supported operating system after standard validation. A configured check is not a passing result; record observed platform outcomes separately.

## Documents and review

Keep requirements in `docs/implementation-order.md` synchronized with `docs/implementation-register.csv`. Preserve stable IDs and update the requirement count when adding or removing requirements. Keep source citations and evidence limitations explicit.

Before committing, run `python3 scripts/validate.py` for code, manifests, toolchain, validation or CI changes; use `--docs-only` for document-only edits. The document checks compare each requirement's ID, priority, subsystem, order and text between Markdown and CSV, summary counts, source counts, CSV structure and local Markdown file links. They do not verify remote URLs or Markdown fragment anchors. Review `git status --short` and the full diff, including new files, for unintended files or credentials. Keep build output and secrets ignored; keep `Cargo.lock` tracked.

Use descriptive commits. For ongoing work, use focused branches and pull requests when requested or appropriate; do not rewrite shared history without authorization. After an authorized push, verify the remote commit and inspect the configured CI checks. There is no deployment or release process yet.
