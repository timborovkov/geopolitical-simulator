# Development progress

The goal is the full documented game. The current implementation is a playable foundation; it is **not complete**. The [requirement register](implementation-register.csv) retains all 341 original requirements and their IDs. No unimplemented system is silently excluded. There are no delivery dates or workload estimates here.

## Current evidence

- Baseline governing campaigns were validated and pushed to `main` at `6475996`. Remote commit identity was verified. GitHub Actions could not be read through the available API/web connection; hosted results remain unverified.
- Local validation covers formatting, strict lints, tests, documentation, native release compilation, headless replay and successful crisis recovery in all 15 seeded countries.
- UI foundation work adds private/public projections, recoverable snapshots, interactive map controls and overlays, dated charts, pinned comparisons, asynchronous time controls, shortcut rebinding and scaling.
- The foundation checkpoint passed all 39 automated tests and the full local validation script. Native Linux checks exercised campaign loading, interface scaling and next-event advancement; the latter stopped at the fiscal boundary with the chart and comparisons still usable. These checks do not establish macOS or Windows behavior.
- The next foundation work adds distinct food/energy quantities with checked arithmetic, a reproducible fictional three-government test pack, structural command-journal validation and command/final-field replay diagnostics. The version-2 golden checksum remains unchanged. Broader dimensional typing, temporal content and historical geometry remain outstanding.
- This work passed the full local validation: 48 unit/integration tests, one compile-fail dimension check, strict lints/docs, native release compilation and successful seed-17 recovery/replay in all 15 starter countries and three fictional governments. A terminal playtest also exercised an unreformed deficit, growing arrears, pending construction, duplicate-policy rejection and save/load/replay. Hosted cross-platform CI remains unverified.
- Reference-country mapping now adds pinned Natural Earth outlines for the 15 starters, with island/hole triangulation, polygon selection and dated overlay fills. Tests distinguish the Lesotho interior hole, overseas parts, both sides of the antimeridian, non-roster territory and fictional marker-only packs. The source release is not certified as 2000 borders; historical sovereignty/control and administrative geometry remain incomplete.
- The map checkpoint passed 52 unit/integration tests plus the compile-fail dimension check and the full local validation script. Native Linux release checks selected Brazil away from its marker, loaded a saved campaign, displayed approval fills, selected France's land and preserved the camera across page changes. Source compilation and manifest validation also run offline; hosted macOS/Windows results remain unverified.
- Historical identity/coverage and major gameplay systems still require implementation and source review. Source facts, authored calibration, forecasts and simulated outcomes must remain distinguishable.

## Complete priority inventory

| Priority | System | Requirements touched / total | Status |
|---|---|---:|---|
| 01 | Simulation kernel and command boundary | 8 / 8 | Partial |
| 02 | Content schema and historical world identity | 5 / 7 | Partial |
| 03 | Saves, replay and simulation inspection | 6 / 7 | Partial |
| 04 | Native desktop shell, map and navigation | 6 / 7 | Partial |
| 05 | Accounts, units and causal reporting | 7 / 7 | Partial |
| 06 | Population, households and social groups | 4 / 7 | Partial |
| 07 | Production, consumption and domestic markets | 6 / 7 | Partial |
| 08 | Government institutions and legal powers | 7 / 11 | Partial |
| 09 | Policy and commitment engine | 6 / 7 | Partial |
| 10 | Government budget, taxation and public debt | 8 / 8 | Partial |
| 11 | Parties, elections and political survival | 8 / 14 | Partial |
| 12 | Baseline country AI and delegated administration | 7 / 9 | Partial |
| 13 | The complete governing interface | 9 / 9 | Partial |
| 14 | Utilities, energy and natural resources | 0 / 8 | Proposed |
| 15 | Construction, transport, housing and communications | 0 / 7 | Proposed |
| 16 | Public services, welfare and labor policy | 0 / 8 | Proposed |
| 17 | Monetary policy, banks and financial crises | 0 / 8 | Proposed |
| 18 | Trade, investment and commercial policy | 0 / 8 | Proposed |
| 19 | Firms, ownership and industrial policy | 0 / 8 | Proposed |
| 20 | Unions, civil society, media and domestic power | 0 / 8 | Proposed |
| 21 | Civil rights, justice and social legislation | 0 / 8 | Proposed |
| 22 | Migration, citizenship and integration | 0 / 7 | Proposed |
| 23 | Diplomacy and binding agreements | 0 / 8 | Proposed |
| 24 | International organizations, unions and blocs | 0 / 9 | Proposed |
| 25 | Military establishment, procurement and logistics | 0 / 9 | Proposed |
| 26 | Conventional warfare and operational AI | 0 / 9 | Proposed |
| 27 | Peace, occupation and territorial transitions | 0 / 8 | Proposed |
| 28 | Intelligence, covert action and cyber | 0 / 8 | Proposed |
| 29 | Organized crime, insurgency and internal conflict | 0 / 8 | Proposed |
| 30 | Climate, ecology, agriculture and disasters | 0 / 9 | Proposed |
| 31 | Epidemics, public health and demographic change | 0 / 8 | Proposed |
| 32 | Research, technology and structural change | 0 / 8 | Proposed |
| 33 | Nuclear strategy, arms control and strategic deterrence | 0 / 7 | Proposed |
| 34 | Space and advanced strategic infrastructure | 0 / 7 | Proposed |
| 35 | Tourism, culture, sport and soft power | 0 / 7 | Proposed |
| 36 | Historical world completion and differentiated countries | 0 / 7 | Proposed |
| 37 | Scenarios, bookmarks and campaign tools | 4 / 12 | Partial |
| 38 | Editors, modding and scenario inspection | 0 / 8 | Proposed |
| 39 | Full AI depth and long-campaign behavior | 0 / 8 | Proposed |
| 40 | Presentation, learning and accessibility polish | 1 / 8 | Partial |
| 41 | Multiplayer product implementation | 0 / 9 | Proposed |
| 42 | Optional player careers and extra presentation modes | 0 / 6 | Proposed |

## Next implementation boundaries

Continue closing the earliest requirements: typed identities/units and inspection, date-effective content and historical institutions, sovereign/administrative map geometry and picking, production/cohort depth, legal powers and meaningful political preferences. Then extend energy, construction and public services through their real resource/accounting constraints. Later priorities remain in the register, including diplomacy, organizations, warfare, intelligence, climate, technology, historical completion, editors, deeper AI and multiplayer.

Each feature needs a validated player action, costs/commitments, delayed consequences, usable feedback, an AI response and relevant invariants. Tests and a passing executable establish consistency; they do not certify historical accuracy, economic calibration, enjoyment for every player or untested platform behavior.
