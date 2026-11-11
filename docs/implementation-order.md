# Geopolitical strategy game — implementation order

Research snapshot: **2 November 2026**. Target: **Rust; macOS, Linux, Windows; offline single-player first; multiplayer later; government/country control; default start 1 January 2000**.

This is an ordered build specification, not a schedule or workload estimate. Numbered systems are in recommended dependency order. Each system includes the things to implement inside it; later additions remain in the catalogue rather than being silently dropped. The player governs a country through its institutions. Firms, parties, unions, courts and foreign governments have their own agency.

**341 proposed requirements across 42 ordered systems.** The [CSV implementation register](implementation-register.csv) provides stable requirement IDs for tracking. The [source register](source-register.csv) records 81 references and coverage-gap entries; its evidence statuses distinguish reviewed descriptions from broader source citations and inaccessible material.

The central design requirement is **a playable, inspectable simulation**: every significant choice has constraints, affected groups, costs, implementation time, consequences and an explanation. A large collection of disconnected sliders does not satisfy this requirement.

## How to use the research

- [Eversim feature and publication inventory](eversim-research.md): Power & Revolution 2019–2023; GPS5; GPS2026; relevant DLC, manuals, scenarios and developer updates; GPS2027 separately.
- [Geoplanetical and Millennium Dawn inventory](comparators-research.md): published mechanics, recent updates and planned features, with status distinctions.
- [Technical overview and source-backed choices](technical-research.md): Rust architecture, UI/rendering, database, AI, persistence, multiplayer preparation, assets and historical data.
- [Data and content specification](world-data-specification.md): what “every country, leader, party and statistic” means as implementable records.

The references are capability research, not proof that the games implement their advertised mechanics correctly. GPS2026 is the latest verified Steam edition; GPS2027 was announced as a direct release on 30 October, with Steam later. The source inventories document coverage gaps. **This research does not enumerate every internal variable, in-game menu entry or historical announcement.** Those are not fully exposed by public store descriptions. The requirements below are our own proposed implementation, informed by the inventories, not a claim of exact mechanical parity or a transcription of proprietary rules.

## Priority 01 — Simulation kernel and command boundary

- Integer simulation clock, calendar, pause, speed levels, advance-to-date and advance-to-next-important-event.
- Stable IDs for every actor, place, institution, agreement, asset, policy and event; no identity based on names or array positions exposed outside a store.
- Typed units for money/currency, population, energy, goods, distances, rates and time. Explicit rounding and overflow handling.
- Explicit phase order; seeded random streams per subsystem; reproducible runs and canonical state hashes.
- All actions enter as validated commands. UI, AI, scripts and future remote players use the same action rules.
- Domain events record accepted outcomes; rejected commands explain the failed condition.
- Separate world truth, actor knowledge and player-visible information now, before espionage and multiplayer arrive.
- Headless executable that can load a tiny world and advance it without graphics or SQL in the inner loop.

**Working result:** the same scenario, seed and commands reproduce the same world; changing frame rate cannot change the outcome.

## Priority 02 — Content schema and historical world identity

- Versioned definitions for countries, territories, offices, people, parties, organizations, goods, sectors, laws, technologies and scenario rules. Minimal dated technology eligibility is available immediately; the full research/adoption graph comes later.
- Date-effective records: founding, dissolution, terms of office, membership, currency and territorial changes.
- Country identity separate from government, regime, legal sovereignty, occupation, recognition and map geometry.
- Source provenance, observed year, units, revisions, uncertainty and missing-data status for facts.
- CSV/JSON import and validation, stable external-ID crosswalks, reproducible content packs and localization keys.
- A small synthetic test world plus the scaffold for the entire historical world. Synthetic fixtures make rules testable without relying on incomplete historical data.
- Country-specific behavior expressed through institutional definitions and configuration, rather than scattered checks against country names.

**Working result:** a bookmark can contain different borders, leaders, parties and memberships without recompiling the engine.

## Priority 03 — Saves, replay and simulation inspection

- Versioned snapshot, command journal, scenario/content hash, mod order, random state and rules version.
- Autosave rotation, atomic completion, recovery from interrupted writes and clear missing-content errors.
- Migration policy; preserve the old save before conversion. Distinguish save compatibility from replay compatibility.
- Debug state inspector, economic ledger, AI decision log and first-divergence report.
- Historical statistics with bounded retention: recent fine-grained records, older aggregates and important events preserved.
- Save/load during construction, elections, negotiations, battles and pending legislation as those systems appear.
- Developer sandbox commands flagged separately from normal play; inspectability is part of the base game.

**Working result:** saving and resuming at a tick boundary gives the same subsequent results as uninterrupted play.

## Priority 04 — Native desktop shell, map and navigation

- Native builds and launch checks on all three operating systems from the beginning.
- Vector-derived world map: coastline, land, country and administrative boundaries, islands, lakes and rivers; pan, zoom, selection, search and tooltips.
- Separate display geometry from simulation regions. Selection must work for islands, enclaves and tiny states.
- Political, terrain and administrative layers first; overlay infrastructure added by later systems.
- Country overview, ministry navigation, sortable tables, charts, comparison tray, searchable encyclopedia and persistent selection.
- Keyboard navigation, key rebinding, high-DPI/UI scaling, readable compact text, Unicode/IME support and colorblind-safe encodings.
- Pause and speed visible at all times. Escape/back behavior and window focus must be predictable.

**Working result:** select a country, locate its regions, inspect a record and return to the same context without losing map position.

## Priority 05 — Accounts, units and causal reporting

- Treasury, household-sector, business-sector and foreign-sector accounts; add financial intermediaries when their rules exist.
- Stocks versus flows; balances versus rates; nominal versus real values; local currency versus reporting currency.
- Public spending, receipts, transfers, obligations, cash and financing; distinguish authorization, commitment and payment.
- Physical goods accounts: production, use, inventories, losses, imports and exports.
- Currency creation/destruction, default, depreciation, casualties and other non-conservation events must have explicit rules and ledger entries.
- Every headline statistic exposes components, period, units, recent changes and relevant uncertainty.
- Forecasts are separate from realized accounts; expected revenue must not be spendable cash unless a financing mechanism exists.

**Working result:** an expense has a funding source and recipient; a shortage has identifiable demand and supply; unexplained money or goods cannot appear.

## Priority 06 — Population, households and social groups

- Aggregate cohorts by region, age band, employment/occupation, income and selected social attributes; do not simulate billions of individual people.
- Births, deaths, aging, household formation and migration balances.
- Income, savings, consumption baskets, housing costs, purchasing power, poverty, inequality and essential needs.
- Employment status, skills, education and health; people can move between cohorts without being duplicated.
- Political interests, issue salience, trust, perceived living standards and institutional satisfaction.
- Overlapping group membership: a person may be a worker, parent, religious adherent and union member. Avoid double-counting population across interest groups.
- Public estimates can differ from world truth through census quality and statistical capacity.

**Working result:** a price rise or tax change affects identifiable households and subsequently changes behavior and political opinion.

## Priority 07 — Production, consumption and domestic markets

Implementation detail: [markets and trade plan](markets-and-trade-design.md), with dependency-ordered tasks in the [development backlog](development-progress.md#markets-and-trade-todo). These elaborate the existing requirements; they do not change their status.

- Data-driven goods and services with units, substitutability, storage, perishability and essential-use priority.
- Sectors with labor, capital, energy and intermediate-input requirements; output limited by actual bottlenecks.
- Agriculture, extraction, manufacturing, construction and services represented by the same framework where appropriate.
- Prices, wages, demand response, production costs, profitability, investment, capacity utilization and closures.
- Stockpiles, shortages, rationing, strategic reserves, imports and exports through a bounded external-market counterpart with explicit cash settlement and supply limits. Initial credit exists only through modeled debt records; full banking and trade refine these rules later.
- Distinguish desired consumption from affordable demand and delivered consumption.
- Bounded price-adjustment rules and settlement order; ensure oscillations are economic behavior rather than an accidental update-loop artifact.

**Working result:** an energy or input shortage reduces feasible production and has traceable effects on prices, employment and welfare.

## Priority 08 — Government institutions and legal powers

- Head of state, head of government, cabinet, ministries, civil service, legislature and courts as separate offices/bodies.
- Parliamentary, presidential, semi-presidential, monarchical, one-party and military-government configurations.
- Appointment, dismissal, succession, eligibility, term limits, vetoes and emergency powers.
- Chambers, seat allocation, voting thresholds, quorum, committees and constitutional amendment requirements.
- Federal, regional, municipal and autonomous powers; control over taxation and service delivery.
- Independent institutions such as central banks and election authorities have their own mandates. The player cannot directly order powers the government does not possess.
- Rule-of-law, bureaucratic capacity and corruption affect implementation without becoming a universal magic multiplier.
- Fully playable personalist dictatorships, single-party states, military juntas, absolute monarchies, theocracies, dominant-party and hybrid regimes alongside democracies. These are core political configurations, not optional late-game scenarios.
- Separate constitutional rules from effective power: ruling-party bodies, military councils, religious authorities, security services, patronage networks and personal rule can hold actual decision rights.
- Authoritarian governing routes include decrees, party/council decisions and controlled legislatures; material capacity and elite cooperation still constrain execution. Model violations or suspension of constitutional rules as explicit contested actions rather than making regime change impossible through legal validation.
- Regime change in both directions: democratization, concentration of executive power, party monopoly, military takeover, restoration and negotiated transition, with distinct causes and institutional consequences.

**Working result:** two countries can face the same policy problem but have different legal routes to address it.

## Priority 09 — Policy and commitment engine

- Shared lifecycle: drafted, proposed, amended, approved, funded, implementing, operating, evaluated, repealed/expired.
- Prerequisites, jurisdiction, legal route, administrative cost, staffing, enforcement, start date, duration and transition rules.
- Policy levels and thresholds; mutually exclusive choices, grandfathering, regional exemptions and interactions.
- Delayed effects, capacity ceilings, diminishing returns and unintended effects defined explicitly.
- Commitments reused by election promises, coalition agreements, contracts, treaties and project deadlines.
- Preview cards: legal feasibility, budget effect, affected groups, implementation bottlenecks and forecast range.
- Outcome inspection separates the direct effect, indirect effects, external shocks and uncertainty.

**Working result:** passing a law does not instantly build hospitals or create trained staff; the interface explains the remaining steps.

## Priority 10 — Government budget, taxation and public debt

- Budget proposal, enactment, supplementary budgets, execution and year-end reporting.
- Income bands, corporate tax, VAT/sales tax, social contributions, excise, property/land, capital gains, inheritance, resource royalties and customs revenue.
- Tax base, collection capacity, compliance, evasion and enforcement; a nominal tax rate is not guaranteed revenue.
- Public wages, procurement, benefits, transfers, grants, interest, subsidies and capital spending.
- Debt instruments with creditor, currency, maturity, interest and repayment schedule; domestic versus external debt.
- Borrowing limits, refinancing, arrears, restructuring, default and emergency financing.
- Automatic stabilizers and transparent fiscal forecasts under alternative assumptions.
- Taxes and spending affect households, firms and political support through actual accounts.

**Working result:** the player can enact a funded budget, follow its execution and understand an emerging financing problem.

## Priority 11 — Parties, elections and political survival

- Real historical party identities and leaders at the bookmark; affiliation, legal status, organization and regional support.
- Multidimensional preferences: economic, social, institutional, regional and foreign-policy positions.
- Electoral calendars and rules for executive, lower/upper house, regional and local elections as configured.
- Constituencies, thresholds, turnout, proportional and district allocation, coalitions and confidence/support agreements.
- Polls with uncertainty; campaigning, media access, funding, activists, debates and delivery of promises.
- Party discipline, dissent, defections, splits, mergers, leadership contests and dissolution.
- Confidence votes, caretaker governments, constructive no-confidence, dissolution and peaceful transfer of power.
- Explicit country-continuation rule: default country play continues with the incoming government, constrained by its mandate. Optional office-bound challenges end on removal.
- Regime-specific political survival: competitive elections, internal party leadership, elite coalitions, military backing, dynastic succession and religious appointment. Elections may be absent, restricted or noncompetitive; approval alone does not determine control.
- Single-party organization: congresses, leadership bodies, factions, cadre appointments, party-state relationships, internal bargaining and succession. Model a party-led state separately from a ruler who dominates the party personally.
- Ideology definitions cover liberal, conservative, social-democratic, socialist, communist, nationalist, fascist, religious-political, libertarian and other country-specific traditions, including mixed or evolving positions. Labels are data-driven families, not universal stat bonuses.
- Separate leader, party, faction, public and official-state ideology; distinguish declared doctrine from implemented policy. Preferences influence economic organization, civil rights, religion, political participation, national identity and foreign policy independently.
- Authoritarian governance includes patronage, co-option, censorship, political policing, controlled participation and opposition restrictions as abstract game choices with fiscal, informational, legitimacy and institutional consequences.
- Dictatorship is not omnipotence: elite loyalty, credible information, administrative competence, military obedience, succession and public acquiescence remain separate pressures. Include ruler/party survival challenges alongside default country-continuation mode.

**Working result:** policy consequences influence an election, and a government change preserves the country’s accounts, laws and obligations.

## Priority 12 — Baseline country AI and delegated administration

- Each government maintains goals, beliefs, constraints, risk appetite and commitments.
- First behaviors: pay obligations, maintain essential services, respond to shortages, avoid infeasible budgets and defend political viability.
- Candidate generation, shared validation, approximate outcome evaluation and utility ranking.
- Hierarchical plans with review dates, cancellation conditions, cooldowns and inertia to prevent constant reversals.
- Opposition, parties, institutions and basic firms use smaller role-specific agents.
- Player delegation by ministry with budgets, permitted actions, priorities, approval thresholds and an audit trail.
- Debug explanations show considered alternatives and rejection reasons; normal players see only explanations consistent with their knowledge.
- Extend this AI in every subsequent system. Do not postpone AI until the feature catalogue is complete.
- Regime-aware AI balances its actual survival coalition and ideology: electoral accountability, party factions, military leadership, dynastic interests or religious institutions. Do not give every country a democratic reelection objective or force convergence toward one regime.

**Working result:** the world continues plausibly without the player managing every country, and delegated policies remain understandable and reversible where rules permit.

## Priority 13 — The complete governing interface

- Situation brief organized around problems and pending decisions rather than an undifferentiated notification feed.
- Ministry screens for economy, finance, society, institutions and foreign affairs, expanding with later systems.
- Policy search by problem, action, ministry or affected group; show why unavailable choices are unavailable.
- Draft comparison with baseline, distributional effects, cost, authority, implementation lag and uncertainty.
- Before/after charts, pinned indicators, regional heatmaps and sortable international comparisons.
- Actionable alerts, deduplication, severity controls, watchlists and configurable automatic pause.
- Negotiation and legislation timelines; pending decisions clearly distinguished from completed actions.
- Guided first governing session: identify a problem, inspect causes, enact a feasible intervention, advance time, inspect consequences.
- Regime-aware political workspace: legislature/elections where relevant, ruling-party leadership, military council, court or religious hierarchy where relevant; show legal authority, effective power, faction support and succession risks separately.

**Working result:** a player can complete that loop without reading developer documentation. Priorities 01–13 establish the first coherent government game; subsequent priorities deepen the same loop.

## Priority 14 — Utilities, energy and natural resources

- Fuel deposits, reserves, extraction cost, depletion, exploration and import dependence.
- Electricity generation by technology, fuel requirements, dispatch, network capacity, reliability and seasonal demand.
- Coal, oil, gas, nuclear, hydro, wind, solar, geothermal, biomass and storage definitions; technology availability depends on date and research.
- Grid interconnection, electricity contracts, outages, maintenance, reserve margin and emergency rationing.
- Water abstraction, treatment, distribution, irrigation and scarcity.
- Energy pricing, household assistance, industrial subsidies, efficiency standards and strategic stockpiles.
- Shutdown, reopening, decommissioning and environmental liabilities.
- Regional energy/resource overlays and clear explanation of the marginal bottleneck.

## Priority 15 — Construction, transport, housing and communications

- Projects need site, permissions, design, funding, workforce, materials and time; staged completion and cancellation settlement.
- Roads, bridges, tunnels, rail, airports, ports, logistics hubs and pipelines as capacity networks.
- Congestion, travel time, freight cost, maintenance, damage and reconstruction.
- Housing supply, rent, rent regulation/indexation, overcrowding, land use, zoning, public housing, homelessness and urban growth.
- Public transport, tolls, vehicle standards, electric-vehicle infrastructure and later autonomous transport.
- Telephone, mobile, broadband, data centers, internet access and later-generation network upgrades.
- Buildings attach to the relevant economic/service system instead of merely adding flat national bonuses.

## Priority 16 — Public services, welfare and labor policy

- Healthcare capacity, staffing, access, prevention, insurance/financing, medicines and treatment demand.
- Education by level, attainment, teacher supply, vocational training, universities, research and skills matching.
- Pensions, retirement age, eligibility, disability, unemployment support, child benefits and means-tested assistance.
- Minimum wage, public wages, hours, overtime, leave, dismissal/severance, collective bargaining and workplace safety.
- Childcare, parental leave, fertility support and long-term fiscal projections.
- Universal income and alternative transfer designs funded through the actual fiscal system.
- Civil-service hiring/training, digital administration and service backlogs.
- Outcomes reported as access, quality, affordability and capacity—not only spending percentages.

## Priority 17 — Monetary policy, banks and financial crises

- Currency regimes, exchange rates, reserves, convertibility, pegs, currency unions and capital controls.
- Central-bank mandate and independence; policy rate, liquidity facilities and monetary financing where legal.
- Commercial-bank balance sheets, deposits, lending, loan losses, capital and liquidity; mortgage terms, interest resets and borrower-protection policies.
- Inflation channels: demand, inputs/imports, monetary conditions, expectations and capacity constraints.
- Deposit insurance, bank runs, liquidity support, recapitalization, resolution and sovereign-bank feedback.
- Sovereign wealth funds, asset ownership and returns; foreign-currency debt exposure.
- Currency conversion with explicit handling of existing contracts, deposits and debt denominations.
- Financial statistics reconciled to underlying accounts; no market value counted as government cash without a transaction.

## Priority 18 — Trade, investment and commercial policy

- Bilateral goods/services trade, capacity and transport costs; access to shipping lanes and chokepoints.
- Tariffs by product/partner, quotas, embargoes, licensing, customs unions and rules of origin.
- Trade agreements, market access, services access, dispute procedures and withdrawal.
- Foreign direct investment, project approval, ownership restrictions, profit repatriation and expropriation risk.
- Sanctions by finance, trade, technology, assets and named actors; exemptions, evasion and enforcement.
- Supply-chain dependencies, commodity shocks, strategic stockpiles and substitution.
- Negotiated contracts with quantity, quality, price rule, delivery, term, penalties and force-majeure conditions.
- Aid shipments and military supply use the same delivery constraints where appropriate.

## Priority 19 — Firms, ownership and industrial policy

- Aggregate ordinary enterprises; explicit strategic firms where ownership, procurement or market power matters.
- Public/private/mixed/cooperative and worker-owned enterprises, ownership conversion, nationalization, privatization, shares, dividends and liabilities.
- Market share, competition, anti-trust, subsidies, procurement, industrial strategy and research support.
- Firm investment, employment, wages, pricing, insolvency, mergers, acquisitions and foreign subsidiaries.
- Banking exposure, stock-market valuations and government financial interests.
- Corruption, lobbying, procurement fraud and regulatory capture tied to institutions and investigations.
- Tourism and cultural-sector firms reuse the same business rules.
- Corporate AI has solvency, profitability, investment and compliance constraints; it does not exist purely to satisfy government orders.

## Priority 20 — Unions, civil society, media and domestic power

- Labor unions, employer associations, farmers, landowners, religious organizations, NGOs and professional groups.
- Military, security services, business elites and regional power centers with distinct resources and institutional access.
- Membership, organization strength, funding, public trust, demands and issue priorities.
- Lobbying, petitions, bargaining, strikes, demonstrations, boycotts and negotiated agreements.
- Media ownership, public broadcasting, press freedom, audience reach, credibility and scandal exposure.
- Propaganda, disinformation, conspiracy spread, fact-checking and literacy as competing information processes.
- Public order response, escalation/de-escalation and consequences for trust, rights and economic activity.
- Leaders can make commitments, but their delivery still requires policy implementation.

## Priority 21 — Civil rights, justice and social legislation

- Citizenship, voting rights, equality and discrimination, privacy, surveillance and freedom of association.
- Speech, press, internet access, religious practice and separation of religion/state.
- Family/marriage/adoption rules, reproductive healthcare, surrogacy, gender recognition and end-of-life policy.
- Alcohol, tobacco, cannabis, other controlled substances, gambling and sex-work regulation.
- Child protection, child labor, school attendance and age-based legal rules.
- Police staffing/training/oversight, courts, prosecution, legal aid, prisons, sentencing and rehabilitation.
- Criminal responsibility, capital/corporal punishment, facial recognition and social-credit-style systems as policy options with modeled institutional and social consequences.
- Differentiate legality, enforcement, actual prevalence and reporting quality. Avoid making ideology determine outcomes by hidden fiat.

## Priority 22 — Migration, citizenship and integration

- Internal migration and international flows by origin/destination and reason.
- Economic, education, family, asylum and humanitarian routes; temporary versus permanent status.
- Visas, residency, work permission, naturalization, birthright citizenship and dual-nationality rules.
- Border capacity, irregular migration, trafficking, detention, regularization, returns and bilateral transit agreements.
- Refugees, displacement, resettlement, reception capacity and integration services.
- Labor-market matching, housing/service pressure, remittances and diaspora links.
- Migration preferences respond to safety, jobs, family networks and legal routes; it is not a single country attractiveness score.

## Priority 23 — Diplomacy and binding agreements

- Bilateral relations decomposed into trust, threat, disputes, dependency, ideological affinity and public opinion.
- Recognition, embassies, diplomatic rank, meetings, state visits and negotiation agendas.
- Treaties represented by clauses, signatories, ratification, duration, obligations, monitoring and remedies.
- Aid, loans, guarantees, access, bases, intelligence sharing, defense commitments and research cooperation.
- AI proposal generation, counteroffers, reservation values and acceptance reasons.
- Credibility, reputation and domestic approval constraints; secret terms stored in restricted information views.
- Summits and multilateral bargaining use the same proposal/commitment framework.
- Foreign influence emerges from concrete dependencies and relationships; no universal percentage that automatically buys another country.

## Priority 24 — International organizations, unions and blocs

- Generic organization engine: charter, members/observers, accession, suspension, withdrawal, offices, councils, voting, budget and mandate.
- UN General Assembly/Security Council, vetoes, rotating seats, recognition/admission, resolutions, sanctions and peacekeeping authorization.
- IMF/World Bank-style lending, conditions and development projects; WTO-style trade commitments and disputes.
- NATO-style defense and consultation, military cooperation and accession.
- EU institutions, legislation, budget, cohesion funding, accession, common market, customs, monetary arrangements and integration/withdrawal paths.
- Regional and economic organizations: ASEAN, OAS, Arab League, OAU/AU transitions, CIS, Mercosur, regional development banks and trade areas.
- Producer coordination, G7/G20-style summits, non-aligned cooperation, later BRICS-style institutions and custom organizations.
- Health, education, labor, children, culture and humanitarian agencies linked to their actual simulation domains.
- All membership and existence dates come from the bookmark. Later organizations can emerge through rules or scenario conditions.

## Priority 25 — Military establishment, procurement and logistics

- Service branches, formations, personnel, recruitment, reserves, conscription, training, morale and readiness.
- Equipment stocks, age, condition, maintenance, ammunition, fuel and spare parts.
- Domestic/foreign procurement, production lines, industrial capacity, delivery schedules, upgrades and export restrictions.
- Private military companies and contracted forces: payment, mission limits, command authority, accountability, reliability and termination.
- Bases, ports and airfields with capacity, access agreements, rent, supply links and notice periods.
- Doctrine, command structure, loyalty, civilian oversight and mobilization.
- Strategic logistics graph; supply and deployment constrained by routes, lift, access and infrastructure.
- Land, air, sea, missile and support roles; unit design only as detailed as decisions warrant.
- Military expenditure reconciles with treasury and industry; nominal unit count does not equal usable strength.

## Priority 26 — Conventional warfare and operational AI

- War participants, goals, escalation, declarations, intervention and rules of engagement.
- Operational areas, fronts, territorial control, movement and supply; map geometry remains independent of state ownership.
- Combat aggregates by forces and objectives, with terrain, weather, reconnaissance, readiness and logistics.
- Land maneuver, air missions/interception, naval movement/blockade, amphibious operations and long-range strikes.
- Fortifications, trenches, obstacles, mines, air/missile defense and electronic warfare.
- Drones, unmanned surface/subsurface/ground systems, AWACS, jamming and swarms gated by technology and industrial capability.
- Casualties, displaced civilians, infrastructure damage, war expenditure, public support and war exhaustion.
- Orders to military staff with objectives and constraints; manual interventions coexist with operational delegation.
- Tactical city encounters and street fighting remain an optional later presentation of these rules, not a separate incompatible simulation.

## Priority 27 — Peace, occupation and territorial transitions

- Ceasefires, armistices, peace conferences and third-party mediation.
- Negotiated territory, demilitarization, reparations, resource rights, neutrality, guarantees, prisoner exchange and force limits.
- Occupation administration, resistance, civilian services, reconstruction and withdrawal.
- Annexation, independence, federation, confederation, merger, partition, autonomy and referendums.
- Atomic transfer rules for people, territory, debt, cash, facilities, military assets, contracts and institutions.
- Legality, control and recognition tracked separately; unresolved claims survive a map recolor.
- Enforcement, monitoring, violations and renewed conflict require explicit conditions.
- AI accepts limited settlements when appropriate instead of requiring total conquest.

## Priority 28 — Intelligence, covert action and cyber

- Agencies, budgets, staff, foreign networks, collection priorities and counterintelligence.
- Estimates with confidence, age, source quality and possible deception; fog of war applies to economics and politics too.
- Reconnaissance, industrial/technical espionage, influence operations, sabotage and support to foreign factions.
- Operations require preparation, access, resources and time, with distinct success, detection and attribution.
- Cyber capability, infrastructure exposure, defensive investment, persistent access, incident response and recovery.
- Economic, electoral, communications, logistics and military disruption expressed as high-level game effects.
- Diplomatic, domestic and escalation consequences of exposure.
- Omniscient debugging is a separate tool; a normal player or AI should not see hidden operation truth.

## Priority 29 — Organized crime, insurgency and internal conflict

- Illicit markets, demand, production, routes, laundering, corruption and state capacity.
- Criminal networks/cartels with finances, territorial reach, resilience and relationships.
- Customs, financial investigation, specialist policing, prevention, treatment, prisons, extradition and informant systems.
- Terrorist threats and hostage crises abstracted through intelligence, preparedness and response choices.
- Rebel organization, grievances, recruitment, funding, territorial control and external support.
- Coups, defections, civil war, negotiations, demobilization and reintegration.
- Emergency measures and repression have effects on legitimacy, organization, reporting and longer-term stability.
- No automatic conversion from low approval to armed rebellion; escalation needs modeled organization and conditions.

## Priority 30 — Climate, ecology, agriculture and disasters

- Emissions by sector/fuel, cumulative climate drivers, regional exposure and uncertainty.
- Environmental regulation, carbon pricing, standards, subsidies and international climate commitments.
- Adaptation, coastal protection, water management and disaster preparedness.
- Drought, flood, wildfire, storms, earthquakes and other hazards with geographically appropriate exposure.
- Agriculture, yields, irrigation, fertilizer, soil, land use, fisheries, food security and strategic food stocks.
- Forests, deforestation, habitat, biodiversity, pollution, waste, recycling, plastics and ocean impacts.
- Animal welfare: farming standards, hunting, wildlife protection, testing and entertainment rules.
- Migration, health, fiscal and supply consequences feed the existing systems; disaster relief uses real capacity.
- Climate negotiations, common funds and compliance reuse organization/treaty machinery.

## Priority 31 — Epidemics, public health and demographic change

- Disease spread at cohort/region level, contact patterns, severity, immunity and healthcare capacity.
- Testing, surveillance, masks, vaccines, treatments, hospital capacity and risk communication.
- Targeted restrictions, schools/workplaces/events, travel and border measures with economic/social consequences.
- Vaccine research, manufacturing, procurement and distribution; international cooperation.
- Chronic illness, prevention, nutrition, environmental exposure and causes of mortality.
- Aging, fertility, longevity and projections of health, education, pensions and labor supply.
- Preparedness and shock scenarios remain configurable; historical outbreaks are not inevitable on the same date after history diverges.
- Optional deliberately caused biological-threat scenarios at abstract crisis/response level, with detection uncertainty and international consequences; no operational biological detail.

## Priority 32 — Research, technology and structural change

- Technology graph with prerequisites, knowledge, institutions, skilled labor, funding and physical capability.
- Public research, universities, firms, international projects, technology transfer and diffusion.
- Distinguish discovery, prototype, commercialization, adoption and productive deployment.
- Automation/robotics, AI adoption, labor displacement/retraining, productivity and distributional consequences.
- Energy/storage, medicine, agriculture, materials, communications, transport and defense technologies.
- Standards, patents/licensing, export controls and dependency on critical inputs.
- Long-horizon or speculative technologies flagged separately and enabled by scenario rules.
- Future availability responds to research and economic conditions rather than a hard-coded calendar unlock alone.

## Priority 33 — Nuclear strategy, arms control and strategic deterrence

- Civil nuclear infrastructure, fuel supply and separate strategic-program status.
- Delivery-system inventories, readiness, detection, warning, doctrines and command authority.
- Deterrence, extended guarantees, escalation perceptions and crisis communication.
- Arms-control treaties, inspection, limits, verification, withdrawal and disarmament.
- Proliferation, sanctions, diplomacy and domestic political constraints.
- Strategic-use consequences include damage, humanitarian crises, environmental effects and international response.
- Keep all of this at strategic game abstraction; no engineering of real weapons is necessary for the mechanic.

## Priority 34 — Space and advanced strategic infrastructure

- Space agencies, launch infrastructure, launch vehicles, budgets and procurement.
- Communications, positioning, weather and reconnaissance satellites with coverage, maintenance and lifetime.
- Launch failure, constellation degradation, debris, anti-satellite conflict and replacement capacity.
- International science missions and prestige grounded in research and resource costs.
- Lunar activity, space resources, Mars and advanced orbital projects as optional future-era content.
- Settlement logistics, supplies, administration, political status and autonomy if off-world settlements are enabled.
- Reuse projects, trade, logistics, science and territorial-administration primitives rather than inventing an unrelated minigame.

## Priority 35 — Tourism, culture, sport and soft power

- Visitor flows, spending, accommodation, transport access, seasonality and capacity.
- Heritage, parks, conservation, safety, sanitation and environmental carrying capacity.
- Culture, creative industries, language/cultural exchange and public funding.
- Sporting organizations, bids, hosting, infrastructure, security, costs and reputational outcomes.
- Cultural/sporting boycotts, international exchange and tourism diplomacy.
- Soft power decomposed into attraction, access, credibility and networks, with limited and explainable diplomatic effects.
- These features use the production, mobility, construction and event engines rather than arbitrary cash rewards.

## Priority 36 — Historical world completion and differentiated countries

- Complete the eligible-country and territory register for the 2000 bookmark; every omission, special status or unplayable entity explicitly recorded.
- Validate office-holders, major parties, cabinets, institutions, elections, currencies, treaties and organization memberships against that date.
- Curate subnational geography, population, industries, resources, infrastructure and armed forces to the selected simulation granularity.
- Add country-specific institutional configurations, political movements and strategic circumstances.
- Preserve universal system support: distinct country content should add context and constraints, not be required to make basic government work.
- Country selection displays data completeness, difficulty context and special rules. Missing facts remain labeled estimates, never invented certainty.
- Historical names/facts coexist with generated future personalities and alternate outcomes.

This is completion of the historical content layer introduced at Priority 02, not a reason to defer historical validation until the end.

## Priority 37 — Scenarios, bookmarks and campaign tools

- Default 2000 sandbox with configurable historical pressure versus open divergence.
- Additional independently validated bookmarks; changing the start date is a dataset change, not just changing the clock.
- Scenario definitions: setup overlays, objectives, constraints, actors, triggers, events, failure/success conditions and scoring.
- Economic scenarios: recession, banking crisis, sovereign debt, inflation, industrialization, energy dependence, household living costs and trade disruption.
- Political scenarios: election, coalition, constitutional reform, democratic transition, federalism, reunification and institutional breakdown.
- Security scenarios: regional war, territorial dispute, peace negotiation, insurgency, nuclear crisis and maritime disruption.
- Social scenarios: health emergency, migration, education, crime, inequality and demographic transition.
- Environmental scenarios: decarbonization, disaster recovery, drought/food security and climate cooperation.
- Scientific/cultural scenarios: technological leadership, international infrastructure, tourism and major events.
- Optional speculative scenarios: future crises, altered institutions and off-world politics; explicitly separate from historical claims.
- Trigger historical-inspired events by conditions where possible. Historical mode can weight choices without guaranteeing the original outcome.
- Campaign chronicle, meaningful comparisons, custom objectives and optional post-defeat observation/continuation.

## Priority 38 — Editors, modding and scenario inspection

- Content editing for countries, borders/control, actors, laws, organizations, economy, technology and scenario setup.
- Conditional events with schema validation, references, clear error locations and a deterministic execution budget.
- Custom names, flags, portraits, colors, icons, localization and map styles.
- Simulation mods with explicit dependencies, load order, overrides and compatibility declarations.
- Sandbox inspector with controlled mutations, instant events and reproducible scenario fixtures.
- Mod content hashes in saves/replays; safe handling of missing and conflicting packs.
- Begin with declarative content. Add sandboxed scripting only where declarative rules cannot express the needed behavior.
- Local import/export first; optional Workshop distribution later, without making Steam mandatory for normal mod use.

## Priority 39 — Full AI depth and long-campaign behavior

- Strategic doctrine changes after elections, regime changes, threats and major economic shifts.
- Negotiation across linked issues, coalition bargaining, deception under incomplete information and treaty compliance decisions.
- Multi-year investment and infrastructure planning; scenario-aware crisis responses.
- Military plans coordinated with political war aims, logistics and feasible peace terms.
- Distinct personalities without irrational randomness; institutional checks can override a leader’s preference.
- AI budgets expressed in bounded candidate evaluations; relevant-country detail can increase without breaking basic world obligations.
- Automated campaigns compare outcomes and detect loops, stagnation, universal bankruptcies, endless wars or unrealistic convergence.
- Difficulty changes planning quality, uncertainty or resources only through declared settings; no undisclosed access to hidden world truth.

## Priority 40 — Presentation, learning and accessibility polish

- Consistent map language, icons, flags, charts, typography, spacing, audio cues and color usage.
- Rich tooltips and encyclopedia entries linked to live state and formulas.
- Visual explanation of flows: money, trade, electricity, logistics, migration and political support.
- Configurable dashboards, comparisons, saved filters, bookmarks, layout persistence and multiple-display behavior where supported.
- Interactive tutorials for budget, lawmaking, diplomacy, logistics and crisis response.
- Keyboard-only completion of core tasks, accessible focus order, text alternatives, scalable interface and non-color cues.
- Localization, bidirectional/complex-script assessment, readable large-number formatting and unambiguous dates/units.
- Actual accessibility and platform issues are addressed as encountered from Priority 04; this priority completes consistency across all systems.

## Priority 41 — Multiplayer product implementation

- Headless authoritative server using the existing simulation and command validators.
- Hosted sessions and dedicated servers; version/content/mod agreement before joining.
- Country assignment, AI takeover, reconnect, save/resume, spectators and optional cooperative government roles.
- Per-player filtered snapshots/deltas; hidden intelligence never sent to unauthorized clients.
- Ordered/idempotent commands, authentication, rate limits, validation and rejection feedback.
- Shared pause/speed policy, readiness, handling of disconnected players and simultaneous negotiations.
- Transport, discovery, relay/NAT support and optional Steam integration selected against actual platform needs.
- Desync diagnostics and replay support; server authority removes the need for every client to simulate all hidden world state.
- Multiplayer-specific balance and exploit checks on gifts, loans, collusion, trade loops and repeated commands.

The protocol boundary, role permissions and information separation are built in Priority 01; only the networked product is deferred.

## Priority 42 — Optional player careers and extra presentation modes

- Opposition-party leadership, electoral climb and government transition using the existing political system.
- Corporate leadership using existing firms, production, finance and regulation.
- Personal wealth, property, donations, influence and scandals when relevant to an explicit role.
- Multi-country single-player control, observer mode and custom role permissions.
- Optional tactical/city views, ceremonial events and geography challenges.
- Keep the default government game complete without requiring these additional player perspectives.

## Rules that apply at every priority

1. **A feature is a connected loop:** action → validation → cost/commitment → implementation → world effect → player explanation → AI response.
2. **Build the AI behavior and UI alongside the rule.** A hidden subsystem with no decisions or usable feedback is not complete.
3. **Verify the relevant invariants immediately.** Accounting, save/reload, temporal references, deterministic results and permissions cannot be left until the catalogue is finished.
4. **Keep world data separate from formulas.** Imported historical observations are not gameplay causal coefficients.
5. **Keep original implementation and assets.** The reference inventories establish subjects to cover; source code, proprietary databases, writing and art are not the implementation material.
6. **Differentiate observed facts, estimates, simulation output and forecasts.** The player should be able to tell which number they are seeing.
7. **No new subsystem can bypass the core constraints.** A space project, climate fund, covert action or sporting event must pay costs and use real capacity just as an ordinary policy does.

## Recommended technical shape, in one page

| Layer | Build choice |
|---|---|
| Simulation | Pure Rust library, typed indexed state, explicit phases, deterministic random streams; runs headless |
| Interface | egui/eframe with wgpu; production-styled map/table/policy prototype validates suitability |
| Map | Offline geographic compilation, shared topology, triangulated LOD meshes, GPU overlays and separate picking |
| Storage | Narrow adapter; bundled SQLite/rusqlite baseline, Turso (formerly Limbo) evaluated with the same save/content workload |
| Runtime state | Typed in-memory stores; SQL for durable data and tools, not per-entity tick queries |
| AI | Utility scoring + hierarchical plans + shared action validation + knowledge limits + decision traces |
| Data | Versioned historical packs with provenance, validity dates, licenses and missing-data policy |
| Modding | Declarative definitions and deterministic events first; sandboxed scripts only when needed |
| Multiplayer | Local authority now; authoritative host/server and filtered replication later |
| Verification | Headless replay, accounting/property tests, transition fixtures, long campaigns and real cross-platform runs |

These are recommendations, not measured performance results. [The technical overview](technical-research.md) gives alternatives, current upstream evidence and the conditions under which to change a choice.
