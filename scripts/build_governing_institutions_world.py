"""Build authored modeled governing institutions, preserving all opening inputs."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = "authored-governing-institutions"
VERSION = "governing-institutions-v1"
ROLES = ["legislature", "party_council", "royal_decree", "religious_council", "military_council", "executive_decree", "consultative_council"]
NAMES = ["national legislature", "party leadership council", "royal authority", "religious council", "military council", "executive authority", "consultative council"]


def build(source: Path, fictional: bool) -> dict:
    raw = source.read_bytes()
    world = copy.deepcopy(json.loads(raw))
    world.update(schema=28, rules=28, id="synthetic-governing-institutions-v28" if fictional else "initial-2000-governing-institutions-v28", name="Fictional governing institutions" if fictional else "A New Century · governing institutions")
    world["description"] += " Authored stable identities for modeled governing routes and submission context. These are gameplay institutions, without claims about historical names, membership, founding dates or votes."
    world["sources"].append({"id": SOURCE, "quality": "Authored institution identity prototype", "description": "Original modeled governing institution identities and route bindings. Preserved input SHA-256 " + hashlib.sha256(raw).hexdigest()})
    institutions = []
    bindings = []
    for country in world["countries"]:
        code = country["id"]
        for role, name in zip(ROLES, NAMES):
            key = f"institution:{code}:{role}"
            institutions.append({"id": key, "country": code, "name": f"Modeled {name} · {country['name']}", "role": role, "source": SOURCE, "source_version": VERSION, "quality": "authored"})
            if role != "consultative_council":
                bindings.append({"country": code, "route": role, "primary": key, "council": f"institution:{code}:consultative_council"})
    world["governing_institutions"] = {"institutions": institutions, "bindings": bindings}
    return world


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for source, target, fictional in [("initial-2000-executive-identities.json", "initial-2000-governing-institutions.json", False), ("synthetic-executive-identities-test.json", "synthetic-governing-institutions-test.json", True)]:
        payload = (json.dumps(build(ROOT / "data" / source, fictional), ensure_ascii=False, indent=2) + "\n").encode()
        path = ROOT / "data" / target
        if args.check:
            if not path.exists() or path.read_bytes() != payload:
                raise SystemExit(f"Generated institution pack differs: {target}")
        else:
            path.write_bytes(payload)
        print(f"{target}: {'verified' if args.check else 'written'} ({len(payload)} bytes)")


if __name__ == "__main__":
    main()
