# Comparator research: Geoplanetical and Millennium Dawn

Research snapshot: 2 November 2026. Primary developer documentation and Steam announcements. Features below are documented claims, not independently play-tested. Documentation can lead or lag builds. The inventories are compact paraphrases, not copied manuals; source links are provided for detailed verification.

## Geoplanetical

### Release and scope

The exact title is **Geoplanetical**, by Polytale Games. Steam records Early Access release on 28 October 2026. Its campaign begins 1 January 2026, offers government or opposition play across 200+ countries/territories, and runs without a fixed ending. The developer estimates 18–24 months in Early Access, explicitly subject to change. Single-player, Steam Cloud and Workshop are advertised. The advertised simulation includes roughly thirty economic sectors, 100+ goods, 100+ institutional choices, and 200+ technologies in nineteen fields. These counts describe this comparator, not recommended scope commitments for the new game. [Steam store](https://store.steampowered.com/app/940230/Geoplanetical/)

### Published gameplay inventory

The developer's gameplay catalogue covers:

- Institutions: legislative procedure, vetoes, courts, chamber design, electoral rules, succession.
- Politics: campaigns, activists, promises, coalitions, opposition, confidence votes.
- Society: migration, asylum, diaspora, social groups, unions, strikes, housing, protests.
- Fiscal/financial: tax bands, contributions, VAT, other taxes, debt, currency regimes, central banks, sovereign funds, bank rescues, ownership.
- Production: investment, employment, transport, tourism, culture, sport, extraction, reserves, electricity, decarbonisation.
- Services: healthcare, education, policing, courts, prisons, administration.
- Risks: epidemics, vaccines, disasters, warming, adaptation.
- Foreign affairs: trade, sanctions, treaties, organisations, crises, intelligence, hostage incidents, coups.
- Defence: procurement, bases, supplies, military loyalty, operational directives, amphibious operations, occupation, peace.
- Strategic systems: nuclear programmes, deterrence, technology acquisition/diffusion, territorial negotiations, lunar settlement.

Important boundaries: leaders are fictional; parties are generated/anonymised. Smallest microstates are visible but unplayable. Historical starts and scripted scenarios are absent. Mods change presentation, not simulation rules. These constraints make a genuinely historical 2000 database and simulation modding clear opportunities. [Official gameplay and FAQ](https://polytale.games/geoplanetical)

### Interaction details worth studying

The official manual distinguishes a draft from an enacted action: a budget preview must be applied; a bill may require assent after parliamentary passage; a negotiated union promise still needs implementing. Elections for chambers and executive offices have separate calendars. Coalition membership does not guarantee legislative support. Opposition parties with seats retain limited legislative power. Financial rescue distinguishes capital inadequacy from liquidity shortages. A currency change preserves existing debt denomination. Foreign bases require agreements before construction; agreements have rents and notice periods. Allies can respond with combat, support, access, or refusal. Covert operations have preparation, detection and attribution; nuclear decisions have doctrine and response settings. Lunar operations include supplies, administration and political independence. [Official manual](https://polytale.games/geoplanetical-wiki)

**Design implication:** represent proposals, approvals, funding, construction, delivery, enforcement and outcomes as separate states. Every interface must distinguish “promised”, “approved”, “funded”, “operational” and “effective”. This is our proposed architecture, not a claim that the comparator implements it perfectly.

### Latest delivered changes

The live Steam feed documents:

| Date | Documented addition or revision |
|---|---|
| 30 October 2026 | Construction/finance rework; 753 deposits covering seventeen resources and 128 countries; energy interface revisions. |
| 31 October 2026 | Legislative, economic, naval logistics and readability corrections. |
| 1 November 2026, 1.0.5 | Migration origins/destinations, refugee routes/resettlement, individual-product tariffs. |
| 1 November 2026, 1.0.6 | Military production projects, equipment modernisation, domestic/foreign/surplus purchasing, sales restrictions, replenishment sources, contract tracking. |
| 1 November 2026, 1.0.7 | Peace summits, conditional sanctions relief, AI sanctions reconsideration, NATO applications/votes, clearer negotiations. |
| 2 November 2026 | Troop transfers to domestic/allied bases, grouping, recalls, automatic-control handoff and map-based management. |

The same feed records save-load, financial-ledger, annexation and production bugs. This supports regression scenarios around cross-system transitions; it does not establish overall software quality. [Steam announcements](https://steamcommunity.com/app/940230/allnews/)

### Official future roadmap — not shipped promises

The roadmap has no ordered schedule or feature release dates. It lists cyberattacks; telecom/internet infrastructure; advanced trade; additional laws; deeper EU politics; BRICS; more organisations; controversial actions; rebels; improved leader decisions; clearer UI; future technologies; expanded space mining; Mars; biological attacks; campaign summaries. Continuing work includes stability, country AI, negotiation and warfare. [Official roadmap](https://polytale.games/roadmap)

**Interpretation:** broad roadmap labels can overlap delivered systems. For example, product tariffs now exist while “advanced trade” remains a future heading. Treat each as a capability family with completed and pending subfeatures, not a binary checkbox.

## Millennium Dawn: A Modern Day Mod

### Version and evidence boundary

The official documentation currently identifies expected version **2.0.2** and checksum `ccc6`; its wiki is explicitly work in progress. Roadmap maps distinguish existing country focus trees, shared trees, planned country content, and military-industrial organisations. Tentative country content may move between patches. [Official home](https://millenniumdawn.github.io/Millennium-Dawn/)

The 2.0 changelog introduces UN governance, international aid/recognition, Antarctica, resource storage and expanded economics; extends technology to the 2080s and adds 50+ special projects; adds or updates national content and military-industrial organisations; includes Event Horizon and revised zombie scenarios. It also adds configurable satellite-programme automation, conditional-peace safeguards, treaty/embargo interactions and energy load sharing. [2.0 changelog](https://millenniumdawn.github.io/Millennium-Dawn/changelogs/v2-0-changes/)

**Caution:** these are mechanics within HOI4's engine and sometimes DLC-dependent. They are inspiration for independently designed systems, not modules that can simply be ported to Rust.

### Economic capability inventory

Documented domains: GDP and per-capita output; corporate/population taxation; resource-export revenue and changing prices; seigniorage; military, administrative, security, educational, health and welfare expenditure; debt and interest; reserve currencies, currency strength/backing, monetary policy and inflation; employment, productivity, economic cycles; trade/economic laws; privatisation and mining policy; literacy; sanctions; electricity; offices, networks, agriculture, energy/industrial infrastructure; semiconductor/composite plants and input shortages; immigration; domestic/international investment; agriculture allocation and drought; IMF/African investment financing and bailouts. [Economy guide](https://millenniumdawn.github.io/Millennium-Dawn/player-tutorials/economy-guide/)

**Recommended adaptation:** replace HOI-style factory abstraction with auditable sector accounts and physical bottlenecks. Preserve the interesting decisions—budget tradeoffs, critical inputs, monetary constraints—while exposing how each number was calculated. Do not reproduce the source's numerical tuning as economic truth.

### Politics and domestic interest groups

The politics guide documents five ideology groupings, twenty-four subideologies, multiparty elections, popularity, coalitions, party promotion/attacks, bans/legalisation, ideological drift, reforms, coups/civil wars, retirement and term limits. [Politics guide](https://millenniumdawn.github.io/Millennium-Dawn/player-tutorials/politics-guide/)

The interest-group system includes businesses, banks, fossil-fuel firms, industrial groups, oligarchs, shipping, defence industry, armed forces, intelligence services, unions, landowners, farmers, communist cadres and religious organisations. National variants include North Korean entrepreneurs, the Saudi royal family, Quds Force, foreign jihadists, Wall Street and chaebols. Opinion changes affect bonuses; the game uses three selectable faction slots. [Internal factions guide](https://millenniumdawn.github.io/Millennium-Dawn/player-tutorials/internal-factions/)

**Recommended adaptation:** model all materially relevant groups concurrently, with membership overlap, resources, organisational strength, policy interests and institutional access. Avoid importing ideology labels as universal classifications: represent economic preferences, social preferences, constitutional commitments and foreign alignment separately. Country-specific political structures should derive from content data rather than hard-coded national exceptions.

### Diplomatic influence and investment

Influence is divided between domestic independence and foreign shares. Manual and automated influence can enable aid, political intervention, coups, satellite relationships and extraction from subjects. Defensive actions contest influence. Power rankings affect projection and resistance; subject statuses vary autonomy and obligations. [Influence guide](https://millenniumdawn.github.io/Millennium-Dawn/player-tutorials/influence-guide/)

Investment distinguishes foreign construction projects with host acceptance, income and influence from domestic regional improvements. Project count, duration, cancellation and acceptance matter. The documentation includes AI investor eligibility, target selection, building/state scoring and host response. [Investment guide](https://millenniumdawn.github.io/Millennium-Dawn/player-tutorials/investments-guide/)

**Recommended adaptation:** do not let a single influence percentage automatically purchase sovereignty. Derive bargaining power from debt ownership, investment exposure, trade dependency, security dependence, elite ties and institutional legitimacy. Each should have a separate causal effect and countermeasure.

### International institutions

General international coverage includes NATO and major non-NATO ally status, F-35 collaboration, African Union membership, counterterrorism, financial institutions, monetary policy, sanctions and private military companies. [International systems guide](https://millenniumdawn.github.io/Millennium-Dawn/player-tutorials/international-systems/)

The UN has bilateral recognition, membership admission, General Assembly and Security Council voting, elected seats, vetoes, lobbying, budgets and aid. Recognition changes access to institutions and diplomatic standing. [UN guide](https://millenniumdawn.github.io/Millennium-Dawn/player-tutorials/united-nations/)

EU coverage includes eligibility/accession/pre-accession support; euroscepticism; influence and offices; shared legislation; Parliament and Council; euro/ECB/fiscal integration; Article 50; military cooperation; federal and confederal integration paths. Member states remain the principal player actors until particular unification outcomes. [EU guide](https://millenniumdawn.github.io/Millennium-Dawn/player-tutorials/eu-tutorial/)

**Recommended adaptation:** create one organisation framework with memberships, accession criteria, councils, seat allocation, voting rules, budgets, mandates, enforcement and treaty amendments. Add institution-specific rules as declarative content. For a 2000 start, memberships and even organisation existence must be date-effective; never initialise 2026 memberships in 2000.

### Military, cyber, space and peace

Military mechanics include modular armoured vehicles; APC/IFV/tank/artillery/rocket/air-defence roles; specialised aircraft including AWACS, transports, drones and maritime patrol; modern surface/subsurface hulls; cruise/ballistic/intercontinental/air-defence missiles; silos; production; formation design; support units; helicopters; special forces; doctrines. [Military guide](https://millenniumdawn.github.io/Millennium-Dawn/player-tutorials/military-tutorial/)

Cyber systems distinguish capabilities, offensive power, defence, attribution, target slots, operation duration, success and detection. Operations cover positioning interference, economic disruption, propaganda, infrastructure, surveillance, radar deception, elections, espionage, communications, finance, logistics and persistent access. Defenders can harden systems, search for intrusions and investigate attribution. **Documentation inconsistency:** its capability table says 0–5 while later tiers require values above seven and eleven. Do not treat its thresholds as validated rules. [Cyberwarfare guide](https://millenniumdawn.github.io/Millennium-Dawn/player-tutorials/cyberwarfare/)

Space systems cover launch programmes, navigation/communications/reconnaissance satellites, anti-satellite weapons, launch vehicles, reusable launchers, production time, coverage, constellation tiers, augmentation, degradation and civil/military effects; advanced projects include orbital weapons, solar generation and fire support. [Space guide](https://millenniumdawn.github.io/Millennium-Dawn/player-tutorials/space-systems-guide/)

Conditional peace supports reciprocal concessions: territorial transfers, subject creation, demilitarisation, liberation, resource rights, reparations, neutrality, regime change and military access. AI evaluates an offer and war contribution; a ceasefire differs from a final settlement. [Peace guide](https://millenniumdawn.github.io/Millennium-Dawn/player-tutorials/conditional-peace-deals/)

**Recommended adaptation:** build a proposal evaluator shared by AI, UI previews and treaty validation. Store exact clauses and obligations rather than an opaque diplomacy score. Military simulation should generate bargaining positions, material losses and war exhaustion, not require every war to end in total conquest.

## Research-derived implementation priorities

These are original planning recommendations, not additional claims about either comparator:

1. Build a historical content compiler. Political actors, offices, laws, borders, alliances and statistics require validity dates, provenance, uncertainty and fallback behaviour.
2. Make policy effects delayed and capacity-constrained: enactment cannot instantly produce staff, beds, factories, trained formations or electricity.
3. Expose causal accounting. Every major statistic needs components, units, time basis, recent changes and forward estimates with uncertainty.
4. Formalise commitments. Election promises, coalition agreements, treaties, loans, procurement, bases and peace terms share deadlines, conditions, breaches and remedies.
5. Give AI the same command and observation interfaces as players. It must budget commitments, respect actual production capacity and explain rejection reasons.
6. Treat multi-country transitions as first-class regression tests: annexation, independence, succession, currency change, bankruptcy, default, treaty expiry and player-country switching.
7. Separate scenario facts from simulation rules. The new game should support 2000, later bookmarks and alternate scenarios without recompilation.
8. Use delegated automation with explicit budgets and priorities. Military staff, industrial agencies and space programmes need reviewable plans and manual override.
9. Preserve defeated players through opposition, exile, successor governments or observer modes where scenario rules permit.
10. Build multiplayer readiness through deterministic commands and authority boundaries before adding a network transport.

## Coverage limitations

This review reads the official published catalogues, current announcements and major Millennium Dawn subsystem guides. It is not a claim that every source-code rule, every country event, every historical changelog line or every in-game parameter has been enumerated. A literal parity audit requires a version-pinned executable/mod checkout, a structured feature register and hands-on verification. Roadmaps are intentions, not guarantees.
