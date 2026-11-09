"""Build attributed bounded public auxiliary-stock trade exercises offline."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = "authored-industrial-trade"
VERSION = "industrial-trade-v1"
DESCRIPTION = (
    "Authored public auxiliary-inventory import and export contracts, not historical national trade volumes or private firm profits. "
    "Prices quote basis points of opening annual GDP per ten country-specific production lots. Ask prices exceed bids. "
    "Import contracts reserve their full quoted cash; all orders pay a nonrefundable one-basis-point administration fee. "
    "Ordinary authority and consent precede at most 30 daily fills. Imports arrive after production and before storage, and can support later construction delivery. "
    "Exports protect current domestic service use and packages needed by funded construction due that day. "
    "External supply and demand have separate per-good daily limits; exports also require actual counterparty cash. No bank credit, cash, wages or GDP is created. "
    "Purchase payments round up and sale receipts down on cumulative delivered quantities. Small interim fills can round to zero whole credits. "
    "Imported stocks remain subject to the same storage losses. Cancellation retains sunk administration, goods already delivered and payments already settled. "
    "Closed contracts remain in campaign history; only simultaneous open orders are limited. "
    "Export receipts are recorded separately from recurring tax revenue and are not forecast as guaranteed income. Bilateral trade, private ownership and profit-led firms remain unfinished."
)
PRICES = {"biomass": (2, 1), "industrial_fuel": (4, 3), "raw_materials": (3, 2),
          "tools": (5, 4), "construction_materials": (5, 4), "assembly_packages": (10, 9)}


def build(name, fictional):
    raw = (ROOT / "data" / name).read_bytes()
    world = json.loads(raw)
    world.update(schema=34, rules=34,
                 id="synthetic-industrial-trade-v34" if fictional else "initial-2000-industrial-trade-v34",
                 name="Fictional industrial trade" if fictional else "A New Century · industrial inputs and trade")
    world["description"] += " " + DESCRIPTION
    world["sources"].append({"id": SOURCE, "quality": "Authored bounded industrial trade exercise",
                             "description": DESCRIPTION + " Preserved lifecycle pack SHA-256 " + hashlib.sha256(raw).hexdigest()})
    world["industrial_trade"] = {"source": SOURCE, "source_version": VERSION, "description": DESCRIPTION,
                                  "order_days": 30, "fee_bps": 1, "price_lots": 10, "maximum_orders": 128,
                                  "goods": {good: {"ask_bps": ask, "bid_bps": bid, "import_lots_per_day": 5, "export_lots_per_day": 1}
                                            for good, (ask, bid) in sorted(PRICES.items())}}
    return world


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for source, target, fictional in [("initial-2000-industry-lifecycle.json", "initial-2000-industrial-trade.json", False),
                                      ("synthetic-industry-lifecycle-test.json", "synthetic-industrial-trade-test.json", True)]:
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
