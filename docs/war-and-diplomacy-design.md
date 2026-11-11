# War, diplomacy and territorial change

Design baseline: **11 November 2026**. This is an original proposed game design, not implemented behavior or a claim about historical law. The [implementation register](implementation-register.csv) is authoritative for status; priorities 23–27 remain proposed. Existing domestic territorial charters do not implement sovereign subjects, warfare or international transfers. Historical examples such as the Alaska purchase describe a category of agreement; no historical scenario or price calibration is claimed here.

## Player outcomes and scope

The player must be able to negotiate, deter, coerce, ally, fight for limited objectives, settle, occupy, release territory, establish subjects and buy or sell territory. These actions use institutions, budgets, geography and other actors' decisions. A government's acceptance does not imply popular consent, legal recognition or effective control. Each is recorded separately.

The default campaign starts in 2000. Treaties and subject arrangements are date-effective content, with unknown historical facts marked unknown. New countries can emerge during play. Older bookmarks may author different institutions and relationship presets without changing the underlying agreement system. Tactical city presentation remains optional; strategic battles and their consequences are required. Nuclear escalation connects to P33 and is not silently approximated as ordinary combat.

## Shared state and technical boundaries

`sim-core` owns typed identities for agreements, clauses, proposals, crises, wars, participants, formations, battles, territorial units, claims and subject relationships. Geography has stable identities independent of the state currently administering it. Store legal title, effective control, administration and each actor's recognition separately, with dated changes and provenance. Existing reference-map polygons must not silently become authoritative sovereignty data.

Treaties hold signatories, scoped beneficiaries, clause dependencies, authority requirements, signatures, ratifications, effective dates, expiry, notice periods, monitoring and remedies. Clause types cover commitments and transfers; their costs and events link to ordinary accounts and the Chronicle. Subject relationships reference treaties and delegated powers. Wars retain participants, sides, objectives, entry/exit dates and settlement scope. Ownership transfers operate on explicit territorial units, not display colours or country-name strings.

All orders use shared player/AI validation, previews and recorded outcomes. Previews expose legal authority, cost, reservations, decision dates, affected actors and uncertainty. Accepted proposals do not immediately execute future clauses. Deterministic daily phases process effective agreements and transfers before movement/combat, then occupation and economic consequences; new combat outcomes can inform the next decision boundary. Stable-ID ordering resolves simultaneous execution; incompatible transactions fail revalidation without partial mutation. Version the phase contract when implemented and protect existing content/save/replay behavior.

Foreign estimates use observation dates, ranges and confidence. Secret clauses are visible only to authorized participants or through modeled disclosure; developer inspection remains separately flagged. Headless and desktop consume the same projections. Save/load must retain negotiations, notice clocks, wars, occupation, payment schedules, subject obligations and disputed claims.

## Diplomacy, alliances and pacts

Relations expose trust, perceived threat, interests, disputes, dependencies, ideology and public opinion. Reputation follows observable promises and violations; no single influence score purchases another state's obedience.

A proposal proceeds through draft, offer, counteroffer, signature, domestic ratification, scheduled activation and operation. It can be rejected, withdrawn or expire before activation. Amendments create a version requiring affected parties' consent. Government succession rechecks pending authority; state obligations survive unless a clause explicitly makes them government-specific. Termination, suspension and breach are distinct recorded outcomes.

| Agreement | Required behavior |
|---|---|
| Non-aggression | Named parties and scope, duration, notice, prohibited direct attacks and explicitly authored proxy support rules. Breach permits reactions; it never makes an attack physically impossible. |
| Defensive alliance | Defined attack triggers, eligible theaters, exclusions, consultation, request deadline and assistance obligations. An offensive war does not automatically trigger defense. |
| Mutual or offensive pact | Explicit joint objectives and participation commitments; each entry still checks authority and capabilities. |
| Guarantee | Named protected actor/territory and qualifying aggression; guarantor decides how to honor or breach its promise. |
| Neutrality | Scope, transit/basing restrictions and exceptions, separate from recognition or friendship. |
| Access, bases and intelligence | Routes/facilities, capacity, fees, permissions, information scope, expiry and withdrawal/evacuation terms. |
| Economic or research pact | Actual funded deliveries, sanctions exceptions and downstream production constraints. |

Calls to arms allow honor, limited assistance where permitted, refusal or renegotiation. Record timeliness and delivered assistance rather than treating acceptance as fulfillment. Conflicting commitments produce visible choices and breaches; they cannot recursively force every ally into every war. Multilateral entry, exit and voting use P24 organization rules. Membership alone does not grant unlimited military command.

## Coercion and crisis bargaining

A demand names the required act, target, evidence/justification, deadline and threatened response. Options include concessions, payments, access, neutrality, release of territory or subjects, and treaty changes. Available pressure includes diplomatic isolation, sanctions, mobilization and demonstrations of force; blockades and attacks use explicit escalation rules rather than being free diplomatic actions.

Targets may comply, partially concede, counteroffer, seek mediation/guarantees, mobilize or refuse. Issuers may accept, extend by agreement, withdraw or authorize a threatened response. Deadline expiry creates a decision/event, not an automatic unapproved declaration of war. Standing conditional authorization must be explicit, bounded and revocable before execution. Guarantees and allied consultations can widen the crisis with attributable entry decisions.

Credibility depends on available forces, routes, finances, commitments, domestic backing and observed past conduct. Failed threats have contextual reputation and domestic costs; they do not create compulsory war. Coerced agreements retain their origin and may face resistance or recognition disputes. AI explanations report observed incentives and uncertainty without exposing another government's secret thresholds.

## War, command and battles

Declarations name objectives, targets, authority and claimed justification. A contested or unlawful declaration may still occur where effective institutional control permits it, with separate domestic and international consequences. Objectives include defense, limited territorial claims, access, reparations, liberation, subject release or subjugation. Demands can change during war, with additional diplomatic costs and participant consent where required.

Coalitions track contributions and negotiated command rights. Separate peace cannot dispose of a non-signatory's territory or commitments. Civil wars and foreign intervention connect to P29 actor and recognition rules; belligerent status is not automatically sovereign recognition.

Players assign formations to objectives and fronts, priorities, engagement constraints and retreat conditions. Military staff handle routing and operational allocation under those orders; manual orders use the same rules. Mobilization, recruitment, procurement, maintenance and readiness consume real resources through P25. Access, transport capacity, fuel, ammunition, repairs and supply routes constrain deployable strength.

Battles resolve deterministically from forces committed, mission, terrain, weather, reconnaissance, morale, readiness, technology and supply, with seeded uncertainty where specified. Land advances, air interception/support, naval control/blockade, amphibious landings and long-range strikes have distinct prerequisites and counterplay. Air or naval superiority alone does not transfer legal title. Reserve reinforcement, retreat, surrender and encirclement must resolve explicitly; units cannot teleport or consume negative stocks.

Battle reports explain estimated versus observed forces, supply limitations, casualties, equipment losses, displacement, damage, retreat and control changes. Personnel and equipment losses reconcile with formation stocks and population accounting without double counting displaced or captured people. Infrastructure damage reduces real capacity; reconstruction requires actual resources. War costs, civilian needs, political support and exhaustion feed settlement decisions. Occupation is temporary control and creates administration obligations and resistance risks.

## Peace and enforcement

Ceasefires pause specified hostilities with defined lines, dates, monitoring and exceptions; an armistice need not resolve sovereignty. Peace proposals combine independently priced clauses into an atomic agreement: territory, reparations, demilitarization, neutrality, guarantees, force limits, resource rights, prisoners, independence or subject terms. Negotiators can preview the whole package and counteroffer.

Acceptance compares expected continuation costs and prospects against the package, domestic constraints and allies' interests. Occupation, battles, supply, exhaustion and credibility inform bargaining; no universal war-score threshold grants ownership. Defeat does not automatically authorize every demand. Limited peace and status-quo peace must be possible before total conquest.

Execution separates immediate transfers from scheduled payments and withdrawals. Reparations are obligations against actual capacity; arrears trigger specified remedies, not invented cash. Occupation continues until an agreed withdrawal or replacement administration actually takes effect. Verification reports distinguish suspected from established violations. A breach can trigger consultation, sanctions, suspension or a new authorized conflict; it does not silently undo completed territorial transfers or reset borders.

## Subjects and delegated sovereignty

A subject remains a distinct political actor. Domestic autonomous jurisdictions remain part of the parent state. Relationship presets are configurable bundles of powers, not interchangeable names or a linear annexation meter.

| Preset | Default allocation to refine through authored terms |
|---|---|
| Associated state | Domestic and most external self-government; negotiated defense or other delegated services, with an agreed exit process. |
| Protectorate | Domestic institutions retained; protector handles defense and specified external relations. |
| Tributary | Periodic funded payments or service obligations; broad domestic and diplomatic independence. |
| Client state | Constraints on alliances, basing and selected external decisions; substantial domestic administration retained. |
| Puppet state | Patron has explicit leadership intervention and stronger military/external control; local institutions, loyalty and resistance still matter. |
| Dependency | Separate administered territory with limited delegated government; citizenship, representation and fiscal obligations explicitly authored. |

Every relationship defines diplomatic freedom, war-entry rights, defense obligations, command/access rights, revenue/tribute, debt responsibility, trade/currency arrangements, appointment powers, citizenship, autonomy amendment, dispute resolution and exit. No preset automatically transfers the subject's treasury, population or army into the patron's accounts. Fiscal consolidation must avoid double counting transfers. A subject's alliance or war declaration is validated against delegated powers; its participation is not inferred solely from its patron being at war.

Creation can follow voluntary negotiation, a peace settlement or release of owned territory. Coerced creation records the coercion and resulting legitimacy/recognition disputes. Obligations require actual payments or assistance. Subjects can request autonomy, refuse demands, seek external support, negotiate independence or rebel under P29 rules. Integration requires its own lawful or contested territorial process; loyalty alone never annexes a country. Patron collapse, competing patrons, succession and independence preserve outstanding claims and obligations through explicit settlement rules. Relationship graphs reject cycles and conflicting exclusive delegations.

## Release, liberation and border changes

The player can select owned territorial units and propose an independent country or a subject. A third option changes domestic autonomy without creating a sovereign actor. Release may restore a scenario-defined identity or create a new identity with chosen public name, institutions, capital and citizenship rules. Identity collisions are rejected; eliminated country records remain historical and are not overwritten. Releasing an existing subject changes its relationship rather than duplicating a country.

The preview lists residents, facilities, resources, forces, cash, debt, contracts, public-service responsibilities and access routes affected. It requires a viable administration/capital and explicit handling of enclaves, disconnected land, transit and maritime access. Contiguity is not mandatory, but logistical consequences are real. Local approval/referendums follow applicable institutions; contested release remains visibly contested. Occupied or disputed territory cannot be treated as uncontested disposable property.

A staged transaction establishes the actor and institutions, assigns territorial rights and administration, allocates public assets/liabilities, defines citizenship and protected private contracts, installs any subject agreement, and updates map/economic views together. Validate and commit the whole transfer or leave the world unchanged. Residents do not become a treasury asset, and private property does not automatically become government property. Debt allocation needs creditor consent where required; refusal leaves the original obligation intact. Military personnel, equipment and bases have explicit allocation/evacuation terms.

Liberating foreign-held territory through war requires a settlement or a recorded contested administration. Returning land to an existing state, restoring an independent state and installing a subject are separate objectives with distinct recognition consequences. Borders, claims and effective control remain separately inspectable after each outcome.

## Purchases, exchanges and leases

Territorial purchases are peacetime cession treaties using the same transfer transaction as peace settlements. Either party can propose a sale, purchase, gift or territorial exchange. The offer identifies territorial units, title/control disputes, price and currency, payment schedule, transfer date, ratification, population rights, assets, liabilities, access and guarantees. Domestic authorization and any required local consent apply to both parties.

For a cash purchase, reserve the buyer's actual funds after required authorization; at closing revalidate title, authority and funds, then settle payment and territory atomically. Cancellation releases only unspent reservations and retains explicit sunk costs. Installments require an agreed credit contract, collateral/remedies and creditor accounting; missing installments do not silently reverse sovereignty. No installment option ships before those dependencies exist. Exchanges validate and transfer both sides together.

AI values fiscal needs, strategic access, resources, residents' preferences, domestic opposition, security, claims and alternatives. Some territories are unacceptable to sell under current conditions regardless of price; show the relevant refusal reasons. Buying a remote territory does not create a supply route or erase opposition. An Alaska-style transaction is supported by this design as a category, not a guaranteed historical event.

Leases transfer specified use/administration rights for a duration; base access and resource concessions transfer still narrower rights. They do not transfer sovereignty. Expiry, renewal, eviction disputes, improvements and evacuation follow explicit clauses. A seller cannot sell the same territory twice through concurrent pending offers; accepted reservations and closing revalidation prevent double execution.

## Interface and AI

Diplomacy shows relations, treaties, proposals, crises, subject obligations and dated reasons for decisions. A clause editor displays costs, consent gates, conflicts and execution dates. A territorial selector previews before/after legal ownership, control, population and accounts. War views show objectives, fronts, supply and battle reports; peace views show which parties each clause binds. Subjects have a powers/obligations view and autonomy/independence actions. Purchases, leases and releases are discoverable outside wartime menus.

Consequential actions use a reviewable command preview with explicit confirmation, followed by actual ratification and execution. Notifications can pause for demands, calls to arms, declarations, ratification, major battles, transfer failures and breaches. Missing intelligence stays unknown. Keyboard and scaled layouts must expose the same commands as the terminal.

AI uses the same legal gates, budgets, observations and order interfaces. Evaluate offers against alternatives, strategic interests, expected war costs and domestic constraints; preserve decision reasons and stable tie-breaking for replay. Do not grant omniscience, free resources or guaranteed submission. Re-evaluate when evidence changes, with bounded decision cadence and proposal cooldowns to avoid spam or treaty loops.

## Ordered implementation todos and acceptance gates

All boxes are open. Completing this document does not complete these requirements. The global priority register remains intact; the sequence below is the dependency order within this workstream. Unrelated earlier systems do not have to be complete before a bounded diplomacy slice starts. Fictional fixtures come first; historical coverage requires separate sourced work.

### Readiness and first implementation

The existing engine supplies deterministic time, shared command validation, outcomes, information projections, accounts, domestic consent and replay infrastructure. International agreements must extend these contracts rather than assume domestic commitments are already treaties.

The first implementation is a bilateral non-aggression agreement lifecycle in the fictional three-country world: offer, counteroffer, signature/ratification, activation, expiry and termination. Include institutional authority, a public treaty inspector, basic AI acceptance/refusal and pending-state save/replay. Until declarations and combat exist, this proves agreement behavior only; it cannot demonstrate military enforcement or complete P23-09.

Territorial actions require authoritative simulation regions with assigned residents, production, infrastructure, assets and obligations, plus dynamic country creation and atomic transfers. Reference polygons and national totals are insufficient. Battles require actual formations, equipment, readiness, routes and supply. Military coercion requires those capabilities to make threats meaningful.

### Implementation checklist

- [ ] Define the first slice's typed Rust records, commands/events, lifecycle transitions, daily phases, content/save versioning and compatibility behavior. Specify bounded initial AI rules and a fictional acceptance fixture before marking implementation complete.
- [ ] Implement bilateral proposals, authority, ratification, visibility, expiry and termination (P23-01–08/11 and the lifecycle portion of P23-09). Verify counteroffers, government succession, public/private projections, AI/player command parity and pending-state replay in both clients.
- [ ] Establish sovereign units, regional population/assets/obligations, new-country identities and separate title/control/recognition (P02, P04, P27-05/06). Verify islands, enclaves, disputed title, regional reconciliation and unchanged state after invalid transfers.
- [ ] Implement atomic cash purchases, exchanges and voluntary independent release (P27-10/11/13). Verify payment and asset/liability conservation, failed ratification, stale offers, competing transactions and creditor rights.
- [ ] Implement subject relationships and release-as-subject (P27-09/10/13). Verify delegated powers, separate accounts, non-cyclic relationships, autonomy requests and negotiated independence. Armed rebellion waits for P29 dependencies.
- [ ] Implement military establishment, logistics, movement and operational orders (P25, P26-01–06). Verify actual procurement, maintenance, access, route capacity and supply-limited formations.
- [ ] Implement battles, fronts, casualties, war consequences and coalition participation (P26-07–10). Verify retreat, surrender, blockade, personnel/equipment reconciliation and explainable reports.
- [ ] Implement limited peace, occupation, withdrawal, liberation and enforcement (P27-01–08/13). Verify separation of control/title and rejection of settlements that dispose of non-signatories' rights.
- [ ] Complete alliance/guarantee triggers, calls to arms, conflicting obligations and non-aggression enforcement (P23-09, P24). Verify defensive pacts exclude offensive wars and refusal creates explicit consequences.
- [ ] Implement coercive demands, pressure, counteroffers, mediation and de-escalation (P23-10). Diplomacy-only demands can use the agreement framework earlier; military threats wait for actual capabilities. Verify peaceful resolution and that deadline expiry alone cannot issue an unauthorized war command.
- [ ] Complete subject integration/rebellion, installment purchases, leases and concessions (P27-09–13, P10, P17, P22, P29). Verify real obligations, explicit remedies, lease expiry without sovereignty change and preservation of claims.
- [ ] Complete integrated AI, native/terminal workflows, historical content and long campaigns (P23-11, P26-10, P27-13, P36, P39–40). Verify refusal of unacceptable offers, limited-war settlement, information boundaries and clearly labeled historical/fictional data.

Required regression scenarios include alliance refusal; guarantee escalation; non-aggression notice and breach; a crisis resolved without war; supply-limited invasion; costly stalemate ending in limited peace; occupied but unrecognized land; release with debt and enclaves; creation and independence of a protectorate; subject delegation conflicts; cash sale with failed ratification; simultaneous offers for the same land; installment default without automatic border reversal; lease expiry without sovereignty change; and multi-party settlement with a non-signatory. Each must check causal outcomes, account/population conservation, privacy, command parity, transactional failure and deterministic save/replay. Native UI checks must verify actual controls and scaled views when those features exist.

Balance coefficients, precise battle formulas, AI reservation weights and historical preset calibration remain implementation tuning work. They must be documented with bounded units, reproducible fixtures and sensitivity checks before the affected requirement can be marked complete. The player actions, state distinctions and acceptance gates above are required scope, not optional tuning decisions.
