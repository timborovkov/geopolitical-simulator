"""Rebuild test fixtures and the market starter from committed inputs, without network access."""
from copy import deepcopy
import argparse
import json
from import_reference_data import apply_manifest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def build():
    countries = []
    for code, name, regime, route, delay, threshold, council in [
        ("AUR", "Aurora Republic", "parliamentary", "legislature", 14, 51, 0),
        ("BOR", "Boreal Union", "single_party", "party_council", 21, 55, 50),
        ("COR", "Coral Crown", "absolute_monarchy", "royal_decree", 0, 50, 45),
    ]:
        parties = [dict(id=f"{code}-{i}", name=f"{name} body {i}",
                        leader=f"Fictional delegate {code}-{i}", ideology="Authored test profile",
                        seats=seats) for i, seats in [(1, 60), (2, 40)]]
        households = [dict(id=identifier, name=label, population_share=population,
                           income_share=income, savings_share=savings, working_age_share=working)
                      for identifier, label, population, income, savings, working in [
                          ("workers", "Working households", 55, 65, 45, 70),
                          ("public", "Public-service households", 15, 10, 15, 65),
                          ("vulnerable", "Transfer-dependent households", 20, 10, 10, 40),
                          ("owners", "Asset-owning households", 10, 15, 30, 55)]]
        countries.append(dict(
            id=code, name=f"{name} (fictional)", capital=f"{name} test capital",
            regime=regime, executive=f"Fictional delegate {code}-1", office="Test governing office",
            currency="TEST", population=1_000_000, gdp=100_000, tax=24, services=9,
            welfare=4, energy=80, food=80, capacity=70, approval=55,
            longitude={"AUR": 0, "BOR": 40, "COR": 80}[code], latitude=30,
            ruling_party=parties[0]["id"], coalition=[parties[0]["id"]], parties=parties,
            households=households,
            government=dict(route=route, threshold=threshold, consent_days=delay,
                            council_threshold=council,
                            leadership_review_days=None if route == "legislature" else 360),
            quality="authored_synthetic_fixture", identity_source="synthetic-original",
            economic_source="synthetic-original"))
    policies = []
    for identifier, name, minimum, maximum, days, cost, explanation in [
        ("income_tax", "Income tax reform", 10, 45, 30, 2, "Change the wage tax rate."),
        ("services", "Public service budget", 4, 18, 60, 8, "Authorize annual service spending as a GDP percentage."),
        ("welfare", "Household safety net", 0, 12, 30, 3, "Authorize targeted transfers as a GDP percentage."),
        ("energy", "Energy capacity program", 1, 1, 90, 50, "Add 15 energy capacity points after implementation."),
        ("agriculture", "Food resilience program", 1, 1, 120, 35, "Add 15 food capacity points after implementation."),
        ("administration", "Civil service modernization", 1, 1, 180, 40, "Add 8 administration points after implementation."),
        ("coalition", "Negotiate governing support", 1, 1, 45, 10, "Negotiate support through the configured governing route."),
    ]:
        policies.append(dict(id=identifier, name=name, min=minimum, max=maximum,
                             days=days, cost_bps=cost, description=explanation,
                             groups="Fictional household and governing groups"))
    return dict(schema=2, id="synthetic-rules-v2", name="Three fictional governments",
                start="2000-01-01", description="Entirely authored rules fixture, not historical data. "
                "Identical opening economies isolate parliamentary, party and royal authority. "
                "Map markers are arbitrary test locations and assert no sovereignty.",
                sources=[dict(id="synthetic-original", quality="authored_synthetic_fixture",
                              description="Original fictional content for repeatable rules tests; no historical claims.")],
                facts=[], countries=countries, policies=policies)


def build_temporal():
    pack = build()
    pack.update(id="synthetic-dated-v2", name="Dated fictional institutions", start="2001-07-15",
                description="Entirely fictional date-effective office/currency fixture. Historical reference terms initialize the bookmark; future records do not force campaign transitions. A one-day reference handover makes boundary behavior inspectable. Statistics and institutions are authored, not historical observations.")
    pack["countries"][0].update(governing_office="office:aurora:government",
                                executive="Fictional Steward B", office="First Minister of Aurora", currency="AUT")
    def period(start, end=None):
        return dict(valid_from=start, valid_to=end)
    def provenance(**fields):
        return dict(**fields, source="synthetic-original", source_version="fictional-records-v1", quality="authored")
    offices = [provenance(id=f"office:aurora:{identifier}", country="AUR", title=title, role=role,
                          required=required, validity=period("1990-01-01"))
               for identifier, title, role, required in [
                   ("government", "First Minister of Aurora", "head_of_government", True),
                   ("state", "Aurora Council", "head_of_state", True),
                   ("speaker", "Assembly Speaker", "legislature", True),
                   ("auditor", "Independent Auditor", "independent_institution", False)]]
    terms = [provenance(id=f"term:aurora:{identifier}", office="office:aurora:government",
                        holder=dict(kind="person", person=f"person:aurora:{identifier}"), validity=validity)
             for identifier, validity in [
                 ("a", period("1990-01-01", "2001-07-15")),
                 ("b", period("2001-07-15", "2001-07-16")),
                 ("c", period("2001-07-16"))]]
    terms.extend([
        provenance(id="term:aurora:council", office="office:aurora:state",
                   holder=dict(kind="collective", name="Fictional Aurora Council",
                               members=["person:aurora:a", "person:aurora:c"]), validity=period("1990-01-01")),
        provenance(id="term:aurora:speaker-vacancy", office="office:aurora:speaker",
                   holder=dict(kind="vacant"), validity=period("1990-01-01")),
    ])
    pack["identities"] = dict(
        people=[provenance(id=f"person:aurora:{identifier}", name=f"Fictional Steward {identifier.upper()}")
                for identifier in ["a", "b", "c"]],
        offices=offices, terms=terms,
        currencies=[provenance(id=f"currency:aurora:{identifier}", country="AUR", code=code, validity=validity)
                    for identifier, code, validity in [
                        ("old", "AUT", period("1990-01-01", "2001-07-16")),
                        ("new", "NEW-AUT", period("2001-07-16"))]])
    return pack


def build_markets():
    pack = build()
    pack.update(schema=3, id="synthetic-markets-v3", rules=3, name="Fictional relative-price markets",
                description="Entirely authored rules-3 market fixture, not historical data. Food and energy retain distinct physical quantities. Relative prices adjust after settlement and apply the next day; food reprices part of essential household baskets, and both prices change import bills. This is not a complete sector, labor, inflation or banking model.")
    pack["markets"] = dict(food_budget_share_bps=4000, source="synthetic-original",
                           source_version="authored-market-v1", goods=[
        dict(id=identifier, kind=kind, name=name, unit=unit,
             minimum_price_bps=5000, maximum_price_bps=20000, adjustment_bps=250,
             import_limit_bps=imports, storage_days=storage, loss_bps=loss)
        for identifier, kind, name, unit, imports, storage, loss in [
            ("test:power", "energy", "Fictional energy", "modeled energy unit", 2500, 3, 0),
            ("test:staple", "food", "Fictional essential food", "modeled food-basket unit", 3500, 11, 100)]])
    return pack


def build_finance():
    pack = build_markets()
    pack.update(schema=4, rules=4, id="synthetic-finance-v4", name="Fictional borrowing authority",
                description="Entirely authored financing-authority prototype. Ordinary borrowing needs institutional consent; pending requests are not cash. Contested emergency borrowing requires effective council backing and carries political costs. Debt remains fixed-rate 4% perpetual debt in constant-2000 reporting credits, without local-currency FX, banks or bond markets.")
    pack["finance"] = dict(source="synthetic-original", source_version="authored-finance-v1",
        authorities=[dict(id=f"borrowing:{route}", label=label, route=route, consent_days=days)
                     for route, label, days in [
                         ("legislature", "Legislative borrowing consent", 14),
                         ("party_council", "Party council borrowing consent", 21),
                         ("royal_decree", "Royal borrowing decree", 0),
                         ("religious_council", "Religious council borrowing consent", 30),
                         ("military_council", "Military council borrowing consent", 7),
                         ("executive_decree", "Executive borrowing decree", 0)]],
        emergency=dict(council_threshold=65, approval_cost=15, legislative_cost=20,
                       elite_cost=10, council_cost=20, cooldown_days=180))
    return pack


def build_constitutions():
    pack = build_finance()
    pack.update(schema=5, rules=5, id="synthetic-constitutions-v5", name="Fictional constitutional transitions",
                description="Entirely authored political-transition prototype. Lawful supermajority routes and explicitly contested effective-power routes can change a country's governing constitution in either direction. Costs, reserved cash, backing, sponsorship, political consequences and inertia constrain changes. Templates, thresholds and AI preferences are fictional; this does not reproduce historical constitutions, military coercion, independent institutions or complete electoral laws.")
    templates = []
    for regime, route, title, threshold, council, consent, review in [
        ("parliamentary", "legislature", "parliamentary cabinet", 51, 0, 14, None),
        ("presidential", "legislature", "presidential cabinet", 51, 0, 21, None),
        ("semi_presidential", "legislature", "semi-presidential cabinet", 51, 0, 21, None),
        ("single_party", "party_council", "party council", 55, 50, 21, 180),
        ("absolute_monarchy", "royal_decree", "royal court", 50, 45, 0, 90),
        ("theocracy", "religious_council", "religious council", 55, 55, 30, 180),
        ("personalist", "executive_decree", "executive council", 50, 45, 0, 90),
        ("military", "military_council", "military council", 55, 55, 7, 90)]:
        templates.append(dict(regime=regime, title=title, government=dict(route=route,
            threshold=threshold, council_threshold=council, consent_days=consent, leadership_review_days=review)))
    pack["constitutions"] = dict(templates=templates, source="synthetic-original", source_version="authored-transitions-v1",
        cooldown_days=180, ai_targets={"BOR": "parliamentary", "COR": "single_party"},
        lawful=dict(days=90, cost_bps=50, upfront_bps=2000, legislative_threshold=67, elite_threshold=75,
                    council_threshold=65, approval_threshold=55, approval_cost=3,
                    legislative_cost=5, elite_cost=10, council_cost=5),
        contested=dict(days=14, cost_bps=100, upfront_bps=10000, legislative_threshold=0, elite_threshold=80,
                       council_threshold=80, approval_threshold=0, approval_cost=20,
                       legislative_cost=25, elite_cost=15, council_cost=20))
    return pack


def build_commitments():
    pack = build_constitutions()
    pack.update(schema=6, rules=6, id="synthetic-commitments-v6", name="Fictional policy lifecycle",
                description="Authored commitment lifecycle fixture: paid amendments restart consent, withdrawal releases only unspent reservations, and operating programmes retain direct changes and observed outcome reviews. These are game assumptions, not historical legislative or contracting procedures.")
    pack["commitments"] = dict(amendment_fee_bps=1000, maximum_amendments=3, evaluation_days=30,
                               source="synthetic-original", source_version="authored-commitments-v1")
    return pack


def cabinet_rules(countries, source="synthetic-original", source_version="authored-cabinet-v1"):
    portfolios = []
    candidates = []
    for country in countries:
        for key, title, policies in [("finance", "Finance ministry", ["income_tax", "services", "welfare"]),
                                     ("supply", "Supply ministry", ["energy", "agriculture"]),
                                     ("administration", "Cabinet administration", ["administration"])]:
            office = f"portfolio:{country['id']}:{key}"
            portfolios.append(dict(id=office, country=country["id"], title=title, policies=policies, minimum_skill=50))
            for kind, skill, acceptability in [("professional",85,40),("consensus",55,85)]:
                candidates.append(dict(person=f"generated:{country['id']}:{key}:{kind}", office=office,
                    name=f"Generated {country['id']} {key} {kind} nominee", skill=skill, acceptability=acceptability))
    return dict(portfolios=portfolios, candidates=candidates, appointment_cost_bps=2,
                           dismissal_cost_bps=1, cooldown_days=90, vacancy_preparation_days=30,
                           source=source, source_version=source_version)


def build_cabinet():
    pack = build_commitments()
    pack.update(schema=7, rules=7, id="synthetic-cabinet-v7", name="Fictional ministerial appointments",
                description="Authored ministerial portfolios and generated candidates. Appointments require current authority and cabinet continuity; fees and political costs constrain choices. Skill affects preparation of new proposals only. Initial unassigned portfolios use permanent civil service; no claim about historical vacancies or ministerial careers is made.")
    pack["cabinet"] = cabinet_rules(pack["countries"])
    return pack


def build_elections():
    pack = build_cabinet()
    pack.update(schema=8, rules=8, id="synthetic-elections-v8", name="Fictional electoral authorities",
                description="Authored independent electoral authority mandates. Governments may request an early poll only within configured powers and intervals. Announced polls cannot be withdrawn by government order or used to postpone an earlier legal poll. Polls retain the simplified 100-seat model; no national electoral law, turnout or integrity certification is claimed.")
    boreal = next(c for c in pack["countries"] if c["id"] == "BOR")
    boreal.update(regime="presidential", government=dict(route="legislature", threshold=51,
                 council_threshold=0, consent_days=21, leadership_review_days=None))
    pack["elections"] = dict(mandates=[dict(authority=f"authority:{c['id']}:elections", country=c["id"],
                             title=f"{c['id']} electoral authority", snap_forms=["parliamentary", "semi_presidential"])
                             for c in pack["countries"]], decision_days=7, campaign_days=60,
                             minimum_interval_days=365, regular_term_years=4, administration_cost_bps=5,
                             source="synthetic-original", source_version="authored-electoral-mandates-v1")
    return pack


def build_subnational():
    pack = build_elections()
    pack.update(schema=9, rules=9, id="synthetic-subnational-v9", name="Fictional territorial charters",
                description="Authored non-overlapping budget jurisdictions, tax entitlements and autonomous/delegated service powers. No historical regional geography, local tax rates, staffing or physical service capacity is claimed. Protected local public subaccounts remain part of consolidated treasury. Local base purchases are limited by available local funds; funded supplements retain their own paid terms.")
    charters = []
    for country in pack["countries"]:
        code = country["id"]
        charters.append(dict(country=code, form={"AUR":"decentralized_unitary", "BOR":"federation", "COR":"delegated_unitary"}[code],
            regions=[dict(id=f"region:{code}:{key}", authority=f"authority:{code}:{key}", name=f"Fictional {code} {name}",
                          tier="autonomous_jurisdiction" if key=="province" and code!="COR" else "municipality" if key=="city" else "region",
                          population_share_bps=population, tax_entitlement_bps=tax, autonomous_services=code=="BOR" or code=="AUR" and key=="province", service_level=2)
                     for key,name,population,tax in [("province","provincial authority",6000,1500),("city","municipal authority",4000,500)]]))
    pack["subnational"] = dict(charters=charters, grant_review_days=14, cooldown_days=90,
                               administration_cost_bps=2, absorption_days=180,
                               source="synthetic-original", source_version="authored-territorial-charters-v1")
    return pack


def build_territorial():
    pack = build_subnational()
    pack.update(schema=10, rules=10, id="synthetic-territorial-v10", name="Fictional territorial reforms",
                description="Authored territorial reform, local consent/compensation and contested base-fund requisition. Consolidated public money is conserved. Regional voting, coercive forces and historical territorial law are not modeled.")
    pack["territorial"] = dict(
        templates=[dict(form=form, tax_pool_bps=share) for form,share in
                   [("federation",3000),("decentralized_unitary",2000),("delegated_unitary",1000)]],
        lawful=pack["constitutions"]["lawful"].copy(), contested=pack["constitutions"]["contested"].copy(),
        cooldown_days=180, local_consent_approval=65, compensation_bps=50,
        requisition=dict(administration_cost_bps=25, elite_threshold=80, council_threshold=80,
                         approval_cost=20, legislative_cost=20, elite_cost=15, council_cost=20, cooldown_days=180),
        ai_targets={"AUR":"federation","BOR":"decentralized_unitary","COR":"decentralized_unitary"},
        source="synthetic-original", source_version="authored-territorial-reforms-v1")
    return pack


def build_undertakings():
    pack = build_territorial()
    pack.update(schema=11, rules=11, id="synthetic-undertakings-v11", name="Fictional delivery undertakings",
                description="Authored public delivery pledges, governing support agreements and deadlines attached to actual policy projects. Declaring a promise neither orders its policy nor supplies its resources. Shared commitment milestones and policy delivery records determine fulfilment; explicit deadlines, political consequences, credibility, fees and cabinet sponsorship apply. Full election campaigns, coalition formation, commercial contracts and treaties are not claimed.")
    zero = dict(approval=0, legislative=0, elite=0, council=0)
    pack["undertakings"] = dict(cooldown_days=180, maximum_open=3, source="synthetic-original", source_version="authored-undertakings-v1",
        templates=[
            dict(id="energy-pledge", name="Public energy delivery pledge", kind="public_pledge", policy="energy", direction="at_least", delta=15,
                 review_days=7, deadline_days=180, administration_cost_bps=2, minimum_credibility=0, elite_threshold=0, council_threshold=0,
                 acceptance=dict(zero,approval=3), breach=dict(zero,approval=-8,council=-3)),
            dict(id="service-agreement", name="Governing support for a service commitment", kind="support_agreement", policy="services", direction="at_least", delta=1,
                 review_days=14, deadline_days=180, administration_cost_bps=5, minimum_credibility=40, elite_threshold=30, council_threshold=30,
                 acceptance=dict(approval=0,legislative=12,elite=12,council=12), breach=dict(approval=-5,legislative=-18,elite=-18,council=-18)),
            dict(id="project-deadline", name="Public deadline for an existing policy project", kind="project_deadline", policy=None, direction="at_least", delta=0,
                 review_days=7, deadline_days=180, administration_cost_bps=1, minimum_credibility=0, elite_threshold=0, council_threshold=0,
                 acceptance=zero.copy(), breach=dict(zero,approval=-3))])
    return pack


def build_fiscal_plans():
    pack = build_undertakings()
    pack.update(schema=12, rules=12, id="synthetic-fiscal-plans-v12", name="Fictional annual and supplementary budgets",
                description="Authored combined fiscal proposals with institutional review, administration costs and fully reserved policy implementation. Enacted annual or supplementary plans fund actual tax/service/welfare programmes; delivery dates, daily cash execution and conditional forecasts remain distinct. This is not historical budget law or a hard appropriation ceiling.")
    pack["fiscal_plans"] = dict(review_days=14, cooldown_days=30, annual_window_days=180, administration_cost_bps=3,
                               source="synthetic-original", source_version="authored-fiscal-plans-v1")
    return pack


def build_income_bands():
    pack = build_fiscal_plans()
    pack.update(schema=13, rules=13, id="synthetic-income-bands-v13", name="Fictional progressive private-income bands",
                description="Authored progressive withholding on actually received private income, using a representative resident within each cohort and 365-day annualisation relative to opening GDP per resident. The existing income-tax policy sets a standard marginal reference rate; source-defined bands scale it. Public/local wages and welfare are outside this private-income base. Collection capacity affects actual withholding; potential tax is not treasury cash or an arrears claim. No historical tax filing law, intra-cohort distribution or other tax bases are claimed.")
    pack["income_taxes"] = dict(annualisation_days=365, bands=[
        dict(id="exempt",lower_gdp_per_resident_bps=0,rate_multiplier_bps=0),
        dict(id="standard",lower_gdp_per_resident_bps=1000,rate_multiplier_bps=10000),
        dict(id="upper",lower_gdp_per_resident_bps=7500,rate_multiplier_bps=15000),
        dict(id="top",lower_gdp_per_resident_bps=15000,rate_multiplier_bps=20000)],
        source="synthetic-original", source_version="authored-private-income-bands-v1")
    tax=next(p for p in pack["policies"] if p["id"]=="income_tax")
    tax["name"]="Standard private-income marginal rate"
    tax["description"]="Set the standard marginal reference rate for authored private-income bands. Withholding uses actual private income, representative cohort means and collection capacity; benefits and public/local wages remain outside this base. Full tax filing law and other bases are not modeled."
    return pack


def build_market_starter():
    pack = json.loads((ROOT / "data/initial-2000.json").read_text(encoding="utf-8"))
    pack.update(schema=3, rules=3, id="initial-2000-markets-v3",
                name="A New Century · relative-price starter")
    pack["description"] += " Relative food/energy prices are authored rules-3 model assumptions, not sourced historical prices. The original fixed-price pack is retained for classic saves."
    source = "authored-markets-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model",
                                description="Original relative-price, import, storage and basket-share assumptions. No historical price-series calibration is claimed."))
    pack["markets"] = build_markets()["markets"]
    pack["markets"].update(source=source, source_version="relative-prices-v1")
    for good in pack["markets"]["goods"]:
        good["id"] = good["kind"]
        good["name"] = "Energy" if good["kind"] == "energy" else "Essential food"
    return pack


def build_finance_starter():
    pack = build_market_starter()
    pack.update(schema=4, rules=4, id="initial-2000-finance-v4",
                name="A New Century · institutions and markets")
    pack["description"] += " Borrowing authority, consent delays and emergency political costs are authored rules-4 assumptions, not a reproduction of historical debt law. Earlier market and classic packs remain available for their original campaigns."
    source = "authored-finance-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model",
                                description="Original regime-route borrowing consent, fixed-rate debt and contested emergency assumptions. No historical constitutional or debt-market calibration is claimed."))
    pack["finance"] = build_finance()["finance"]
    pack["finance"].update(source=source, source_version="borrowing-authority-v1")
    return pack


def build_constitution_starter():
    pack = build_finance_starter()
    pack.update(schema=5, rules=5, id="initial-2000-constitutions-v5",
                name="A New Century · political transitions")
    pack["description"] += " Constitutional transitions use authored rules-5 templates, backing thresholds, costs and inertia rather than sourced historical amendment law. Starter AI prefers its opening governing form; fictional fixtures demonstrate alternative preferences. Earlier campaigns retain their original packs."
    source = "authored-constitutions-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model",
                                description="Original lawful and contested constitutional-transition templates, political costs, consent constraints and status-quo AI preferences. These are game assumptions, not historical constitutional procedures."))
    pack["constitutions"] = build_constitutions()["constitutions"]
    pack["constitutions"].update(source=source, source_version="constitutional-transitions-v1",
                                ai_targets={country["id"]: country["regime"] for country in pack["countries"]})
    return pack


def build_commitment_starter():
    pack = build_constitution_starter()
    pack.update(schema=6, rules=6, id="initial-2000-commitments-v6", name="A New Century · policy commitments")
    pack["description"] += " Policy amendments, withdrawal, history and observed reviews use authored rules-6 procedures. Existing campaigns retain their exact original rules."
    source = "authored-commitments-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model",
                                description="Original amendment administration costs, per-proposal limits and observed-review intervals. These are game assumptions, not historical legislative or contracting rules."))
    pack["commitments"] = build_commitments()["commitments"]
    pack["commitments"].update(source=source, source_version="policy-commitments-v1")
    return pack


def build_cabinet_starter():
    pack = build_commitment_starter()
    pack.update(schema=7, rules=7, id="initial-2000-cabinet-v7", name="A New Century · cabinet government")
    pack["description"] += " Ministerial portfolios, generated nominees and preparation rules are authored gameplay definitions, not historical ministry rosters or vacancy claims. Unassigned portfolios use permanent civil service. Earlier campaigns retain their exact original rules."
    source = "authored-cabinet-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model",
                                description="Original functional portfolios, generated candidate profiles, eligibility, appointment/dismissal costs and preparation delays. No historical ministerial staffing or career claims."))
    pack["cabinet"] = cabinet_rules(pack["countries"], source, "ministerial-portfolios-v1")
    return pack


def build_election_starter():
    pack = build_cabinet_starter()
    pack.update(schema=8, rules=8, id="initial-2000-elections-v8", name="A New Century · electoral mandates")
    pack["description"] += " Independent electoral mandates, early-poll powers and calendar terms are authored gameplay assumptions, not historical national election law. Earlier campaigns retain their exact original rules."
    source = "authored-elections-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model",
                                description="Original independent electoral mandates, early-poll jurisdiction, review/campaign intervals, administration costs and calendar terms. No historical poll records, national electoral law or integrity certification is claimed."))
    pack["elections"] = build_elections()["elections"]
    pack["elections"].update(source=source, source_version="electoral-mandates-v1",
        mandates=[dict(authority=f"authority:{c['id']}:elections", country=c["id"],
                       title=f"{c['id']} electoral authority", snap_forms=["parliamentary", "semi_presidential"])
                  for c in pack["countries"]])
    return pack


def build_observed_starter():
    pack = build_election_starter()
    pack.update(id="initial-2000-observed-v8", name="A New Century · expanded observations")
    pack["description"] += " Population observations cover all 15 starter countries and GDP covers 14, from the pinned World Bank mirrors. North Korean GDP retains its explicit authored calibration. Annual observations are bookmark proxies; primary sources have not been independently reconciled."
    manifest = json.loads((ROOT / "data/starter-observations.json").read_text(encoding="utf-8"))
    return apply_manifest(pack, manifest)


def build_territorial_starter():
    pack = build_observed_starter()
    pack.update(schema=10, rules=10, id="initial-2000-territorial-v10", name="A New Century · territorial government")
    pack["description"] += " National/local budget charters and territorial procedures are authored gameplay abstractions. The two disjoint budget jurisdictions per country are population partitions, not historical administrative units, mapped boundaries or regional electorates. Local purchase levels, collected-tax shares, autonomy templates, compensation and consent proxies are original model assumptions."
    source = "authored-territorial-government-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model",
                                description="Original two-jurisdiction budget partitions, local purchasing levels, autonomy/tax-share templates, grant consent, lawful compensation and contested territorial/requisition costs. No historical territorial law, administrative geography, regional voting or local fiscal calibration is claimed."))
    federal = {"USA", "BRA", "DEU", "IND", "RUS"}
    charters = []
    for country in pack["countries"]:
        code = country["id"]
        form = "federation" if code in federal else "decentralized_unitary" if country["regime"] in {"parliamentary","presidential","semi_presidential"} else "delegated_unitary"
        pool = {"federation":3000,"decentralized_unitary":2000,"delegated_unitary":1000}[form]
        charters.append(dict(country=code, form=form,
            regions=[dict(id=f"region:{code}:{key}", authority=f"authority:{code}:{key}",
                          name=f"Authored {code} {label} budget authority", tier=tier,
                          population_share_bps=population, tax_entitlement_bps=pool*population//10000,
                          autonomous_services=form=="federation" or form=="decentralized_unitary" and key=="regional", service_level=2)
                     for key,label,tier,population in [("regional","regional","region",6000),("municipal","municipal","municipality",4000)]]))
    pack["subnational"] = build_subnational()["subnational"]
    pack["subnational"].update(charters=charters, source=source, source_version="starter-budget-jurisdictions-v1")
    pack["territorial"] = build_territorial()["territorial"]
    pack["territorial"].update(source=source, source_version="starter-territorial-procedures-v1",
                               ai_targets={charter["country"]:charter["form"] for charter in charters})
    return pack


def build_undertaking_starter():
    pack = build_territorial_starter()
    pack.update(schema=11, rules=11, id="initial-2000-undertakings-v11", name="A New Century · delivery undertakings")
    pack["description"] += " Public delivery pledges, governing support arrangements and linked project deadlines use authored procedures and credibility records, not historical campaign promises or party agreements. They do not supply resources or form cabinet coalitions. Existing policy authority and implementation constraints apply."
    source = "authored-delivery-undertakings-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model",
                                description="Original public delivery pledges, conditional governing-support arrangements, linked programme deadlines, political effects, administration costs, fixed review/interval rules and record-based credibility. No historical election promises, party agreements, commercial contracts or treaties are claimed."))
    pack["undertakings"] = build_undertakings()["undertakings"]
    pack["undertakings"].update(source=source, source_version="starter-delivery-undertakings-v1")
    # Full-year linked deadlines accommodate the roster's longer administration programmes.
    next(t for t in pack["undertakings"]["templates"] if t["kind"]=="project_deadline")["deadline_days"] = 365
    return pack


def build_fiscal_starter():
    pack = build_undertaking_starter()
    pack.update(schema=12, rules=12, id="initial-2000-fiscal-plans-v12", name="A New Century · annual and supplementary budgets")
    pack["description"] += " Reviewed annual and supplementary fiscal packages use authored preparation, administration, implementation reservation and calendar accounting procedures. Enactment funds actual policy programmes; recurring purchasing remains subject to cash and delivery constraints. These plans are not historical budget laws or hard appropriation ceilings."
    source = "authored-fiscal-plans-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model",
                                description="Original annual/supplementary fiscal preparation, institutional review, administration costs, implementation reservations and calendar execution. No historical budget law, tax-base calibration or hard appropriation ceiling is claimed."))
    pack["fiscal_plans"] = build_fiscal_plans()["fiscal_plans"]
    pack["fiscal_plans"].update(source=source, source_version="starter-fiscal-plans-v1")
    return pack



def build_income_starter():
    pack = build_fiscal_starter()
    pack.update(schema=13, rules=13, id="initial-2000-income-bands-v13", name="A New Century · progressive private-income bands")
    pack["description"] += " Progressive private-income withholding uses authored representative-cohort bands relative to opening GDP per resident, not historical filing law or within-cohort distributions. Public/local service payments and welfare remain outside this base. Potential tax is not treasury cash or arrears."
    source = "authored-private-income-bands-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model", description="Original representative-cohort private-income bands, GDP-per-resident thresholds, marginal rate factors and fixed 365-day annualisation. No historical tax filing law, individual distributions or national tax-base calibration is claimed."))
    authored = build_income_bands()
    pack["income_taxes"] = authored["income_taxes"]
    pack["income_taxes"].update(source=source, source_version="starter-private-income-bands-v1")
    tax = next(p for p in pack["policies"] if p["id"] == "income_tax")
    definition = next(p for p in authored["policies"] if p["id"] == "income_tax")
    tax.update(name=definition["name"], description=definition["description"])
    return pack



def build_fiscal_ai():
    pack = build_income_bands()
    pack.update(schema=14, rules=14, id="synthetic-national-fiscal-ai-v14", name="Fictional national fiscal reviews")
    pack["description"] += " Fiscal AI reviews use the national daily balance after protected local claims, while earlier rules retain their original consolidated trigger. Authority, available cash, fees, review and real programme delivery remain ordinary shared constraints. This is an authored bounded heuristic, not historical government behavior or optimal economic planning."
    return pack


def build_fiscal_ai_starter():
    pack = build_income_starter()
    pack.update(schema=14, rules=14, id="initial-2000-fiscal-ai-v14", name="A New Century · national fiscal reviews")
    pack["description"] += " Nonplayer fiscal reviews use the national daily balance after protected local tax claims. A consolidated surplus cannot finance national obligations from protected local cash. Budget authority, submission funding, review and actual programme delivery remain ordinary shared constraints. This is an authored bounded heuristic, not historical government behavior or optimal economic planning."
    pack["sources"].append(dict(id="authored-national-fiscal-ai-v1", quality="authored_scenario_model", description="Original national-funding review objective for the bounded fiscal AI. It respects protected local claims and shared legal/cash/implementation rules; it does not claim historical government decisions, privileged revenue or optimal planning."))
    return pack


def build_debt_maturities():
    pack = build_fiscal_ai()
    pack.update(schema=15, rules=15, id="synthetic-debt-maturities-v15", name="Fictional dated loan principal")
    pack["description"] += " Newly disbursed loans have an authored 180-day bullet maturity. Opening debt remains a perpetual calibration, not a historical issuance. Scheduled principal uses available national cash after protected local claims and reservations, before interest and national purchases. Unpaid matured principal remains outstanding without being duplicated as operating arrears. Separate ordinary borrowing can fund repayment but is not guaranteed refinancing. No historical maturity curve, default/restructuring, FX or market pricing is claimed."
    pack["finance"].update(maturity_days=180, source_version="authored-dated-loans-v1")
    return pack


def build_cash_management():
    pack = build_debt_maturities()
    pack.update(schema=16, rules=16, id="synthetic-cash-management-v16", name="Fictional temporary cash retention")
    source = "authored-temporary-cash-retention-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model", description="Original bounded temporary protection of actual national cash for a future decision. Deferring operating payments creates arrears; unpaid principal remains debt. This is not a historical cash-management or legal-priority claim."))
    pack["cash_management"] = dict(max_target_bps=100, max_days=90, cooldown_days=90, approval_cost=3, source=source, source_version="authored-retention-v1")
    pack["description"] += " Temporary cash retention protects only actual national cash after capital/local reservations, up to 1% opening GDP for at most 90 days. Approval falls by three points; a 90-day reuse cooldown applies from creation. Release, expiry or government change ends protection without cancelling any obligation or granting spending authority."
    return pack



def build_interest_accrual():
    pack = build_cash_management()
    pack.update(schema=17, rules=17, id="synthetic-interest-accrual-v17", name="Fictional carried debt interest")
    source = "authored-interest-carry-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model", description="Original fixed 400-basis-point actual/365 principal-day accrual. Fractional reporting credits carry until a whole unit becomes due. No historical debt pricing, currency precision, compounding or legal settlement convention is claimed."))
    pack["finance"].update(interest_accrual="actual_365_carry", source=source, source_version="authored-interest-carry-v1")
    pack["description"] += " Debt interest accrues at fixed 4% actual/365 on outstanding principal after scheduled principal settlement. Fractions carry without compounding; only whole reporting units become due. Cash payments and unpaid whole-unit external arrears reconcile separately. A sub-unit remainder survives retirement of principal and is not cash or spending authority. Taxes, wages, purchases and prices still use whole million-credit units."
    return pack



def build_technology_eligibility():
    pack = build_interest_accrual()
    pack.update(schema=18, rules=18, id="synthetic-technology-eligibility-v18", name="Fictional dated programme eligibility")
    source = "authored-technology-eligibility-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model", description="Original fictional scenario prerequisites for programme commissioning. Dates and eligible countries are authored restrictions, not historical scientific discovery, adoption, research progress or future innovation predictions."))
    countries = [c["id"] for c in pack["countries"]]
    pack["technologies"] = dict(definitions=[dict(id="grid_planning", name="Grid planning", available_from="2000-01-31", eligible_countries=countries, source=source, source_version="fictional-eligibility-v1"), dict(id="agronomy", name="Agronomic planning", available_from="1990-01-01", eligible_countries=countries, source=source, source_version="fictional-eligibility-v1")], policy_requirements=[dict(policy="energy", technologies=["grid_planning"]), dict(policy="agriculture", technologies=["agronomy"])])
    pack["description"] += " Grid programme commissioning is unavailable before 2000-01-31 in this fictional scenario. Agronomic planning is already eligible. Eligibility permits an ordinary proposal; it grants no research, adopted technology, cash, capacity or shortened construction. All prerequisites, governing backing, full funding and delivery remain necessary. Full research and adoption dynamics remain outside this scenario."
    return pack


def build_debt_starter():
    pack = build_fiscal_ai_starter()
    pack.update(schema=15, rules=15, id="initial-2000-debt-maturities-v15", name="A New Century · dated loan principal")
    pack["description"] += " New loan principal matures an authored 365 days after actual disbursement. Opening debt remains perpetual calibration; historical debt curves and actual refinancing markets are not claimed."
    source = "authored-dated-loan-principal-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model", description="Original 365-day maturity and scheduled-principal settlement for new loans. Existing authority and emergency constraints are retained; this is not historical debt calibration or default/restructuring."))
    pack["finance"].update(source=source, source_version="starter-financing-with-dated-principal-v1", maturity_days=365)
    return pack


def build_cash_starter():
    pack = build_debt_starter()
    pack.update(schema=16, rules=16, id="initial-2000-cash-management-v16", name="A New Century · treasury cash management")
    source = "authored-starter-cash-retention-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model", description="Original temporary retention of actual national cash after protected local and capital claims. Deferred operating payments remain arrears; unpaid principal remains debt. Uniform authored gameplay procedure, not historical cash-management law or payment priority."))
    pack["cash_management"] = dict(max_target_bps=100, max_days=90, cooldown_days=90, approval_cost=3, source=source, source_version="starter-retention-v1")
    pack["description"] += " Temporary cash retention can protect actual national funds for a future decision, up to 1% opening GDP for 90 days, with approval cost and reuse cooldown. Unfunded targets are not cash. Release, expiry or a successor government ends protection without cancelling obligations or granting spending authority."
    return pack



def build_sovereignty():
    pack = build_technology_eligibility()
    pack.update(schema=19, rules=19, id="synthetic-sovereignty-v19", name="Fictional sovereignty and recognition references")
    source = "authored-sovereignty-reference-v1"
    pack["sources"].append(dict(id=source, quality="authored_synthetic_fixture", description="Original fictional territory claims, administration, occupation and bilateral recognition. Separate from government, regime, map outlines and campaign outcomes; not historical or legal assertions."))
    def provenance(**fields):
        return dict(**fields, source=source, source_version="fictional-reference-v1", quality="authored")
    def period(start="1990-01-01", end=None):
        return dict(valid_from=start, valid_to=end)
    territories = [provenance(id=identifier, name=name) for identifier, name in [("aurora_home", "Aurora mainland"), ("boreal_home", "Boreal mainland"), ("coral_home", "Coral mainland"), ("strait", "Fictional Strait Islands"), ("unrecorded", "Unrecorded test territory")]]
    references = [provenance(id=identifier+"_opening", territory=identifier, validity=period(), sovereignty=dict(kind="sovereign", country=country), control=dict(kind="civil_administration", country=country)) for identifier,country in [("aurora_home","AUR"),("boreal_home","BOR"),("coral_home","COR")]]
    references += [provenance(id="strait_opening", territory="strait", validity=period(end="2001-01-01"), sovereignty=dict(kind="sovereign", country="AUR"), control=dict(kind="occupation", occupier="BOR")), provenance(id="strait_later", territory="strait", validity=period("2001-01-01"), sovereignty=dict(kind="disputed", claimants=["AUR","BOR"]), control=dict(kind="civil_administration", country="COR")), provenance(id="unrecorded_opening", territory="unrecorded", validity=period(), sovereignty=dict(kind="unknown"), control=dict(kind="unknown"))]
    recognition = [provenance(id="cor_aur", observer="COR", subject="AUR", position="recognized", validity=period()), provenance(id="bor_aur_opening", observer="BOR", subject="AUR", position="not_recognized", validity=period(end="2001-01-01")), provenance(id="bor_aur_later", observer="BOR", subject="AUR", position="recognized", validity=period("2001-01-01")), provenance(id="aur_bor", observer="AUR", subject="BOR", position="unknown", validity=period())]
    pack["sovereignty"] = dict(territories=territories, references=references, recognition=recognition)
    pack["description"] += " Fictional legal sovereignty, occupation, civil administration, disputed claims and directional recognition are independently attributed bookmark references. Future records select later bookmarks, never force a campaign transfer. Unknown records are explicit; absent recognition means unknown. No military, diplomatic, revenue, border or map-geometry effects are granted."
    return pack


def build_party_preferences():
    pack = deepcopy(build_sovereignty())
    pack.update(schema=20, rules=20, id="synthetic-party-preferences-v20", name="Fictional governing-party policy preferences")
    source = "authored-party-policy-preferences-v1"
    pack["sources"].append(dict(id=source, quality="authored_synthetic_fixture", description="Original fictional economic policy positions and bounded governing-party alignment pressure. No historical party platforms, roll-call votes, ideology-label bonuses or additional legal powers are asserted."))
    boreal = next(c for c in pack["countries"] if c["id"] == "BOR")
    template = next(t for t in pack["constitutions"]["templates"] if t["regime"] == "single_party")
    boreal.update(regime="single_party", government=deepcopy(template["government"]), office=template["title"])
    profiles = []
    for country in pack["countries"]:
        for party in country["parties"]:
            governing = party["id"] == country["ruling_party"]
            ranges = [(24, 30), (7, 11), (3, 6)] if governing else [(10, 18), (4, 8), (0, 3)]
            unknown = party["id"] == "BOR-2"
            positions = [dict(policy=policy, importance=importance, preferred=None if unknown or (party["id"] == "COR-2" and policy == "welfare") else dict(min=bounds[0], max=bounds[1])) for policy, importance, bounds in zip(["income_tax", "services", "welfare"], [3, 2, 2], ranges)]
            profiles.append(dict(country=country["id"], party=party["id"], positions=positions, source=source, source_version="fictional-positions-v1", quality="unknown" if unknown else "authored"))
    influences = [dict(country=country, legislative_max_penalty=legislative, party_leadership_max_penalty=leadership) for country, legislative, leadership in [("AUR", 20, 0), ("BOR", 20, 15), ("COR", 0, 0)]]
    pack["party_preferences"] = dict(profiles=profiles, influences=influences, source=source, source_version="fictional-influence-v1")
    pack["description"] += " Economic preferences use explicit fiscal target ranges and importance weights, independently of ideology labels. Operating policy distance can lower the backing target for a governing coalition or ruling-party leadership, with gradual existing political reviews. Unknown positions create no pressure and remain identified. Aurora uses parliamentary coalition seats; Boreal begins under party-council leadership; Coral's royal route has advisory positions only. Positions grant no cash, votes, consent exemption or extra powers. Other preference dimensions and party organization remain unfinished."
    return pack


def build_interest_starter():
    pack = build_cash_starter()
    pack.update(schema=17, rules=17, id="initial-2000-interest-accrual-v17", name="A New Century · carried debt interest")
    source = "authored-starter-interest-carry-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model", description="Original fixed 4% actual/365 principal-day accrual with carried fractional reporting units. Whole interest due, actual cash payments and unpaid whole-unit arrears reconcile separately. This is uniform gameplay accounting, not historical debt pricing, compounding, currency precision or legal settlement law."))
    pack["finance"].update(interest_accrual="actual_365_carry", source=source, source_version="starter-interest-carry-v1")
    pack["description"] += " Debt interest accrues at fixed 4% actual/365 on actual outstanding principal after scheduled redemption. Fractions carry without compounding until a whole reporting unit becomes due; only paid amounts transfer cash. Unpaid whole interest remains external arrears, while the unsettled sub-unit creditor claim survives principal retirement without becoming cash or spending authority. Tax, wage, purchase and price rounding remain whole reporting units."
    return pack



def build_technology_starter():
    pack = build_interest_starter()
    pack.update(schema=18, rules=18, id="initial-2000-technology-eligibility-v18", name="A New Century · programme technology prerequisites")
    source = "authored-starter-technology-eligibility-v1"
    pack["sources"].append(dict(id=source, quality="authored_scenario_model", description="Original baseline programme eligibility for the 2000 bookmark. All seeded countries may commission the existing grid and agriculture programmes from the bookmark, subject to ordinary funding and governing authority. Dates denote scenario permissions, not historical scientific discovery, knowledge adoption or research completion."))
    countries = [c["id"] for c in pack["countries"]]
    pack["technologies"] = dict(definitions=[dict(id="grid_planning", name="Grid planning", available_from="2000-01-01", eligible_countries=countries, source=source, source_version="starter-eligibility-v1"), dict(id="agronomy", name="Agronomic planning", available_from="2000-01-01", eligible_countries=countries, source=source, source_version="starter-eligibility-v1")], policy_requirements=[dict(policy="energy", technologies=["grid_planning"]), dict(policy="agriculture", technologies=["agronomy"])])
    pack["description"] += " Existing grid and agriculture programmes have explicitly authored technology eligibility for every seeded country from the 2000 bookmark. Eligibility grants no research, adoption, cash, capacity or faster construction; ordinary authority, funding and delivery remain necessary. These dates are scenario permissions, not historical discovery or adoption records."
    return pack


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for filename, pack in [("synthetic-test.json", build()), ("synthetic-dated-test.json", build_temporal()),
                           ("synthetic-market-test.json", build_markets()),
                           ("synthetic-finance-test.json", build_finance()),
                           ("synthetic-constitution-test.json", build_constitutions()),
                           ("synthetic-commitment-test.json", build_commitments()),
                           ("synthetic-cabinet-test.json", build_cabinet()),
                           ("synthetic-election-test.json", build_elections()),
                           ("synthetic-subnational-test.json", build_subnational()),
                           ("synthetic-territorial-test.json", build_territorial()),
                           ("synthetic-undertakings-test.json", build_undertakings()),
                           ("synthetic-fiscal-plans-test.json", build_fiscal_plans()),
                           ("synthetic-income-bands-test.json", build_income_bands()),
                           ("synthetic-national-fiscal-ai-test.json", build_fiscal_ai()),
                           ("synthetic-debt-maturities-test.json", build_debt_maturities()),
                           ("synthetic-cash-management-test.json", build_cash_management()),
                           ("synthetic-interest-accrual-test.json", build_interest_accrual()),
                           ("synthetic-technology-eligibility-test.json", build_technology_eligibility()),
                           ("synthetic-sovereignty-test.json", build_sovereignty()),
                           ("synthetic-party-preferences-test.json", build_party_preferences()),
                           ("initial-2000-markets.json", build_market_starter()),
                           ("initial-2000-finance.json", build_finance_starter()),
                           ("initial-2000-constitutions.json", build_constitution_starter()),
                           ("initial-2000-commitments.json", build_commitment_starter()),
                           ("initial-2000-elections.json", build_election_starter()),
                           ("initial-2000-observed.json", build_observed_starter()),
                           ("initial-2000-territorial.json", build_territorial_starter()),
                           ("initial-2000-undertakings.json", build_undertaking_starter()),
                           ("initial-2000-fiscal-plans.json", build_fiscal_starter()),
                           ("initial-2000-income-bands.json", build_income_starter()),
                           ("initial-2000-fiscal-ai.json", build_fiscal_ai_starter()),
                           ("initial-2000-debt-maturities.json", build_debt_starter()),
                           ("initial-2000-cash-management.json", build_cash_starter()),
                           ("initial-2000-interest-accrual.json", build_interest_starter()),
                           ("initial-2000-technology-eligibility.json", build_technology_starter()),
                           ("initial-2000-cabinet.json", build_cabinet_starter())]:
        target = ROOT / "data" / filename
        expected = json.dumps(pack, ensure_ascii=False, indent=2) + "\n"
        if args.check:
            if not target.exists() or target.read_text(encoding="utf-8") != expected:
                raise SystemExit(f"{filename} is stale: run python3 scripts/build_synthetic_world.py")
            print(f"{filename} matches its committed build inputs.")
        else:
            target.write_text(expected, encoding="utf-8", newline="\n")
            print(f"Built {filename} from committed inputs without network access.")


if __name__ == "__main__":
    main()
