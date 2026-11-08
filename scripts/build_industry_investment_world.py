"""Build attributed paid public auxiliary industry investment exercises offline."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = "authored-public-industry-investment"
VERSION = "public-industry-investment-v1"
DESCRIPTION = (
    "Authored public auxiliary equipment investment, not historical industry ownership, national capital stocks or returns. "
    "Each paid programme commissions one country-specific build step after ordinary authority, consent, preparation, construction and actual assembly packages. "
    "Added batches and matching capital become available the following day and depend on previously settled physical maintenance readiness. "
    "Each installed level owes 10 basis points of opening annual GDP per 365 days in maintenance cash, with exact fractional carry, and needs one country maintenance unit per day. "
    "There are at most three additions per sector. These common calibrations do not create jobs, wages or GDP; private investment, profitability, depreciation and closures remain unfinished."
)


def build(name, fictional):
    raw = (ROOT / "data" / name).read_bytes()
    world = json.loads(raw)
    world.update(schema=32, rules=32,
                 id="synthetic-industry-investment-v32" if fictional else "initial-2000-industry-investment-v32",
                 name="Fictional public industry investment" if fictional else "A New Century · public industry investment")
    world["description"] += " " + DESCRIPTION
    world["sources"].append({"id": SOURCE, "quality": "Authored public investment exercise",
                             "description": DESCRIPTION + " Preserved production pack SHA-256 " + hashlib.sha256(raw).hexdigest()})
    production = world["production"]
    policies = {}
    for sector, recipe in sorted(production["framework"]["sectors"].items()):
        policy = "industry_" + sector
        policies[policy] = sector
        world["policies"].append({"id": policy, "name": recipe["name"] + " · capacity investment",
                                  "min": 1, "max": 1, "days": 90, "cost_bps": 100,
                                  "description": "Fund one country-specific auxiliary production build step. Ordinary consent, paid construction and actual assembly packages determine operation. Added equipment works from the following day, needs maintenance and creates no extra wages or GDP.", "groups": "Treasury, auxiliary producers and public service users"})
        production["construction_policy_weights"][policy] = 20
    for portfolio in world["cabinet"]["portfolios"]:
        if "energy" in portfolio["policies"]:
            portfolio["policies"].extend(policies)
    world["industry"] = {"source": SOURCE, "source_version": VERSION, "description": DESCRIPTION,
                         "policies": policies, "maximum_levels": 3, "maintenance_bps_per_level": 10,
                         "countries": {country: {sector: max(1, batches) for sector, batches in opening["installed"].items()}
                                       for country, opening in production["countries"].items()}}
    return world


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for source, target, fictional in [("initial-2000-sector-production.json", "initial-2000-industry-investment.json", False),
                                      ("synthetic-sector-production-test.json", "synthetic-industry-investment-test.json", True)]:
        data = (json.dumps(build(source, fictional), ensure_ascii=False, indent=2) + "\n").encode()
        path = ROOT / "data" / target
        if args.check:
            if not path.exists() or path.read_bytes() != data:
                raise SystemExit(f"{target} differs from its attributed offline inputs")
            print(f"{target}: verified ({len(data)} bytes)")
        else:
            path.write_bytes(data)
            print(f"{target}: wrote {len(data)} bytes")


if __name__ == "__main__":
    main()
