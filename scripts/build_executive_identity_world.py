"""Build optional live-executive identity packs from preserved statutory inputs."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = "authored-live-executive-identities"
VERSION = "live-executive-identities-v1"


def build(source: Path, fictional: bool) -> dict:
    raw = source.read_bytes()
    world = copy.deepcopy(json.loads(raw))
    world["schema"] = 27
    world["rules"] = 27
    world["id"] = "synthetic-executive-identities-v27" if fictional else "initial-2000-executive-identities-v27"
    world["name"] = "Fictional executive identity continuity" if fictional else "A New Century · live executive identities"
    world["description"] += " Stable live holder references and separately identified simulated successor bodies. Opening people and offices remain source references; future source terms do not force campaign outcomes."
    world["sources"].append({"id": SOURCE, "quality": "Authored identity-continuity prototype", "description": "Original stable holder/body continuity and generated successor identities, without invented historical tenure or membership. Preserved input SHA-256 " + hashlib.sha256(raw).hexdigest()})
    if fictional:
        identities = {"people": [], "offices": [], "terms": [], "currencies": []}
        for country in world["countries"]:
            code = country["id"]
            office = f"office:{code}:governing-reference"
            provenance = {"source": SOURCE, "source_version": VERSION, "quality": "authored"}
            validity = {"valid_from": "2000-01-01", "valid_to": None}
            unknown = code == "BOR"
            title = f"Fictional {code} governing office"
            country["office"] = title
            identities["offices"].append({"id": office, "country": code, "title": title, "role": "council" if code == "COR" else "head_of_government", "required": not unknown, "validity": validity, **provenance})
            if unknown:
                country["executive"] = "Unknown incumbent"
                country.pop("governing_office", None)
            else:
                country["governing_office"] = office
                if code == "COR":
                    country["executive"] = "Fictional Coral stewardship council"
                    holder = {"kind": "collective", "name": country["executive"], "members": []}
                else:
                    person = f"person:{code}:opening-executive"
                    identities["people"].append({"id": person, "name": country["executive"], **provenance})
                    holder = {"kind": "person", "person": person}
                identities["terms"].append({"id": f"term:{code}:opening-executive", "office": office, "holder": holder, "validity": validity, **provenance})
        world["identities"] = identities
    opening_offices = {}
    initial_collectives = {}
    for country in world["countries"]:
        office = country.get("governing_office") or f"office:{country['id']}:governing-reference"
        opening_offices[country["id"]] = office
        term = next((term for term in world["identities"]["terms"] if term["office"] == office and term["validity"]["valid_from"] <= world["start"] and (term["validity"]["valid_to"] is None or world["start"] < term["validity"]["valid_to"])), None)
        if term and term["holder"]["kind"] == "collective":
            initial_collectives[country["id"]] = f"reference:{country['id']}:opening-governing-body"
    world["executive_identities"] = {"opening_offices": opening_offices, "initial_collectives": initial_collectives, "retained_closed_tenures": 128, "source": SOURCE, "source_version": VERSION}
    return world


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for source, target, fictional in [("initial-2000-statutory-authority.json", "initial-2000-executive-identities.json", False), ("synthetic-legislation-test.json", "synthetic-executive-identities-test.json", True)]:
        payload = (json.dumps(build(ROOT / "data" / source, fictional), ensure_ascii=False, indent=2) + "\n").encode()
        path = ROOT / "data" / target
        if args.check:
            if not path.exists() or path.read_bytes() != payload:
                raise SystemExit(f"Generated executive identity pack differs: {target}")
        else:
            path.write_bytes(payload)
        print(f"{target}: {'verified' if args.check else 'written'} ({len(payload)} bytes)")


if __name__ == "__main__":
    main()
