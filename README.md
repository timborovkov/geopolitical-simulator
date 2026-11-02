# Geopolitical Simulator

A proposed Rust geopolitical strategy game for macOS, Linux and Windows, beginning in 2000. The player governs a country through its institutions, including democracies, dictatorships, single-party states, monarchies, military governments and hybrid regimes.

This repository currently contains research and an ordered implementation specification. No game implementation exists yet. Multiplayer readiness is part of the architecture; networked play comes later.

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

`docs/` preserves the research documents and CSV registers. There are currently no dependencies, build commands, environment variables, services or deployments to configure.

See [CONTRIBUTING.md](CONTRIBUTING.md) for document maintenance and validation, and [AGENTS.md](AGENTS.md) for project constraints.
