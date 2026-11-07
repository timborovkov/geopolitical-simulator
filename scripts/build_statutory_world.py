"""Build authored statutory-approval fixtures without altering preserved packs."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def build(source: Path, fictional: bool) -> dict:
    raw = source.read_bytes()
    world = copy.deepcopy(json.loads(raw))
    world["schema"] = 26
    world["rules"] = 26
    world["id"] = "synthetic-statutory-authority-v26" if fictional else "initial-2000-statutory-authority-v26"
    world["name"] = "Fictional statutory authority" if fictional else "A New Century · authored statutory authority"
    world["description"] += " Authored statutory prototype: higher new service programmes require a paid, consented and implemented legal authority. This is an original scenario rule, not a historical statement of national legislation."
    source_id = "authored-statutory-authority"
    world["sources"].append({"id": source_id, "quality": "Authored institutional prototype", "description": "Original paid statutory workflow and service-commissioning prerequisite; not historical legal text. Preserved input SHA-256 " + hashlib.sha256(raw).hexdigest()})
    countries = [country["id"] for country in world["countries"]]
    law_id = "high_service_authority"
    world["legislation"] = {
        "definitions": [{"id": law_id, "name": "High-service appropriation authority", "description": "New service programmes at 14% of opening GDP or higher need this enacted permission. Existing operating services are retained. Enactment grants no services, cash or physical capacity; programme funding and implementation are separate.", "available_from": "2000-01-01", "eligible_countries": countries, "source": source_id, "source_version": "authored-statutory-v1"}],
        "policy_requirements": [{"policy": "services", "minimum_value": 14, "laws": [law_id]}],
        "cost_bps": 2,
        "upfront_bps": 2500,
        "implementation_days": 7,
        "cooldown_days": 30,
        "ai_targets": {country: [law_id] for country in countries},
    }
    return world

def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    cases = [("synthetic-statistical-history-test.json", "synthetic-legislation-test.json", True), ("initial-2000-statistical-history.json", "initial-2000-statutory-authority.json", False)]
    for source, target, fictional in cases:
        payload = (json.dumps(build(ROOT / "data" / source, fictional), ensure_ascii=False, indent=2) + "\n").encode()
        path = ROOT / "data" / target
        if args.check:
            if not path.exists() or path.read_bytes() != payload:
                raise SystemExit(f"Generated statutory pack differs: {target}")
        else:
            path.write_bytes(payload)
        print(f"{target}: {'verified' if args.check else 'written'} ({len(payload)} bytes)")

if __name__ == "__main__":
    main()
