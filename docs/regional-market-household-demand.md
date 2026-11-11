# Regional market household demand

11 November 2026. **Candidate rules for part of MT-02; no implemented market, calibrated scenario or completed readiness gate.** Read with the [integration contract](regional-market-integration.md), [cash quotations](regional-market-quotations.md) and [market plan](markets-and-trade-design.md). Examples are authored arithmetic. The recommended campaign and every archived settlement rule remain unchanged.

## Why the parent basket cannot supply a unit price

The rules-40 parent computes a cohort's food budget by splitting essential spending between food and other consumption using the food price index. It scales requested physical food by that cohort's affordable basket fraction, then charges the delivered fraction of its food budget. This is internally defined basket accounting. Two cohorts with different essential budgets and physical needs need not pay the same cash amount per Food. Its basis-point index therefore cannot also be treated as a wholesale cash price without changing actual household settlement.

The regional version needs one explicit rational cash quote per good and region. The candidate here uses the same positive numerator `B`, positive physical denominator `L` and positive basis-point index `P` as the cash-quotation document. Households buy at `B * P / (L * 10000)` before transaction rounding. No cohort receives a different underlying quote because it has more income. Actual cash payments, physical delivery and book-cost removal must use the same filled quantity.

## Separate need, budget, order and delivery

For each cohort, retain these distinct values:

- Physical food need, derived from the authored need rule and actual residents, without reducing it when cash is scarce.
- An explicit food cash budget, bounded by cash actually available after due income, transfers and withholding. Forecast income, savings already spent, unpaid wage claims and inventory value cannot fund it.
- Affordable ordered quantity at the current quote, capped by physical need.
- Filled delivery, bounded by that order and actual stock allocated under the still-to-be-defined domestic settlement rule.
- Actual cash charged, cash retained, unmet physical need and unfilled affordable order.

This document does **not** choose the food-budget rule or give household orders priority over public construction and producers. Reusing a parent's essential-spending allocation may be a candidate budget, but its formula, price response and placement relative to other consumption need their own decision. A unit-price rule alone cannot settle those questions.

For integer food budget `C >= 0`, physical need `N >= 0` and actual allocated quantity `q`, the candidate arithmetic is:

| Boundary | Exact rule |
| --- | --- |
| Affordable order | `a = min(N, floor(C * L * 10000 / (B * P)))` |
| Actual delivery | `0 <= q <= a` |
| Cash payment | `ceil(q * B * P / (L * 10000))`, with zero for zero delivery |
| Retained food budget | `C - actual cash payment` |
| Unmet physical need | `N - q` |
| Unfilled affordable order | `a - q` |

Each cohort delivery is one actual transaction. Do not impose the merchant dispatch quantum on household consumption, combine several deliveries after separately quoting them, or charge the whole budget for a partly filled order. Checked wide products, positive denominators and checked narrowing follow the quotation contract. Implement ceiling by quotient/remainder rather than an overflow-prone numerator adjustment.

Affordability follows directly: `q <= floor(C * D / U)` implies `ceil(q * U / D) <= C`, with `U = B * P` and `D = L * 10000`. A later allocation may reduce delivery and payment; it cannot increase either beyond the original funded order. All unspent money remains with the cohort. Zero cash buys zero units at a positive quote even if physical need is large.

Transaction rounding remains visible. Small purchases may pay more per delivered unit than a large purchase under the same rational quote. This is a consequence of the existing whole-million-credit money unit, not a hidden rich/poor price schedule. The fixture must test whether its quantities make that granularity acceptable; a finer currency would require an explicit separate protocol decision, never a display-only change.

## Worked cash and rationing boundaries

Use base cash numerator 10m and physical denominator 100 Food. Every row has physical need 100 Food except the explicitly tiny transaction. The supply column is the stock already allocated to this cohort under a hypothetical rationing outcome; it is not a second independent grant of shared stock.

| Case | Food budget | Price index | Allocated supply | Affordable order | Delivered | Actual payment | Retained budget |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Lower cash | 3m | 10000 | 100 | 30 | 30 | 3m | 0m |
| Higher cash | 10m | 10000 | 100 | 100 | 100 | 10m | 0m |
| Lower cash, higher price | 3m | 16000 | 100 | 18 | 18 | 3m | 0m |
| Higher cash, higher price | 10m | 16000 | 100 | 62 | 62 | 10m | 0m |
| Physical rationing | 10m | 10000 | 40 | 100 | 40 | 4m | 6m |
| No cash | 0m | 10000 | 100 | 0 | 0 | 0m | 0m |
| Tiny transaction, need 10 | 1m | 16000 | 10 | 6 | 6 | 1m | 0m |

The first two rows share a unit quote of 0.1m per Food. Increasing price reduces affordable quantities while leaving physical need observable. In the rationed row, the seller receives 4m, the household retains 6m and 60 Food of need remains unmet. Neither the unfilled order nor retained cash becomes a seller receipt. The tiny row demonstrates why an unrounded quote cannot be displayed as an exact final payment.

## Tie actual sales to the owner accounts

Delivery must transfer household cash to the inventory's actual selling owner and remove that owner's exact delivered quantity and carrying cost. Existing imported commercial inventory belongs to the receiving Firms owner after purchase; households do not pay the foreign exporter a second time on its arrival. Arrival itself makes no household payment and earns no resale margin. Public in-kind reserve releases retain explicit public provenance and cannot become private cash sales without a separate ownership rule.

Use the domestic actual-sale receipt once in the household consumption report, business receipts, inventory-cost removal and cash bridge. The difference between sale receipts and removed acquisition cost is a realized modeled margin before other expenses. Neither unmet need, a standing order nor a price increase is earned income. Other consumption must have its own explicit charge and remain bounded by the cash left after the accepted food reservation.

The merchant destination gross quote remains a conditional estimate. It must be backed by observed unsatisfied affordable demand and must share that demand across routes. Tomorrow's household budgets, local supply, prices and competing cargo can change before resale. Even when household transaction ceilings exceed a whole-lot floor estimate, merchants cannot book that difference or spend it before actual sales. Whole-world accounting follows actual payments, not predicted margins.

## Remaining decisions before economic implementation

Choose the food-budget and physical-need rules, their price response and their placement relative to income, withholding, producer purchases and other consumption. Specify stable allocation of shared domestic stock and owner reservations; per-cohort examples do not provide an allocation algorithm. Choose exactly which cash-backed demand and stock observations enter next-day prices, while continuing to report unmet physical needs separately. Preserve the selected one-price observation barrier rather than adjusting prices once for households and again for exports.

The full MT-02 contract must also resolve producer recipes and paid labor, units and public-input purchases, utilization and insolvency, merchant liquidity and entry/re-entry, route allocation and physical transport expenses. MT-03 must author and reconcile the connected fixture and predeclare baseline/shock tolerances. Test the candidate payment rules inside the shared simulator with replay, cash/goods reconciliation, poor and wealthy cohorts, zero/tiny quantities, rationing and ownership boundaries before adopting them. The worked examples here complete none of those implementation or calibration gates.
