"""Build optional authored demographic exercises; never invent historical vital statistics."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = "authored-demographic-balances"
VERSION = "demographic-balances-v1"


def build(source: Path, fictional: bool) -> dict:
    raw = source.read_bytes()
    world = copy.deepcopy(json.loads(raw))
    world.update(schema=29, rules=29, id="synthetic-demographic-balances-v29" if fictional else "initial-2000-demographic-balances-v29", name="Fictional demographic balances" if fictional else "A New Century · authored population change")
    description = ("Optional authored demographic exercise: every country uses the same coarse birth, mortality, age-split, family-formation and statistical-capacity assumptions. These are not historical national vital statistics, census quality rankings or migration estimates. Aggregate household units create no dwellings or wealth. Working age is not employment. Production and import capacity remain tied to opening physical need; new residents increase civilian demand without creating capacity. GDP remains an opening calibration. Migration moves residents between country/cohort stocks, without transferring pooled group cash, assets or arrears. Pooled claims and already-funded local financial obligations remain payable even if a cohort becomes empty; those transfers do not certify staffing or actual service delivery. Regional placement, housing constraints, individual labor mobility, inheritance and endogenous fertility remain unfinished.")
    world["description"] += " " + description
    world["sources"].append({"id": SOURCE, "quality": "Authored demographic and measurement prototype", "description": description + " Preserved governing-pack input SHA-256 " + hashlib.sha256(raw).hexdigest()})
    profiles = {}
    for country in world["countries"]:
        profiles[country["id"]] = {group["id"]: {
            "rates": {"birth_per_working_age_bps": 300, "young_mortality_bps": 50, "working_mortality_bps": 60, "older_mortality_bps": 450, "household_target_bps": 6500, "household_formation_bps": 800, "household_dissolution_bps": 600},
            "young_share_of_dependants_bps": 6000, "initial_households_per_person_bps": 3000,
        } for group in country["households"]}
    # Bilateral equal flows exercise conservation, not a historical prediction.
    pairs = [("AUR", "BOR", 120)] if fictional else [("GBR", "USA", 5000), ("FRA", "DEU", 3000)]
    corridors = []
    for first, second, annual in pairs:
        for origin, destination in [(first, second), (second, first)]:
            corridors.append({"id": f"authored-{origin.lower()}-{destination.lower()}", "origin": {"country": origin, "group": "workers"}, "destination": {"country": destination, "group": "workers"}, "annual_people": annual})
    world["demographics"] = {"source": SOURCE, "source_version": VERSION, "description": description, "profiles": profiles, "corridors": corridors, "statistical_capacities": {c["id"]: 90 for c in world["countries"]}, "census_days": 30, "maximum_capacity": 400}
    return world


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for source, target, fictional in [("initial-2000-governing-institutions.json", "initial-2000-demographic-balances.json", False), ("synthetic-governing-institutions-test.json", "synthetic-demographic-balances-test.json", True)]:
        payload = (json.dumps(build(ROOT / "data" / source, fictional), ensure_ascii=False, indent=2) + "\n").encode()
        path = ROOT / "data" / target
        if args.check:
            if not path.exists() or path.read_bytes() != payload:
                raise SystemExit(f"Generated demographic pack differs: {target}")
        else:
            path.write_bytes(payload)
        print(f"{target}: {'verified' if args.check else 'written'} ({len(payload)} bytes)")


if __name__ == "__main__":
    main()
