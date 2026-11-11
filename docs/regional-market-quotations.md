# Regional market cash quotations

11 November 2026. **Proposed arithmetic for part of MT-02; no implemented market or completed readiness gate.** Read with the [integration contract](regional-market-integration.md) and [market plan](markets-and-trade-design.md). Values below are authored worked examples, not historical prices, calibrated parameters or simulation results. This fixes a candidate cash-quotation boundary; production, household demand, utilization, allocation priorities and the full fixture remain outstanding.

## Physical quantities and cash

Use integer physical quantities and the existing whole-million-credit `Money`. Each good has a positive physical dispatch quantum. A shipment quantity must be a positive multiple of that quantum; household consumption is not restricted to shipping quanta. Do not reinterpret Food as industrial biomass, or national currency labels as implemented exchange rates. All first-prototype payments use the existing common accounting credit; foreign-exchange markets remain separate roadmap work.

A regional quote contains a positive base cash numerator `B`, positive physical denominator `L`, and the region's price index `P` in basis points. Its rational unit price is `B * P / (L * 10000)`. `L` is the quotation denominator, not necessarily the dispatch quantum. Thus an authored quote of 10m per 100 Food can still ship in quanta of 10 Food. Preserve that rational quote until computing the complete actual transaction; rounding a unit quote first would distort small orders.

For candidate quantity `q` from one frozen observation snapshot:

| Cash component | Exact candidate rule |
| --- | --- |
| Exporter payment | `ceil(q * origin_B * origin_P / (origin_L * 10000))` |
| Expected destination gross receipts | `floor(q * destination_B * destination_P / (destination_L * 10000))` |
| Each link's capacity usage | `q * good_transport_weight` using a positive integer weight |
| Each external transport payment | `ceil(capacity_usage * link_fee_numerator / link_fee_denominator)` |
| Import tariff | `ceil(actual_exporter_payment * destination_tariff_bps / 10000)` |
| Importer cash commitment | Exporter payment plus actual external transport payments plus tariff |
| Expected cash margin | Expected destination gross receipts minus importer commitment |

All divisors must be positive and all quantities, rates and monetary inputs nonnegative. Use checked wide multiplication/addition and checked narrowing; integer overflow is an error, never a wrapped price. Compute a positive ceiling as quotient plus a nonzero-remainder indicator, rather than adding `denominator - 1` to a potentially large numerator. A zero-quantity query returns all zeros and cannot allocate a shipment ID or reserve stock/capacity. Every positive purchased shipment has a positive exporter payment, even when its rational value is below 1m.

Quotes are conditional observations. Expected destination receipts are not a receivable, cash, a guaranteed sale, or GDP. A merchant's inventory book value is its actual acquisition cost. A higher destination price cannot create a cash receipt before a buyer pays. MT-02 must still connect the monetary destination quote to the new rules' actual household or producer charges; the parent's basket index alone does not supply that conversion.

## Actual recipients and reservation

The importing country's firms pay the supplying inventory's actual owner, each link's named operator, and the destination treasury. Civilian private output is sold by Firms; existing public auxiliary inputs are sold by Treasury. A domestic public-input sale also moves actual cash from Firms to Treasury; it cannot duplicate stock or take protected construction supplies. The tariff base excludes transport fees and the tariff itself. Charge it once on actual dispatch; arrival is not a second customs collection. An importer cannot be charged for a requested quantity that is not dispatched. Reopening or cancelling a route does not refund settled purchase, tariff or transport payments. A refused or unfilled order transfers nothing.

When an operator is the same aggregate account as the importer, its nominal internal fee is not a cash transfer or operating receipt. Capacity usage and physical operating costs still apply. Forecasts and reports must distinguish the listed fee from the actual external payment. Other same-country payments, such as Firms to Treasury, are real transfers between different accounts. Cross-border cumulative cash positions include only legs whose countries differ.

Reserve all outgoing commitments against the payer's frozen spendable cash before making any commercial payment. Do not recycle exporter or operator receipts into another purchase in that clearing. One owner may operate several routes, so reservations are shared by account, not independently reset for every route. The same is true of destination demand, source stock and link capacity. No reservation may use client advances, payroll creditors or the selected liquidity buffer.

Quote only the final filled quantity. If stock, demand, cash or shared capacity reduces an order, recompute its full quote before accepting it. Splitting one transaction into smaller shipments can increase ceiling charges; never assume the sum of separately rounded quotes equals the original quote. Allocation must not use hypothetical cheap fractions of an order and then settle a different rounded bill.

Cash commitment is nondecreasing in quantity, so the maximum affordable quantum count can use checked integer search. Rounded expected margin need not be monotone at every quantum. A quantity that passes cash/capacity limits must also pass the final margin rule; MT-02 must specify the bounded search/tie rule before implementation. Rejection releases every temporary reservation and leaves authoritative counters and history unchanged.

## Worked boundaries

Use quotation denominator 100 physical units, base 10m, origin index 10000 and destination index 16000. Let the dispatch quantum be 10 units, transport weight 1, one external link fee 3m per 100 capacity units, and destination import tariff 1000 basis points. These numbers instantiate the market plan's illustrative margin; they are not a proposed universal tuning set.

| Case | Exporter | Transport | Tariff | Commitment | Expected gross | Expected margin |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 100 units | 10m | 3m | 1m | 14m | 16m | 2m |
| 90 units | 9m | 3m | 1m | 13m | 14m | 1m |
| 10 units | 1m | 1m | 1m | 3m | 1m | -2m |
| Zero units | 0m | 0m | 0m | 0m | 0m | 0m |

An importer with 13m of spendable cash cannot dispatch 100 units. If the allocation rule permits 90, its actual commitment is 13m; it does not receive 100 units for a partial 13m payment. If the dispatch quantum were 100, it could buy no shipment with that cash. The 10-unit order must not be described as profitable merely because an unrounded per-unit spread is positive.

For shared capacity, remove tariffs and fees only in this separate boundary example. Two good-specific routes each request 80 weighted units over a bidirectional link with capacity 100. Whichever receives first allocation under the declared stable ordering can use at most 80; only 20 remains for the other. At the same source/destination quotes above, those filled quantities cost 8m and 2m and have forecast gross receipts 12m and 3m. They use 100 capacity in total, rather than granting each direction or good its own 100. Another route to the same destination must also share the same remaining buyer demand; capacity alone does not authorize a second sale.

In a zero-stock origin, neither purchase cash nor capacity is committed. In a zero-demand destination, do not create demand from a high quoted price. A quoted price at a cap still requires actual goods, a funded buyer and a usable path. A closed link rejects new dispatch, while a shipment bought yesterday arrives with its recorded owner and cost even if the link closes today. Changing a tariff today affects today's new dispatches; it does not reprice cargo already in transit.

## Carrying value and realized earnings

For the 100-unit example, suppose the exporter's existing inventory cost is 7m. Dispatch removes the sold quantity and its 7m carrying value; the exporter receives 10m and records 3m of modeled trading earnings before other expenses. The importing owner exchanges 14m of cash for cargo carrying value 14m, including actual transport and tariff charges. The operator receives 3m and the treasury receives 1m. No cash is created, and the importer's forecast 2m margin is not earned at dispatch.

Arrival transfers that 14m cost from transit to local inventory without another payment or profit. If the owner subsequently sells the entire quantity for an actual 16m, the resulting gross margin is 2m before other expenses. If none sells, the goods and their cost stay with the owner; a price forecast alone cannot pay its wage claims. Use the parent's checked carrying-value conventions for partial use and losses. For example, retaining 75 of 100 units from a 14m holding gives a rounded-down closing value of 10m and a 4m removal/write-off; do not retain 14m after losing a quarter of the goods.

Supplier receipts, cargo cost, transport receipts, tariff receipts, current production expenses and subsequent final sales must reconcile across the existing business statements and cash bridges. Do not count the same purchase as both immediate consumption and inventory acquisition. A transport operator's receipts are not all profit: the production/transport rules must identify its actual labor, inputs, maintenance and loss costs before the three-region scenario is accepted.

The [household-demand candidate](regional-market-household-demand.md) works through a common cash quote, actual affordability and partial-delivery charges. Its budget, domestic allocation and price-input decisions remain open.

The [commercial-clearing candidate](regional-market-clearing.md) specifies shared reservations, stable route priority, bounded whole-quantity search and all-or-nothing staging. It includes a nonmonotone rounded-margin example; domestic allocation and full economic readiness remain open.

## Remaining MT-02 exit conditions

Still specify and test: monetary quote formation for each connected good; actual household/producer payment rules; production recipes and shared paid-labor allocation; utilization response and funded inputs; wages assessed versus paid once; working-stock targets and price-pressure inputs; merchant entry, target changes, shutdown and re-entry; retained earnings and protected liquidity; deterministic route/demand allocation; physical transport operation/maintenance; and legal delayed policy/upgrade effects. The full fixture must predeclare baseline tolerances and shock comparisons before simulation runs. The worked arithmetic above closes none of those empirical or integration gates by itself.
