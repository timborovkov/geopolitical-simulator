"""Build attributed paid public industry retirement exercises from preserved inputs."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = "authored-public-industry-retirement"
VERSION = "public-industry-retirement-v1"
DESCRIPTION = (
    "Authored public auxiliary equipment retirement, not historical closures, private profitability or salvage values. "
    "A paid 20 basis point opening-GDP programme, with at least 30 days preparation and ordinary consent, retires one previously commissioned addition. "
    "Capacity and new maintenance obligations fall from the following day. Opening equipment cannot be retired. "
    "Sunk investment costs, existing arrears and fractional creditor claims remain; there are no cash proceeds or returned construction packages. "
    "Three additions per sector may operate simultaneously; retired additions may be replaced through ordinary paid investment. "
    "These common calibrations are gameplay assumptions, not national engineering estimates. Private firms, profits, depreciation and insolvency remain unfinished."
)


def build(name, fictional):
    raw = (ROOT / "data" / name).read_bytes()
    world = json.loads(raw)
    world.update(schema=33, rules=33,
                 id="synthetic-industry-lifecycle-v33" if fictional else "initial-2000-industry-lifecycle-v33",
                 name="Fictional public industry lifecycle" if fictional else "A New Century · public industry lifecycle")
    world["description"] += " " + DESCRIPTION
    world["sources"].append({"id": SOURCE, "quality": "Authored public retirement exercise",
                             "description": DESCRIPTION + " Preserved investment pack SHA-256 " + hashlib.sha256(raw).hexdigest()})
    policies = {}
    for sector, recipe in sorted(world["production"]["framework"]["sectors"].items()):
        policy = "retire_industry_" + sector
        policies[policy] = sector
        world["policies"].append({"id": policy, "name": recipe["name"] + " · retire added capacity",
                                  "min": 1, "max": 1, "days": 30, "cost_bps": 20,
                                  "description": "Pay to retire one commissioned auxiliary equipment addition after ordinary consent and preparation. Capacity and maintenance fall the following day. No cash proceeds, refunds or returned packages; existing claims remain due. Opening equipment is protected.",
                                  "groups": "Treasury, auxiliary producers and public service users"})
    for portfolio in world["cabinet"]["portfolios"]:
        if "energy" in portfolio["policies"]:
            portfolio["policies"].extend(policies)
    world["industry"]["retirement"] = {"source": SOURCE, "source_version": VERSION,
                                         "description": DESCRIPTION, "policies": policies}
    world["industry"]["description"] += " " + DESCRIPTION
    return world


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for source, target, fictional in [("initial-2000-industry-investment.json", "initial-2000-industry-lifecycle.json", False),
                                      ("synthetic-industry-investment-test.json", "synthetic-industry-lifecycle-test.json", True)]:
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
