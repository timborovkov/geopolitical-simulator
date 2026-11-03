"""Rebuild test fixtures and the market starter from committed inputs, without network access."""
import argparse
import json
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for filename, pack in [("synthetic-test.json", build()), ("synthetic-dated-test.json", build_temporal()),
                           ("synthetic-market-test.json", build_markets()),
                           ("initial-2000-markets.json", build_market_starter())]:
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
