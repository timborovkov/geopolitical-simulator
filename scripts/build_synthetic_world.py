"""Build an entirely fictional, offline rules fixture; --check detects stale output."""
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    target = ROOT / "data/synthetic-test.json"
    expected = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not target.exists() or target.read_text(encoding="utf-8") != expected:
            raise SystemExit("Synthetic pack is stale: run python3 scripts/build_synthetic_world.py")
        print("Synthetic pack matches its authored definitions.")
    else:
        target.write_text(expected, encoding="utf-8")
        print("Built three fictional governments without historical-source dependencies.")


if __name__ == "__main__":
    main()
