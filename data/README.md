# Initial content pack: 1 January 2000

The bundled `initial-2000.json` uses **content schema 2**, ID `initial-2000-v2`. It seeds 15 countries, 34 parties/governing bodies, eight regime configurations, four household groups per country and seven policy programs. Both clients embed it for offline play; `--content FILE` accepts a validated local replacement. Rules and save format are also version 2. Version-1 packs/saves are rejected without conversion or modification.

## Coverage and identity

The UK, US, France, Germany, Russia, China, India, Japan, Brazil, South Africa, Saudi Arabia, Iran, Cuba, North Korea and Myanmar are selectable. Other countries, disputed/dependent territories, subnational geography, detailed cabinets, treaties, organization memberships, military formations and technology trees are not yet represented. This is not a complete 2000 world database.

Historical country, capital, executive and party names remain a **starter identity curation, not a verified historical database**. Executive denotes the modeled governing authority, not necessarily the constitutional head of state: Jiang Zemin is general secretary, Ali Khamenei supreme leader, Vladimir Putin acting president and Kim Jong Il general secretary. Blair and Jospin are heads of government. Full dated office terms, per-office citations, national electoral laws and independent identity review remain outstanding.

The `identity-notes` source ID records these limitations. Candidate reference families include the [Wikidata office-holder model](https://www.wikidata.org/wiki/Wikidata:WikiProject_Heads_of_state_and_government/Data_Model), national government archives and election authorities in the [world-data specification](../docs/world-data-specification.md). These pointers do not claim a completed import or certification of historical terms. No restricted political dataset is bundled.

The Saudi royal governing council, Iranian clerical/reformist blocs and Myanmar governing military council are scenario governing-body representations, not assertions that they are ordinary parties or historical 100-seat legislatures. Seats, ideology labels and regular election timing are authored. Competitive elections first occur at day 730 and recur every 1460 days; parliamentary confidence can schedule an earlier election. Future cabinets, leadership replacements and decisions are simulated outcomes, with no guarantee that actual later events occur.

## Dated population and GDP observations

Six fields have per-observation provenance: **2000 population and current-USD GDP for GBR, CHN and SAU**. They come from pinned [datasets/population](https://github.com/datasets/population) and [datasets/gdp](https://github.com/datasets/gdp) mirrors. The mirrors identify World Bank population indicator `SP.POP.TOTL` and World Bank/OECD GDP indicator `NY.GDP.MKTP.CD`.

[Reference observations](reference-observations.json) retains original numeric strings, observation periods, units, applied values, transformations, source revisions, full-file SHA-256 hashes, attribution and limitations. The game embeds the same fact records and exposes them in the overview and terminal `data` inspector. Countries with these observations use `quality: mixed_observed_and_authored`; other values retain authored defaults.

**These are published mirror observations, not independently reconciled primary-source facts.** The World Bank API and national sources were not rechecked. Historical revisions can differ substantially. The Saudi population observation in this pinned mirror differs from the earlier authored seed; the source is recorded without claiming an alternative series has been validated. Annual population is a bookmark proxy, not a 1 January census. Annual GDP is truncated from current USD to million USD for the opening 2000 game calibration. No PPP conversion is applied.

Attribution: World Bank; the GDP mirror also credits OECD. GDP mirror metadata declares CC BY 4.0; population mirror metadata declares ODC PDDL. Upstream World Bank-produced data generally uses CC BY 4.0, so upstream attribution/conditions are retained rather than assuming the mirror can waive them. No restricted third-party political data is included.

Rebuild with Python's standard library:

```sh
python3 scripts/import_reference_data.py
```

For offline regeneration, pass `--population FILE --gdp FILE` using the pinned CSVs. Online and offline files must match recorded SHA-256 digests. Missing or duplicate observations fail; absent data never becomes zero. The script updates reference population/GDP, source metadata and facts, leaving other fields intact. Rust validation checks source/country references, observation units and agreement between applied facts and actual seed values. Full raw series are not bundled.

## Authored models and units

**All remaining economic values and coefficients are authored gameplay approximations**, including debt, cash, taxes, spending, capacities, group profiles, political approval, council thresholds, marker coordinates and ideology profiles. The source ID `authored-balance-v1` identifies the original calibration; new household/government definitions are original scenario content. Historical observations initialize the game; they are not causal coefficients.

Money uses millions of constant-2000 USD-equivalent game credits. Currency codes are identity metadata, not exchange-rate behavior. Annual GDP is an opening calibration, not a measured evolving national account. Population uses integer persons. Daily food/energy use modeled units per thousand persons, not empirical tonnes/GWh. Rates use integer percentage points; program capital costs use basis points of opening GDP. Positive divisions truncate; largest-remainder allocation preserves exact group totals and breaks ties by stable definition order. Small economies have visible rounding effects.

Food and energy have distinct nonnegative Rust quantity types across stocks and physical flows. Checked arithmetic rejects overflow and invalid ratios; rational scaling uses a wide intermediate and rounds down. Their JSON representation remains an integer, preserving version-2 snapshots and canonical replay results. Rates, distances and time still require further dimensional typing.

Four disjoint household classifications partition population and cash: working, public-service, transfer-dependent and asset-owning households. Population, income, savings and working-age shares are authored, not measured census or income-distribution observations. Public-service payments go to the public-service group; welfare goes 80% to transfer-dependent and 20% to working households. Private income follows authored shares. Group taxes, purchases, food delivery, employment and arrears are inspectable. Regions, age transitions, skills, complete labor markets and an endogenous price system are not yet implemented.

The external market is a separate counterpart per country, not a network between selectable states. It starts with three times opening GDP in cash, supplies up to 25% of energy needs and 35% of food needs, buys service exports worth up to 15% of realized output and lends subject to cash and debt limits. It can exhaust its funds. This is bounded trade/lending, not complete international finance.

Government definitions specify decision route, backing threshold, consent delay, council threshold and optional leadership-review interval. The UK reference uses 51 parliamentary support and 14-day consent; China uses elite/cadre backing 55/50 and 14-day consent; Saudi Arabia uses elite/council backing 50/45, immediate decrees and a council condition on large spending increases. These and their review intervals are authored institutional mechanisms, not exact constitutional reproductions. See [governing-term scope](../docs/initial-game.md) for consent, reservation, election and succession rules.

The selectable fiscal/supply crisis overrides only the selected country's policy/capacity/approval calibration; it retains the same population/GDP observations. Its scenario ID is saved and replayed. Seven programs cover income tax, services, welfare, energy, food, administration and support negotiations. Metadata specifies parameter bounds, base delays, capital costs and affected groups; Rust owns behavior. Unknown policy behavior is rejected.

## Map asset and original content

`land-110m.json` derives from [Natural Earth 1:110m land](https://github.com/nvkelso/natural-earth-vector/blob/master/geojson/ne_110m_land.geojson), downloaded 3 November 2026. Natural Earth declares its raster/vector data [public domain](https://www.naturalearthdata.com/about/terms-of-use/). Attribution: Natural Earth. The transformation retains polygon outer rings, drops holes and rounds longitude/latitude to three decimals. The client triangulates once at startup. This is geographical land display only: no sovereignty, historical border or territorial claim is asserted. Country markers are authored and do not define control.

The remaining scenario content is original authored game data. No third-party flags, portraits, proprietary game text or restricted political database is included. Dependency licenses are available through the crates referenced in Cargo.lock; release distribution needs the appropriate dependency notices.

## Fictional rules fixture

`synthetic-test.json` is an independent, entirely authored three-country pack: Aurora Republic, Boreal Union and Coral Crown. Their opening economies and household partitions are equal; decision routes, consent delays and council gates differ. All names, institutions, party bodies, statistics and marker locations are fictional test content. The real-land backdrop supplies no fictional borders or sovereignty. No historical observations or external datasets are used.

`python3 scripts/build_synthetic_world.py` produces the pack deterministically from explicit definitions. `--check` compares the committed output without writing. Both clients accept it through `--content`; the terminal also needs `--country AUR` (or BOR/COR). The recovery example accepts `--content data/synthetic-test.json` and tests the same five-year crisis objective through ordinary commands. Synthetic tests establish rule consistency; they do not establish historical accuracy or real-world calibration.
