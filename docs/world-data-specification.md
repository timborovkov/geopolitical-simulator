# World data and content specification

Companion to the [ordered implementation list](implementation-order.md). This specifies records and content to build, not a completed historical database. All catalogues below are proposed game content unless an external source is explicitly identified.

## 1. The meaning of complete world coverage

Create an explicit roster of **all political entities relevant on the selected start date**. Do not use a modern ISO-country list as the 2000 roster. Each entity must be classified: sovereign state, partially recognized state, dependent territory, autonomous jurisdiction, occupied territory, disputed area, international administration or other special status.

For each entity record whether it is selectable, simulated as a separate authority, administered through another authority, or only represented geographically. Record why. Tiny size is not itself a reason to make a country unplayable: the player may use the same institutional rules at lower economic detail.

“Every leader” means office-holders and influential actors relevant to simulation at the bookmark, with succession rules thereafter. It does not require individually simulating every official or legislator. “Every party” means the historical parties relevant under a documented inclusion rule, plus a representation for minor parties, with no silently fabricated facts. “Every statistic” means an extensible indicator registry where each value has a definition and source or calculation. These boundaries make completeness auditable.

### Required registries

| Registry | Required fields and relationships |
|---|---|
| Political entity | Stable ID, names/aliases, entity type, existence interval, predecessor/successor links, capital, territory, legal sovereignty, recognition relationships |
| Geographic atom | Stable ID, geometry reference, land/water class, adjacency, terrain, climate/exposure, jurisdiction and control relationships |
| Administrative unit | Parent, level, name, validity interval, grouped geographic atoms, powers, fiscal relationships |
| Government | Entity, regime/institution configuration, mandate, coalition, appointment date, confidence and legitimacy measures |
| Office | Authority, jurisdiction, selection process, tenure, eligibility, powers, succession procedure, incumbent terms |
| Person | Names, dates where sourced, office terms, affiliations, career facts; simulated preferences/traits separately labeled |
| Party | Founding/dissolution, aliases, predecessors/mergers, legal status, regional scope, leadership, seats, support, multidimensional positions |
| Legislature | Chambers, seats, selection rules, constituencies, term, quorum, vote thresholds and amendment rules |
| Election | Office/body, date/rule, electorate, contestants, system, observed results or simulated outcome |
| Interest group | Membership basis, overlap rules, leadership, resources, organization, goals and institutional access |
| Organization | Charter, existence dates, membership, offices/councils, vote rules, mandate, budget and enforcement |
| Treaty | Parties, clauses, signatures, ratification, entry into force, term, obligations, monitoring and breach rules |
| Currency | Issuer, unit/scale, regime, participating entities, effective dates and conversion events |
| Good/service | Unit, sector, inputs, substitution group, essential uses, storage/losses and trade characteristics |
| Facility | Type, location, owner/operator, capacity, staffing, input/output, condition, commissioning/retirement and upgrade status |
| Firm | Sector, ownership, facilities, workforce, assets/liabilities, contracts, market position and governance |
| Military formation | Command, role, personnel, equipment, location, readiness, supply, access and standing orders |
| Technology | Prerequisites, research requirements, adoption requirements, effects and era/scenario eligibility |
| Policy | Legal route, parameters, jurisdiction, costs, implementation, effects, interactions, visibility and AI evaluation |
| Scenario | Date, content dependencies, starting-state overlays, rules, objectives, event weights and end/continuation conditions |

Names and flags are presentation data, never identity keys. A renamed country must keep its accounts and obligations. Party mergers and state succession require explicit relationships rather than overwriting old records.

## 2. Temporal model

Use both **when a fact applied** and **when the source reported or revised it**:

```text
FactRecord
  subject_id + field_id
  value + unit + geographic_basis
  valid_from + valid_to
  observation_period
  publication_date + retrieved_at
  source_id + source_locator + source_version
  quality_class + uncertainty + transformation_id
```

Examples of important distinctions:

- An annual GDP observation is a flow during a year; population may be a midyear estimate; debt is a stock at a stated date.
- A person can hold a party office and a state office simultaneously, with different term intervals.
- Membership may have separate signature, ratification and effective dates.
- Present boundaries may not match the territory used by a historical statistical series.
- A later revision of a 2000 statistic is useful for calibration but was not necessarily available to a government in 2000.

Initialize with historical facts, then branch. Future elections, leaders, wars and independence do not automatically occur on their actual historical dates unless a scenario explicitly requires them. Separate **historical source records** from **campaign event history**.

## 3. Data quality and completeness

Use six value classes: observed; reconciled; interpolated; inferred; authored gameplay approximation; unknown. Never treat an absent value as zero. Record uncertainty where it changes decisions.

For every country, generate a coverage report across borders, institutions, people, parties, population, economy, services, infrastructure, military and organizations. Show required fields, present fields, estimates, conflicts and unresolved sources. Completeness is not just a percentage: missing a head of government or currency is more consequential than missing a minor tourism statistic.

Essential validations:

- A required office has a valid incumbent or an explicit vacancy/collective rule.
- Legislative seats reconcile, coalition parties exist and elections use a legal rule.
- Country and region populations reconcile; cohorts do not duplicate people.
- Monetary values carry a currency and price basis; PPP and market exchange rates are not interchangeable.
- Every source series is mapped to the correct territorial entity and observation period.
- Memberships, facilities and technologies exist at the bookmark date.
- Administrative geography has no unexplained overlaps or gaps; disputes use explicit overlapping claims.
- Opening balance sheets and sector flows are reconciled, with documented balancing adjustments.
- Sources and assets have redistribution status recorded before packaging.

AI assistance can propose imports, mappings, descriptions and missing-data estimates. It cannot certify a historical fact by generating a plausible value. Keep raw evidence, transformations and review status.

## 4. Source families and packaging decisions

| Source | Intended use | Evidence and handling |
|---|---|---|
| Natural Earth | Coastlines and base cartography | [Public-domain terms](https://www.naturalearthdata.com/about/terms-of-use/). Build a curated historical sovereignty/control layer; current boundaries are not a 2000 bookmark. |
| geoBoundaries | Administrative geometry | [API and licensing distinctions](https://www.geoboundaries.org/api.html). Use gbOpen with file-level metadata; validate represented year. Do not substitute restricted collections. |
| Wikidata | IDs, names, offices, parties, dated relationships | [Structured data licensing](https://www.wikidata.org/wiki/Wikidata:Licensing) and [office-holder model](https://www.wikidata.org/wiki/Wikidata:WikiProject_Heads_of_state_and_government/Data_Model). CC0 structured facts; verify completeness and source statements. |
| World Bank | Economic/development indicators | [License policy](https://datacatalog.worldbank.org/public-licenses), [Indicators API](https://datahelpdesk.worldbank.org/knowledgebase/articles/889392). Inspect each dataset’s terms, units and coverage, including third-party restrictions. |
| UN World Population Prospects | Population and age structure | [Official dataset](https://population.un.org/wpp). Historical estimates and projections are different products. Verify exact download/version terms before redistribution. |
| ParlGov | Elections, parties and cabinets | [Project](https://parlgov.org/). Its stated scope is EU/OECD democracies through 2023, not all countries. Verify the selected release’s redistribution terms. |
| V-Dem | Institutional comparison/calibration | [FAQ](https://www.v-dem.net/about/faq/) states CC BY-SA 4.0. Make a deliberate derivative-data licensing decision before bundling. |
| Manifesto Project | Party-position research | [Terms](https://manifesto-project.wzb.eu/information/documents/terms_of_use) prohibit redistribution without written authorization. Do not bundle merely because a third-party directory calls it open. |
| Correlates of War | Potential institutional/conflict reference | [Published dataset terms](https://correlatesofwar.org/data-sets/) restrict commercial use and redistribution. Not a default game-content import. |
| National statistical/election authorities | Missing country detail and historical validation | Candidate source family; verify each publication and reuse license. Prefer original tables and legal/institutional records. |
| Central banks, treaty repositories and organizations | Currency, legal powers and memberships | Candidate source family; import dated facts rather than assuming today’s arrangements. |
| FAO, ILO, WHO, UNESCO, IMF and other agencies | Sector-specific calibration | Candidate source family, not a blanket assertion of open redistribution or complete coverage. |

Data access, analytical use and redistribution inside a commercial game are different questions. Keep restricted reference material out of packaged data unless the required permission is obtained. The implementation can proceed using permitted sources and authored, labeled approximations.

## 5. Indicator registry to implement

Each indicator needs an ID, label, unit, stock/flow/rate type, time basis, geographic scope, aggregation rule, calculation or source, visibility, uncertainty and explanatory components. Ratios are recomputed from aggregates; country percentages must not simply be added.

| Domain | Indicator families |
|---|---|
| Population | Total, age structure, births, fertility, deaths, mortality causes, life expectancy, household size, urbanization, dependency ratios, internal/international migration |
| Household welfare | Disposable income, consumption basket cost, savings, essential consumption, poverty, inequality, housing burden, service access, food insecurity |
| Labor | Working-age population, participation, employment, unemployment, underemployment, vacancies, skill mismatch, wages, hours, productivity, informal work |
| National accounts | Nominal/real GDP, GDP per capita, sector value added, consumption, investment, government consumption, exports/imports, income flows, depreciation |
| Prices | Consumer basket indices, producer prices, wages, rents, energy/food prices, import prices, inflation and inflation expectations |
| Government finance | Revenue by instrument, spending by function, primary/overall balance, cash, commitments, arrears, debt by currency/maturity/creditor, interest, debt service |
| Finance | Deposits, loans, defaults, bank capital/liquidity, credit growth, rates, reserves, exchange rates, current/external balances, asset values |
| Industry | Capacity, utilization, output, inventories, intermediate inputs, profitability, closures, investment, ownership, concentration and unmet demand |
| Energy/resources | Reserves, production, imports, consumption, storage, generation, peak demand, reserve margin, grid capacity, outages, resource dependency |
| Infrastructure | Capacity, condition, congestion, connectivity, travel/freight cost, project progress, maintenance backlog, housing stock and vacancies |
| Health | Capacity, staff, coverage, waiting/backlog, disease prevalence/incidence, preventable mortality, vaccination and public-health readiness |
| Education/science | Enrollment, completion, attainment, literacy, staff, capacity, skills, research capacity, technology readiness and adoption |
| Environment | Emissions, energy intensity, air/water pollution, waste, recycling, forests, land use, biodiversity proxies, hazard exposure and adaptation |
| Institutions | Administrative capacity, enforcement, court backlog, rule compliance, corruption estimates, public trust and reporting quality |
| Politics | Seats, support, turnout, polling interval, coalition strength, issue salience, approval by group, legitimacy and protest organization |
| Security | Crime categories, reported versus estimated incidence, police/judicial capacity, organized-crime reach, insurgent control and threat estimates |
| Diplomacy | Trade/debt/security dependence, trust, threat perception, treaty obligations, sanctions exposure, recognition and voting alignment |
| Defense | Personnel, equipment, readiness, training, supply, maintenance, mobilization, losses, procurement obligations and deployment capacity |
| Culture/tourism | Visitors, receipts, capacity, seasonal utilization, cultural access, event costs, reputation and environmental pressure |

Do not show false precision. Statistics collected through weak institutions should have wider uncertainty, delays or revisions. A forecast must state its scenario and assumptions.

## 6. Goods, sectors and assets

Start with a coarse classification and support expansion without rewriting the simulation. Every added product must justify a distinct constraint or player decision.

**Resource families:** food crops, livestock/fish, timber, water, coal, crude oil, natural gas, uranium, iron, copper, aluminum inputs, other industrial minerals, precious metals and critical minerals.

**Intermediate goods:** electricity, refined fuels, steel, cement, chemicals, fertilizer, plastics, industrial components, semiconductors, batteries, machinery and construction inputs.

**Final goods/services:** processed food, clothing, household goods, vehicles, electronics/phones, medicines, medical equipment, housing services, transport/logistics, retail, finance, communications, healthcare, education, tourism and public administration.

**Strategic output:** arms/equipment families, ammunition, aircraft/ships/vehicles, satellites and launch services. Model quality/technology variants only where they affect compatibility, capability or trade restrictions.

**Illicit goods/services:** separate legal status from physical demand and production. This supports legalization, enforcement and substitution without deleting a market by changing a law.

Each facility definition specifies inputs, outputs, labor/skills, capacity, operating cost, build requirements, maintenance, pollution and transport access. Public-service facilities also have access and service-quality rules.

## 7. Policy definitions

```text
PolicyDefinition
  id, ministry, labels, jurisdiction
  parameters and units
  prerequisites and exclusions
  legal_route and voting_requirements
  implementation_stages
  cash_cost, recurring_cost, staffing, material_requirements
  enforcement_requirements
  effect_functions and interaction_tags
  transition, expiry, repeal and grandfathering rules
  affected_groups and forecast_model
  ai_candidate_generator and evaluator
  explanatory_text_keys and test_fixtures
```

A numerical rate, legal permission, institutional arrangement and construction program are different parameter types. Do not force all four into a single percentage slider.

Example: a carbon levy specifies taxable emissions, rate and currency basis, collection/enforcement, exemptions and revenue treatment. Its effects flow through costs, prices, substitution, investment and household budgets. It does not directly execute “reduce emissions by X, reduce popularity by Y.”

Example: raising healthcare funding authorizes resources. Hiring and construction depend on worker supply, training, procurement and facilities; outcomes depend on service capacity and actual population needs. The causal path is inspectable.

Example: a pension reform changes eligibility and future obligations. Grandfathering, phase-in and existing entitlements must be modeled so the change cannot retroactively erase accrued obligations unless an explicit policy does so.

## 8. Historical political actors and organizations

People need roles, competence estimates, policy preferences, networks and motivations only to the extent the simulation uses them. Separate sourced public facts from authored personality variables; the latter are game assumptions, not factual diagnoses of real people.

Parties require more than a left/right number: economic position, social policy, institutional preferences, national/regional orientation, foreign alignment, voter base and internal factions. Where evidence is missing, use an explicit estimated profile rather than inferred certainty from a party name.

Organization content should be expressed using reusable primitives:

- Membership/observer/associate status and dated entry/exit.
- Eligibility, application, negotiation and ratification.
- One-member-one-vote, weighted voting, unanimity, qualified majority, veto and rotating seats.
- Contributions, pooled funds, lending and redistribution.
- Joint standards, market access, common policies and military commitments.
- Dispute settlement, monitoring, sanctions, suspension and withdrawal.
- Amendment, deepening integration, breakup and new organization formation.

The initial data catalogue should cover universal institutions, military alliances, regional organizations, economic/trade/monetary unions, producer groups, development institutions and specialized agencies. Institution-specific exceptions belong in explicit rule definitions. Later entities and changes must not appear in a 2000 start merely because they exist in the current source.

### Regimes, ideology and effective power

Democracy, dictatorship and single-party rule are first-class playable configurations. Represent a regime through independent institutional dimensions rather than one democracy score:

| Dimension | Content to represent |
|---|---|
| Executive selection | Competitive election, parliamentary appointment, party selection, military council, hereditary or religious succession, personal designation |
| Political competition | Open competition, dominant-party competition, restricted opposition, party monopoly or no-party system |
| Effective authority | Legislature, executive, party leadership, military, religious hierarchy, royal household and personal networks |
| Constraints | Courts, constitution, internal party rules, elite vetoes, territorial autonomy and actual enforcement |
| Control and information | Media independence, censorship, political policing, civic organization, reliability of reporting and expressed versus private support |
| Economic organization | Ownership, coordination/planning, market allocation, redistribution, labor relations and foreign openness |
| Succession and survival | Electoral mandate, leadership coalition, cadre support, armed-force loyalty, dynastic legitimacy or religious authority |

Record formal rules and effective arrangements separately. A single-party state may have collective leadership or personalist domination; a monarchy may be constitutional or absolute; economic liberalization does not automatically create political competition.

Store ideology as a versioned definition with a historical/local label and multidimensional preferences. Attach distinct ideology profiles to people, parties, factions, social groups and the state’s declared doctrine. Include confidence and source provenance for historical classifications; campaign drift and policy departures are simulation events. Ideology influences objectives, acceptable compromises and relationships, while institutional powers determine available procedures. Neither should directly guarantee prosperity, stability or collapse.

Authoritarian systems need additional records for ruling-party bodies, cadre appointments, leadership factions, elite support networks and succession procedures. Opposition can be legal, restricted, underground or exiled. Public approval, private opinion, fear, legitimacy, organized resistance and elite loyalty are different variables. These are abstract simulation mechanisms, not a requirement to model individual political victims.

## 9. Visual and audio asset specification

- Original visual identity: restrained political atlas, clear regional overlays, readable data panels, consistent pictograms and limited decorative animation.
- Map masters: geographic source layers with provenance; compiled topology/meshes are generated outputs.
- Icons: editable SVG masters, consistent stroke/shape language, purpose-specific alternatives where generic icons are ambiguous.
- Flags: dated identity and variant metadata; text/name fallback for missing assets.
- Portraits: consistent commissioned/original vector-style treatment or licensed images, with a clear generated-person fallback. Do not mix random web photographs with incompatible rights and styles.
- Fonts: licensed for bundling, broad language coverage, tabular numerals and readable small-size rendering.
- Audio: subtle feedback, event priority cues and optional music; every cue also has a visual equivalent.
- Asset manifest: ID, creator, source URL, license, attribution, modification history, applicable dates, checksum and build transformation.

Use [Lucide’s published license information](https://lucide.dev/license) when selecting its generic icons, and inspect individual image licenses for any Commons assets. A data source’s license does not automatically cover a linked photograph or emblem.

## 10. Content-build pipeline

```text
source files + provenance
  -> schema validation and normalization
  -> entity/time/geography crosswalks
  -> explicit conflict resolution and estimates
  -> accounting and institutional reconciliation
  -> geography/topology/LOD compilation
  -> bookmark assembly
  -> automated validation and coverage report
  -> versioned data + asset packs
```

Store the transformation rules and review decisions alongside the content. Rebuilding the same source versions should produce the same content hash. New source releases create reviewed content updates; they do not silently modify a player’s running campaign.

## Global reference atlas boundary

The [map presentation contract](map-presentation.md) adds display-only Natural
Earth feature and settlement identities. Country features retain source `NE_ID`
and `ADM0_A3`; only explicit code matches to the loaded scenario enable country
reports or actions. Unmatched features are reference-only. Settlement membership
is source attribution, not sovereign recognition or a simulation population link.

The manifest records source revisions, input/output SHA-256, public-domain
licensing and historical limitations. Global geometry, transport and settlement
locations are not certified for 2000. Infrastructure is optional reference
cartography and contributes no capacities, routes, production or authority.
The original scenario packs and content hashes remain unchanged.
