# Markets and trade design and implementation plan

Updated 11 November 2026. **Proposed extension, not implemented.** The player governs a country, the simulation advances daily, and the default bookmark is 1 January 2000. Victoria 3 is an inspiration for interconnected markets and understandable dependencies; these are original design proposals, not a verified account of its mechanics.

This repository plan supersedes the standalone discussion draft for implementation. It incorporates the readiness gaps identified in that discussion and reconciles them with the existing game. Track work in the [development backlog](development-progress.md#markets-and-trade-todo). This decomposes existing requirements rather than adding requirements or changing completion statuses.

## Intended player experience

A city depends on food reaching it, industrial producers depend on inputs, and exporters depend on access to buyers. Players inspect a shortage or opportunity, identify the constraint, and choose between infrastructure, domestic investment and commercial policy. Cheap imports, employment, fiscal receipts and resilience can conflict.

Every important movement should expose what changed, who is affected and which action addresses the cause. Removing a tariff can improve a route's economics; it cannot reopen a closed road. Proposed infrastructure must still obtain authority, funding, materials and delivery through the existing policy system.

## Existing foundation and integration constraints

| Existing foundation | Extension needed |
| --- | --- |
| [Food and energy markets](../crates/sim-core/src/markets.rs) already distinguish quotes and settlement and implement bounded integer price adjustment | Regional price state and explicit geographic trade; do not create a second incompatible price engine |
| [Industrial trade](../crates/sim-core/src/industrial_trade.rs) already has paid public contracts, fixed quotes, daily quotas and a bounded external counterparty | Bilateral routes, shared transport capacity and autonomous merchant behaviour; existing contracts are not these features |
| [Current progress](development-progress.md) records production recipes, delivered household consumption, private businesses, earnings, productive equipment and optional payroll claims | Connect merchants and regional inventories to these owners and accounts without minting money or duplicating production |
| Daily clock, deterministic commands, headless runner, replay and exact-pack saves already exist | Extend the same simulation and runner; preserve archived rules and introduce explicit versioned content/state changes |
| Government commands and private/public reporting already have authority and privacy boundaries | Commercial policy uses normal commands; firms retain agency and foreign reports retain existing visibility limits |

The recommended default is finance rules 38. Optional payroll uses rules 40 and snapshot 41. The first engineering decision must name the parent ruleset and dependencies; neither optional systems nor new markets become the default implicitly. Older packs and save/replay outcomes must remain supported according to existing compatibility policy.

The [regional integration contract](regional-market-integration.md) selects optional rules 40 as the parent for the proposed connected-market slice and records ownership, cross-border accounting, settlement barriers and compatibility boundaries. It reserves a future format; it does not implement or enable it. Exact economic rules and the reproducible fixture remain prerequisites.

## Proposed market model

A region groups owner-held local inventories and orders under regional prices. Settlements within it are aggregated initially. A transport graph links regions through roads, rivers and ports. Links have daily capacity, unit transport costs and availability. Customs membership controls permissions and tariffs, not a single frictionless price for all members.

Start with fixed market boundaries. Geographic reference divisions are not automatically economic regions: define stable market IDs, country ownership and explicit mappings. Closing a link affects access without changing market identity. Use one shared capacity ledger for all goods in both directions unless the authored link explicitly provides separate directional capacity.

Retain desired physical need, cash-backed orders and delivered consumption as separate measures. Unaffordable need is not zero need. Price caps do not provide missing goods. Unused goods remain in an owner's inventory, subject to explicitly configured storage and losses.

Household essentials receiving first claim is a **prototype allocation proposal**, not an approved universal rationing rule. Compare it with the existing domestic-use and funded-construction priorities before choosing the new settlement order. Explain any priority to the player and test its effects on producers and exports.

### Prices and production

Reuse the current integer price function as the first candidate. It computes absolute supply-demand imbalance, scales it by configured adjustment basis points divided by demand (at least one), caps pressure at that adjustment, and applies a rounded-down proportional change within configured price bounds. Equal supply and demand leave prices unchanged. Validate the proposed regional inputs, rounding, tiny-quantity behaviour and zero-supply/demand cases before adopting it.

The earlier draft's 10 percent daily movement and 0.25–4 times baseline bounds were illustrative tuning suggestions, not approved overrides. Select fixture values with stability evidence. Price-cap hits and unmet needs must remain visible so bounded prices cannot hide a failing economy.

Output is bounded by actual inputs, labour, equipment and available funds. Define a profit-sensitive utilisation rule within capacity before introducing endogenous expansion or closures. Reuse production-cost, depreciation and wage-claim accounts where the selected parent ruleset provides them. Specify what an insolvent producer can do; expected profit must never fund a purchase.

### Merchants and transport

Aggregate merchants by route initially. Expected unit margin is destination price less purchase price, transport costs and tariffs. Cash-backed purchases, available stock, destination demand and every link's capacity bound shipments. Unsold imports remain merchant-owned stock. Expected margins are not cash receipts.

Use one observed-price snapshot for route evaluation. Reserve budgets and shared capacity before committing export purchases; specify stable allocation and tie-breaking so multiple routes cannot claim the same funds, goods or destination orders. Release unused reservations after actual fills. Routes cannot repeat a region, and goods cannot be reshipped within the same tick.

Proposed first delivery rule: dispatch today, arrive next day. Distance affects cost and capacity usage; physical convoys and variable travel times are deferred. Closing a link prevents new dispatches; already-dispatched cargo arrives unless a separately recorded loss event applies. Later blockades may change this rule only with explicit loss ownership and compensation semantics.

Route opening and volume changes require specified profitability thresholds and persistence periods to avoid daily switching. Merchant reserves, retained earnings, distributions, losses and shutdown/re-entry must be defined; no automatic cash injection or free recapitalisation. Multiple merchants must not independently assume they can sell into the full same demand.

### Money and policy

Every payment identifies payer, recipient and purpose. Tariffs go to the treasury, transport receipts to the transport operator, wages and distributions to their actual owners. Identify explicit government purchases and transport-sector expenses so cash circulation can be inspected. Inventory values and receivables are not spendable funds. Current payroll claims and protected client funds cannot become merchant capital.

For the first connected scenario, implement import tariffs, link closures and funded infrastructure upgrades. Define tariff base, rounding, collecting country and collection point before coding; charging on actual dispatch is the initial proposal. Negotiate access, customs unions, export tariffs, embargoes, subsidies and wartime interdiction later without dropping their existing roadmap requirements. Private merchants respond to conditions; the government does not directly order their profitable trades.

## Prioritised implementation work

These are dependency-ordered tasks within this feature, not a replacement for the global priority order. All unchecked items remain outstanding in the linked backlog.

| Step | Existing requirements | Required output and exit gate |
| --- | --- | --- |
| 1. Freeze the integration contract | P01-04, P03-03, P05, P07-05, P19 | Name parent rules/content versions, region/owner IDs, typed quantities, money recipients, authority and visibility boundaries. Map each daily phase to existing settlement; document compatibility and transactional rollback. |
| 2. Write executable economic rules | P07-01–P07-07, P18-01–P18-02 | Exact price inputs, recipes, needs/budgets, utilisation, allocation and ties, merchant cash lifecycle, reservation release, tariffs and in-flight closure semantics. Include worked zero-stock, insufficient-cash and competing-route examples. |
| 3. Author the bounded scenario | P02, P07, P18-06, P37-04 | Three fictional regions, opening owners/cash/stocks, recipes, capacities, transport weights, costs, alternate route and shock inputs. Attribute all values as authored. Reconcile opening balances and provide reproducible generation. |
| 4. Extend headless simulation | P01-08, P03-04, P07-07, P18-01 | Run baseline, supply failure, road closure/reopening and tariff change through ordinary commands; emit reconciled price, delivery, welfare, cash and route diagnostics. Preserve earlier packs and replay. |
| 5. Add government controls and AI | P09, P12, P15, P18-02 | Validated paid actions with authority, delayed consequences and explainable forecasts; AI uses the same legal commands and available observations. |
| 6. Add player inspection | P13, P40-03 | Market and route views explain constraints, prices, delivered needs, ownership and realised profits; private account data remains private. Test native interaction after core behaviour passes. |

Use existing goods and recipe identifiers where possible. The earlier proposal of grain, timber, iron, coal, tools and cloth is a candidate teaching set, not a replacement for the game's existing food/energy and industrial goods. Before authoring data, choose a complete subset that closes its input chains; every good must have a producer, buyer and unit, or an explicitly bounded external supply. Do not add six unconnected labels.

## Proposed daily transaction sequence

This sequence must be reconciled with the selected parent ruleset in step 1; it is not an instruction to reorder archived settlement.

1. Receive prior-day shipments and record explicit losses; distinguish inventory arrival from subsequent sale.
2. Complete production using the previously committed inputs, labour and capital conditions.
3. Settle due income/claims once and form affordable household, producer and merchant orders from one price snapshot.
4. Reserve budgets and route capacity, then clear purchases against real stock using the specified priorities and tie-breaks. Unfilled quantities retain their unspent cash.
5. Dispatch actual purchased quantities and transfer actual tariffs/transport payments. Release unused reservations and retain unsold stock with its owner.
6. Reconcile cash, goods, input consumption, delivered household use, losses and outstanding claims. Record unmet physical need separately from unfilled orders.
7. Update next-day prices and route targets. Report realised results separately from forecasts.

Engineering must explicitly locate production input consumption, wage assessment versus payment, domestic consumption, protected public construction claims and private earnings in these phases. A single quantity or wage payment may not be counted twice across new and existing systems.

## Scenario and acceptance gates

The first milestone is a stable farming, mining and industrial-port economy that reacts intelligibly to a closed road. Provide an alternate path with higher cost and limited capacity. Grain at 10 in the origin and 16 at destination, with transport 3 and tariff 1, illustrates an expected margin of 2; these are illustrative arithmetic values, not simulated or historical results.

- **Baseline:** define a measurement window and numeric tolerances before execution. With unchanged inputs, prices and route volumes settle within those tolerances without growing oscillations. Record cap hits, unmet needs and idle production, not just average price.
- **Supply shock:** lower actual production; inspect inventories, deliveries, household affordability, price pressure and producer earnings against a matched baseline.
- **Road closure:** new shipments stop on the closed link, already-dispatched goods follow the selected rule, alternative routes share capacity, and reopening permits recovery when merchants have goods and cash.
- **Tariff change:** actual receipts reconcile to shipped quantities and the tariff rule; show consumer, producer and merchant consequences without asserting welfare always improves.
- **Accounting:** reconcile each owner's money and goods, production conversions, losses, transport receipts, tariff receipts and unpaid claims. No negative balances, double-reserved capacity or profitable circular same-day shipping.
- **Boundaries:** cover zero supply/demand, tiny quantities and integer rounding, exhausted merchant funds, competing directions/goods, isolated regions, unsold imports and failed/withdrawn commitments.
- **Determinism:** stable-ID allocation, reorder-invariant definitions where required, replay/resume equality and all-or-nothing failure on invalid content/state. Preserve archived campaign outcomes through the existing validation suite.
- **Player comprehension:** a playtester can identify why food delivery fell and choose an intervention that addresses the actual constraint using market and route views.

Measure tick cost at intended map size before setting a performance budget. Use the existing headless runner rather than a separate simulation with different rules. Required code validation follows [CONTRIBUTING.md](../CONTRIBUTING.md); design-only changes use its document checks.

## Interface and technical overview

`sim-core` owns definitions, live state, settlement, commands and visibility-filtered reports. `headless` exposes scenario runs and inspectable diagnostics. `desktop` consumes those reports and issues normal commands. Neither client recalculates economic truth. Persist market/route/ownership state and necessary pending shipments with explicit content/rules and snapshot compatibility.

The market view should show price history, stock, production, affordable demand, delivered consumption, unmet needs, imports and exports. Route inspection shows origin/destination, full landed cost, actual and target volume, bottleneck links, owner cash exposure and interruption reason. Policy previews distinguish certain mechanical charges from conditional future effects. Use observed flow changes for explanations; do not present an uncomputed causal decomposition as fact.

## Deferred scope and readiness

FX, bank credit, speculation, detailed merchant companies, smuggling, seasonal harvests, variable travel times and physical convoys are outside the first regional prototype. Existing storage/spoilage rules remain where applicable. Broader diplomacy, war logistics and customs institutions remain in the requirement register.

**Ready now:** implement the specification and scenario preparation tasks. **Not yet ready:** integrate an assumed complete regional market system into the default campaign. Resolve steps 1–3 before core implementation; demonstrate step 4 before interface expansion. Government control and the daily tick are already decided by this repository. Formula inputs, money circulation, production response, route allocation and compatibility still need the explicit outputs above.
