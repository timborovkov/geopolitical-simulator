# Regional market commercial clearing

11 November 2026. **Candidate rules for part of MT-02; not implemented or calibrated.** Read with the [integration contract](regional-market-integration.md), [cash quotations](regional-market-quotations.md) and [household demand](regional-market-household-demand.md). This specifies commercial reservation and allocation only. Domestic allocation, production, protected liquidity, route targets and price inputs remain separate prerequisites. No requirement status or default campaign changes.

## Freeze before allocating

After every country's domestic settlement, construct one immutable observation and one temporary reservation book. Arrivals already received and domestic sales already settled must not also appear as fresh demand or available export stock. Existing private and public owners keep separate stock positions and cash accounts.

| Reservation | Shared key and opening limit |
| --- | --- |
| Cash | Country/account owner; actual cash less all selected protections and prior reservations |
| Goods | Origin region, inventory owner and good; actual unprotected exportable stock |
| Buyer demand | Destination region and good; observed residual affordable demand after deductions for usable unsold imported stock and already-owned incoming cargo |
| Transport | Link ID; operational shared capacity across all goods and both directions |

The demand input is an upper bound on plausible resale, not a household reservation or guaranteed next-day purchase. MT-02 must still define its exact construction and reconcile it with household/producer orders and working-stock targets. Different merchants cannot each claim the full same observation. Deduct existing cargo only once when constructing that observation; a newly accepted dispatch consumes the temporary demand balance, rather than subtracting itself again from the frozen input.

The protected cash limit is likewise an input to this allocator, not permission to omit unpaid wages or protected public-client funds. A country's producer and merchant activities share one Firms wallet. Domestic Firms and Treasury remain different account owners. Receipts generated during commercial clearing never increase any frozen outgoing cash limit, including when the recipient also operates another route.

## Bounded selection and exact charges

The candidate uses a single pass in ascending stable route-ID order. Reject duplicate route IDs in content. Do not rank by vector order, country settlement order, destination price alone or client selection. Route-ID priority is an authored allocation advantage and must be visible; it is not a claim of a competitive auction or equal access. A later fairness mechanism requires an explicit rule and replay change.

For each eligible route, inspect the frozen route target, access, supplying owner and receiving owner. A closed link, zero target, zero exportable stock or zero residual demand yields no dispatch. Convert stock, demand and every remaining link capacity into a maximum whole number of the good's positive dispatch quantum. Take their minimum with the target quantity's quantum count and an authored per-route daily maximum. Targets and maxima are integer quantum counts, not quantities to round up.

Bound that authored maximum to **1 through 4096 quanta per route per day** in the candidate content contract. This is a computational bound, not a free production or transport grant. It lets the allocator examine every possible filled quantity without assuming rounded margin is monotone. MT-03 must choose physical quanta and route maxima that represent its intended economy; changing the quantum changes both attainable volume and rounding.

For candidate counts from the resulting maximum down to one:

1. Compute the complete quantity's exporter payment, each actual external operator payment, tariff, total importer commitment and conditional destination gross under the frozen quotation rules. Do not add independently rounded hypothetical subshipments.
2. Require total commitment to fit the importing owner's remaining frozen cash limit.
3. Require expected gross minus commitment to be at least the route's authored nonnegative minimum expected cash margin. This margin is a complete-shipment money amount. Route opening/persistence and target adaptation must still be specified separately; accepting a quantity here does not establish strategic profitability or guarantee resale.
4. Accept the first passing quantity, thus selecting the largest eligible fill under these explicit rules. If none passes, reserve nothing and move to the next route.

On acceptance, subtract the exact commitment from that payer's remaining cash, the quantity from its supplying owner's stock and the shared destination demand, and the weighted quantity from every link on the route. Append one staged shipment and its actual payment legs. A route cannot revisit a region or link, dispatch twice that day, or sell newly acquired cargo onward in the same phase. A zero fill allocates no persistent identity.

Process each route once. Do not revisit an earlier losing route after later receipts, divide its invoice into many cheaper fragments, or release its accepted capacity to promise it to another route. Declined candidates do not reserve anything. Expected receipts never become cash, a receivable or an inventory carrying value.

## Stage the whole commercial phase

Route selection builds a staged plan without mutating the authoritative world or persistent ID counters. Apply all accepted actual debits/credits, owner stock removals, cargo additions and business/public receipts to a staged world. Check recipient overflow, cost-basis removal, cumulative cross-border positions, remaining reservation bounds and next-ID availability before publishing it. Assign shipment IDs in accepted route-ID order within the staged plan.

Insufficient route cash is an ordinary no-fill outcome. An invalid quote, overflow, inconsistent inventory cost or exhausted shipment allocator is an error that aborts the entire enclosing daily transaction, including earlier accepted routes. Do not silently skip corrupt state and publish the rest. An apply error leaves the clock, both countries, all balances/stocks, cargo, capacity, journals and persistent counters at their original values under the existing transactional step boundary.

Temporary reservations end when the commercial phase commits or aborts. Accepted purchases already paid become owned cargo, rather than a second spendable reservation. Unused cash and stock remain with their original owners. Cash received in this phase becomes potentially spendable at the next day's observation, subject to that day's actual obligations and protections.

## Worked shared-resource cases

These examples use the quotation document's 10m/100 Food origin quote, 16m/100 destination quote and 10-Food dispatch quantum unless stated otherwise. All routes are already eligible, their targets are funded only by the limits stated here, and the minimum expected margin is zero.

| Case | Frozen limits and actual allocation |
| --- | --- |
| Shared road | Remove fees/tariffs for this case. Routes `A` and `B` each target 80 Food across a link with capacity 100 and weight 1, using disjoint stock and buyer demand. `A` receives 80, costing 8m with forecast gross 12m; `B` receives 20, costing 2m with gross 3m. Shared use is 100, even if they travel in opposite directions. |
| Shared buyer demand | Remove fees/tariffs. Independent roads and suppliers each offer 80 Food into the same destination's 100-Food residual demand. `A` receives 80 and `B` at most 20; spare road capacity cannot authorize another 80. |
| Shared wallet | Restore an external link fee of 3m/100 weighted units and a 1000-basis-point tariff. Two routes share one importer with 13m, each targeting 100 Food. `A` cannot afford 100, accepts 90 for 13m with forecast gross 14m, and leaves no cash for `B`. Neither gets a separate 13m balance. |
| Receipts cannot fund another order | Remove fees/tariffs. Country X's Firms has 10m; country Y's Firms has zero spendable cash. Route `A` buys 100 Food from Y, moving 10m from X to Y. Route `B`, paid by Y, still has a frozen zero limit and dispatches nothing today. Y's actual receipt is retained for the next observation. |
| Rounded loss | With the fee and tariff restored, stock or capacity limits a route to 10 Food. Exporter 1m, transport 1m and tariff 1m exceed forecast gross 1m. Even a rich importer dispatches zero at the zero-margin threshold. |
| Margin is not monotone | Use a separate zero-fee/zero-tariff quote of 10m/100 Food at origin and 11m/100 at destination, with quantum 1 Food. One unit costs 1m and forecasts 0m; ten units cost 1m and forecast 1m; eleven units cost 2m and forecast 1m. A maximum of eleven selects ten, despite eleven being affordable. Binary-searching a profitable-quantity predicate would be invalid. |
| Protected funds | The importer has 40m actual cash, of which 12m is protected client capital, 8m covers unpaid wages and 7m is the separately specified nonoverlapping liquidity protection. Its frozen limit is 13m, so the fee/tariff example fills 90, not 100. If protections overlap, their actual shared protection calculation must resolve overlap before supplying this limit. |

The last row illustrates the allocator's input, not a final liquidity formula. A route's own operator account incurs no nominal transfer to itself; shared physical capacity and the still-to-be-defined funded transport costs continue to apply. Reversing definition-vector order leaves all cases unchanged. Renaming stable route IDs intentionally changes priority; changing only a display label must not.

## Required integration evidence

Test all rows inside the shared simulator after adoption, including several routes with one payer, a public supplying owner, repeated destinations, multiple goods and bidirectional shared links. Retain exact quote components and show stock, demand, cash and capacity limits separately in owned diagnostics. Public reports must not reveal private wallet or protection limits.

Add invalid-content checks for zero quanta/weights, duplicate IDs, discontinuous or repeating paths and per-route maxima above the bound. Verify rollback after a late recipient overflow, cost-basis mismatch or ID exhaustion, after earlier routes had already been staged. Save/resume with cargo in transit, no same-day onward resale and unchanged archived hashes remain required.

MT-02 remains open for domestic allocation, physical recipes and paid labor, exact demand/price observations, route target persistence, insolvency/re-entry, protected liquidity and transport operations. MT-03 must still author opening accounts and predeclare baseline and shock tolerances. These arithmetic examples are not an executed regional economy or a completed market milestone.
