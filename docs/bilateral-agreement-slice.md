# First bilateral agreement slice

11 November 2026. **Implemented and locally validated; diplomacy requirements remain partial.** This is the first bounded slice of the [war and diplomacy plan](war-and-diplomacy-design.md), rather than a replacement for its remaining requirements. Use a fictional three-country fixture before historical authoring. It demonstrates agreement lifecycle and institutions; declarations, proxy intervention, armed forces and military enforcement remain unimplemented and P23-09 cannot be completed by this slice.

## Version and state

Extend optional payroll rules 40 / snapshot 41 with **agreement rules 41 / snapshot 42**. The default remains finance rules 38. New definitions and world agreement state are absent and omitted from serialization for all archived rules. No automatic campaign migration, implicit default promotion or old JSON rewrite is permitted. The regional-market planning reservation must move to the next unused version before that system is implemented; it has no persisted rules-41 campaign to preserve.

Add distinct `TreatyId` and `TreatyRevision` types. One world-level store contains a checked next-ID counter and agreement records, in stable-ID order. Every record names two distinct existing countries, sorted canonically, the current offering government, terms, current revision, signatures, domestic ratifications, lifecycle dates, paid administration by party, milestones and any termination notice. Government signatures reference the existing government identity. Names are display fields, never keys. Revisions invalidate signatures and ratification of older terms.

The first clause is a mutual promise of non-aggression between the two signatories, with duration and termination notice. No proxy-support exception, third-party beneficiary, secret clause, material transfer or automated military reaction is silently assumed. Scope and current enforcement limits appear in its inspector. Other treaty kinds wait for their own rules and capabilities.

Retain closed records and milestone references. Bounds are explicit content limits: at most 4096 records per campaign, at most 16 revisions per record, and at most 64 milestones per record. Exhaustion rejects an action before charging money or allocating an ID. The allocator cannot wrap or reuse a closed treaty's identity. Old Chronicle entries may be pruned under existing retention without erasing the treaty history.

## Commands and transitions

All orders go through ordinary shared command envelopes, validated actor control, government context, recorded outcomes and transactional failure. Player and AI use the same action handlers. Existing command identifiers continue to identify submissions; they are distinct from treaty and revision identities.

| Command | Allowed state and result |
| --- | --- |
| Offer non-aggression pact | No live pact or negotiation for the pair, and cooldown elapsed. Validate terms, authority and cash; charge the offerer's administration; allocate revision 1; record the offering government's signature and a dated offer. |
| Counteroffer | Only the current receiving party, during negotiation and before the offer deadline. Require the exact current revision and changed valid terms. Charge its administration; increment revision; discard old signatures/ratifications; record only the counterofferer's signature. Reset the response deadline. |
| Sign | Only the other party, for the exact current revision and before its deadline. Recheck its authority/cash and charge administration. Two signatures move the pact to awaiting domestic ratification; signature alone never activates the pact. |
| Submit ratification | A signed party, for the exact revision, with no existing request or passed ratification. Recheck authority and cash, charge administration, and schedule its domestic review after that government's configured consent interval, with at least one day before resolution. |
| Reject | Only the current receiving party before both signatures. Close the negotiation as refused; no refund of previously paid administration. |
| Withdraw | Either party before activation, including scheduled activation. Close as withdrawn; paid administration and cooldown remain. An active pact requires termination notice instead. |
| Give termination notice | A party to an active pact with no prior notice. Keep the pact operative until the earlier of natural expiry and the notice deadline. Zero-day notice terminates immediately. No payment, border change or automatic attack occurs. |

Terms use typed `DaySpan`: duration 30–3650 days, notice 0–duration. The fixture response window is 30 days and pair cooldown is 30 days after closure. Only one live agreement/negotiation per unordered pair is allowed; stale concurrent offers cannot create two overlapping pacts. Counteroffers require the current revision, and a receiving party cannot sign an obsolete offer after a counteroffer. Changing only display names cannot change these rules.

Ratification passes only if the recorded government still holds office and its ordinary governing backing/council conditions still permit the decision. Re-evaluate at the actual review boundary. Failed ratification closes the proposal with a recorded reason, retaining sunk fees. Success by one party is insufficient. After both domestic ratifications pass, schedule activation on the following day; do not retroactively make the agreement active on a signature or submission date.

Before activation, a change of either sponsoring government lapses the proposal and requires a new offer after cooldown. This explicitly conservative first-slice rule avoids silently treating a predecessor's incomplete mandate as a successor's signature. Once active, state obligations survive ordinary government succession. New military or constitutional rules cannot infer otherwise from a leader-name change.

Expiry is `activation day + duration`; the interval is start-inclusive/end-exclusive. Notice is `notice day + notice duration`. Natural expiry wins a same-day tie with notice termination, with one closing milestone. Closed states distinguish expired, terminated after notice, refused, withdrawn, timed out, failed ratification and lapsed sponsorship. A deadline is a lifecycle event, never authorization to declare war.

## Authority, money and outcomes

Use the government's current ordinary route and its backing/council gates for offer, counteroffer, signature and ratification submission/review. These are authored institutional procedures, not a claim of complete treaty law. Rejection, withdrawal and an allowed termination notice cannot be blocked by an unrelated funding shortfall. They still require actor control and the correct treaty/revision/state.

The fictional fixture charges **one basis point of opening annual GDP**, rounded down with a minimum 1m, for each accepted offer, counteroffer, signature and ratification submission. For opening GDP 100000m, each charge is 10m. A straightforward two-signature/two-ratification pact therefore pays 40m in actual administration, split 20m per country. A counteroffer adds its separate 10m service; no invented deposit is refundable later. A rejection or invalid/stale command charges nothing.

Administration transfers actual unreserved Treasury cash to that country's Firms as an earned procedural service. Use the existing public business-receipt/cash bridge; it is not deferred construction capital, a bilateral payment or GDP created by signing. Record payer, payee, purpose, treaty/revision and paid amount. Protect public local funds, retained cash and pending programme reservations through the existing available-cash boundary. No borrowing, wage claim or counterpart payment is created implicitly. Offer previews show the immediate service charge and conditional future ratification steps separately.

Emit typed treaty contexts in the Chronicle for both participants, linked to an actual milestone with matching day, revision and lifecycle state. A failed event/command/treaty allocator or failed accounting validation rolls back the entire command or current daily transaction, including both countries' events, cash, records and counters. Existing earlier completed days remain committed if a later day fails. Do not weaken old programme-event link validation to accommodate the new context.

## Daily phases and AI

For new rules only, after the day advances and demographic review completes, resolve agreement deadlines, sponsorship, due domestic ratifications, scheduled activation, notices and expiry once in stable treaty-ID order. This is a world phase before countries' economic settlement and future movement/combat. Revalidate the completed agreement state before committing the day.

Run bounded offer-response AI after all country settlements, so the result does not depend on which country settles first. The fixture decision cadence is every three elapsed days. Non-player countries may respond to offers and submit their own domestic ratification; the first slice does not generate unsolicited offers or endless counteroffer loops. Submit each selected action through the same shared command path with AI origin. No free authority, fees, signature or cash is granted.

The authored fixture gives BOR a 365-day maximum duration and 30-day minimum notice. It signs compatible offers; otherwise it counteroffers bounded compatible terms while revisions remain available. COR's authored stance declines this pact. These are teaching rules, not hidden historical claims or a strategic international AI. AI reads its own institutional and cash conditions and the received offer, without consulting the other government's private council, treasury or future decisions. Participant-facing outcomes identify the response and the current offered terms in the agreement inspector; unaccepted refusals remain private to those participants. These views do not disclose the other government's private backing, cash or AI policy thresholds. Full interests, threats, reputation and alternative valuation remain P23/P39 work.

After both signatures, an eligible non-player participant submits ratification on its next decision boundary. Insufficient authority or cash cannot be bypassed: the request remains unsubmitted until a later eligible boundary or deadline. A signed proposal has its own 30-day ratification-submission window, and pending reviews can finish after that window only when submitted in time. This prevents an unsigned or unsubmitted agreement from remaining pending forever while honoring an already scheduled lawful review.

## Views and both clients

The public treaty inspector shows agreements once both parties have signed, their current terms, public ratification progress, activation/expiry/notice dates and closed outcomes. Unaccepted offers/counteroffers and their drafting costs are private to the participants. Public reports must not expose a negotiation simply because a third country is selected. Internal backing, cash and AI policy thresholds remain private even for publicly signed pacts.

An owned-government view adds its pending negotiations, its previewed actions and the immediate costs/authority conditions. Observer and foreign-government requests receive only the public projection. The existing host authenticates the viewer; a caller naming a foreign government cannot obtain its private negotiation state. Developer inspection stays a separately flagged route.

Headless provides inspection and exact-revision offer/counter/sign/ratify/reject/withdraw/notice commands. Retain machine command tokens where needed while showing readable terms and dates. Desktop consumes the same projections and previews; drafting a counteroffer does not submit it. Consequential actions use a reviewable preview and explicit submit control. The native view must distinguish signed, ratification pending, scheduled and active, and keep the two participants' decisions separate.

## Acceptance before completion claims

Required behavioral cases: paid accepted lifecycle and natural expiry; counteroffer revision invalidation; COR refusal; exact deadline and same-day notice/expiry boundaries; withdrawal and fee retention; government succession before versus after activation; backing/cash refusal at submission and review; duplicate pairs and IDs/revisions exhausted; unrelated-country privacy; forged/malformed state and content rejection; event links retained/pruned; failed operations leaving exact bytes unchanged; player/AI command parity; and pending-signature/ratification/notice save-resume equality.

Use the existing runner and a reproducible, explicitly fictional fixture based on the preserved rules-40 three-country parent. Check all archived content bytes and recovery hashes. Both clients must replay and resave the same pending/active/closed states. Native interaction at 100%/140% must exercise actual offer/response/ratification/notice controls and a foreign public inspector. A passing test or an authored acceptance rule does not prove military enforcement, full diplomacy, historical coverage or every remaining war-plan scenario.
