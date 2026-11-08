"""Build authored overlapping-membership exercises from preserved population packs."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = "authored-overlapping-memberships"
VERSION = "overlapping-memberships-v1"
DESCRIPTION = ("Authored cross-sectional joint membership profiles, not historical national prevalence, individual identity histories or records about real residents. Disjoint age/employment scopes prevent double-counting people. The employment membership reproduces existing modeled jobs; membership grants no cash, votes, powers or programme exemptions. Joint shares are explicitly configured, without assuming independent memberships. Parent/carer membership uses the model's 15+ bands, not a claim about legal adulthood. Union membership in this exercise is limited to modeled employed residents; retiree/other union membership and longitudinal joining/leaving remain unfinished. These common assumptions are shared across countries and economic cohorts.")


def cell(key: str, memberships: list[str], share: int) -> dict:
    return {"id": key, "memberships": sorted(memberships), "share": share}


def scopes() -> dict:
    return {
        "young": [cell("unaffiliated", [], 5000), cell("faith", ["faith"], 5000)],
        "working_employed": [cell("job_only", ["employment"], 2000), cell("job_parent", ["employment", "parents"], 2000), cell("job_union", ["employment", "union"], 1500), cell("job_faith", ["employment", "faith"], 1500), cell("job_parent_faith", ["employment", "parents", "faith"], 1000), cell("job_union_faith", ["employment", "union", "faith"], 1000), cell("job_parent_union", ["employment", "parents", "union"], 500), cell("job_parent_union_faith", ["employment", "parents", "union", "faith"], 500)],
        "working_other": [cell("unaffiliated", [], 2500), cell("parents", ["parents"], 2500), cell("faith", ["faith"], 2500), cell("parents_faith", ["parents", "faith"], 2500)],
        "older": [cell("unaffiliated", [], 2000), cell("parents", ["parents"], 3000), cell("faith", ["faith"], 2000), cell("parents_faith", ["parents", "faith"], 3000)],
    }


def build(source: Path, fictional: bool) -> dict:
    raw = source.read_bytes()
    world = copy.deepcopy(json.loads(raw))
    world.update(schema=30, rules=30, id="synthetic-overlapping-memberships-v30" if fictional else "initial-2000-overlapping-memberships-v30", name="Fictional overlapping memberships" if fictional else "A New Century · overlapping social groups")
    world["description"] += " " + DESCRIPTION
    world["sources"].append({"id": SOURCE, "quality": "Authored joint-membership prototype", "description": DESCRIPTION + " Preserved population-pack input SHA-256 " + hashlib.sha256(raw).hexdigest()})
    names = {"employment": "Employed residents", "parents": "Parents and carers (15+ model)", "union": "Union members", "faith": "Faith community members"}
    descriptions = {
        "employment": "All residents holding a modeled job, exactly matching existing paid employment. This grants no additional income or votes.",
        "parents": "An authored parent/carer overlay in the model's 15+ bands, without claims about legal adulthood, individual relationships or national prevalence.",
        "union": "An authored overlay among modeled employed residents. It grants no organisation powers, legal recognition, votes or programme exemptions.",
        "faith": "An authored nonexclusive overlay across age bands, without claims about one unified religious organisation or historical national prevalence.",
    }
    catalogue = {"groups": {key: {"id": key, "name": name, "description": descriptions[key]} for key, name in names.items()}, "eligibility": {"employment": "employed_resident", "parents": "age_fifteen_plus", "union": "employed_resident", "faith": "any_resident"}, "employment_group": "employment"}
    profiles = {country["id"]: {group["id"]: {scope: {"cells": cells} for scope, cells in scopes().items()} for group in country["households"]} for country in world["countries"]}
    world["social_memberships"] = {"source": SOURCE, "source_version": VERSION, "description": DESCRIPTION, "catalogue": catalogue, "profiles": profiles}
    return world


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for source, target, fictional in [("initial-2000-demographic-balances.json", "initial-2000-overlapping-memberships.json", False), ("synthetic-demographic-balances-test.json", "synthetic-overlapping-memberships-test.json", True)]:
        payload = (json.dumps(build(ROOT / "data" / source, fictional), ensure_ascii=False, indent=2) + "\n").encode()
        path = ROOT / "data" / target
        if args.check:
            if not path.exists() or path.read_bytes() != payload:
                raise SystemExit(f"Generated membership pack differs: {target}")
        else:
            path.write_bytes(payload)
        print(f"{target}: {'verified' if args.check else 'written'} ({len(payload)} bytes)")


if __name__ == "__main__":
    main()
