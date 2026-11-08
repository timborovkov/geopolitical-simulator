"""Build authored auxiliary infrastructure networks without replacing civilian goods."""
import argparse
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = "authored-auxiliary-production"
VERSION = "auxiliary-production-v1"
DESCRIPTION = (
    "Authored auxiliary infrastructure network, not historical national sector output, deposits, asset holdings or productivity observations. "
    "Agriculture supplies industrial biomass; extraction supplies finite-deposit feedstocks and industrial fuel; manufacturing supplies tools and materials; "
    "construction supplies storable prefabricated assembly packages, not stored worker-hours; services supply nonstorable contracted inputs. "
    "These goods are distinct from national civilian Food/Energy. Paid programmes consume packages and funded public/maintenance services require actual inputs. "
    "A bounded share of live employed workers is reserved for this network and limited by actual private income paid; remaining workers belong to the older abstract civilian economy. "
    "Common stock targets, recipes, productivity and opening maintained-equipment readiness are scenario assumptions. Broader labor, private investment and product markets remain unfinished."
)


def recipe(name, industry, priority, inputs, outputs):
    return {"name": name, "industry": industry, "priority": priority, "labor_per_batch": 1,
            "capital_per_batch": {"equipment": 1}, "inputs_per_batch": inputs, "outputs_per_batch": outputs}


def employed(country):
    groups = country["households"]
    total = sum(g["population_share"] for g in groups)
    numerators = [country["population"] * g["population_share"] for g in groups]
    counts = [n // total for n in numerators]
    order = sorted(range(len(groups)), key=lambda i: -(numerators[i] % total))
    for i in order[:country["population"] - sum(counts)]:
        counts[i] += 1
    return sum(n * g["working_age_share"] * 90 // 10000 for n, g in zip(counts, groups))


def build(path, fictional):
    raw = path.read_bytes()
    world = copy.deepcopy(json.loads(raw))
    world.update(schema=31, rules=31,
                 id="synthetic-sector-production-v31" if fictional else "initial-2000-sector-production-v31",
                 name="Fictional infrastructure production" if fictional else "A New Century · infrastructure production")
    world["description"] += " " + DESCRIPTION
    world["sources"].append({"id": SOURCE, "quality": "Authored auxiliary production prototype",
                             "description": DESCRIPTION + " Preserved social-membership pack SHA-256 " + hashlib.sha256(raw).hexdigest()})
    names = {"deposits": "Mineral feedstock deposits", "biomass": "Industrial biomass", "industrial_fuel": "Industrial fuel",
             "raw_materials": "Raw materials", "tools": "Consumable tool supplies", "construction_materials": "Construction materials",
             "assembly_packages": "Prefabricated assembly packages", "contracted_services": "Contracted service inputs"}
    goods = {key: {"name": name, "unit": "authored " + key.replace("_", " ") + " unit"} for key, name in names.items()}
    sectors = {
        "agriculture": recipe("Industrial biomass cultivation", "agriculture", 3, {"industrial_fuel": 1, "tools": 1}, {"biomass": 10}),
        "extraction": recipe("Feedstock extraction", "extraction", 1, {"deposits": 1, "industrial_fuel": 1, "tools": 1}, {"raw_materials": 10, "industrial_fuel": 10}),
        "manufacturing": recipe("Tools and material manufacture", "manufacturing", 2, {"biomass": 1, "raw_materials": 1, "industrial_fuel": 1}, {"tools": 10, "construction_materials": 10}),
        "construction": recipe("Prefabricated package construction", "construction", 4, {"construction_materials": 1, "tools": 1, "industrial_fuel": 1}, {"assembly_packages": 10}),
        "services": recipe("Contracted public and maintenance inputs", "services", 0, {"tools": 1, "industrial_fuel": 1}, {"contracted_services": 10}),
    }
    countries = {}
    for country in world["countries"]:
        need = max(1, country["population"] // 1000)
        unit = max(1, (need + 999) // 1000)
        installed = {key: unit for key in sectors}
        installed["services"] = (unit * 35 + 9) // 10
        stocks = {key: unit * 200 for key in names}
        stocks["deposits"] = need * 36525 * 10
        stocks["assembly_packages"] = stocks["contracted_services"] = 0
        storage = {key: {"capacity": unit * 300, "loss_bps": 0} for key in names}
        storage["deposits"]["capacity"] = stocks["deposits"]
        storage["contracted_services"]["capacity"] = 0
        storage["biomass"]["loss_bps"] = 100
        storage["tools"]["loss_bps"] = 50
        countries[country["id"]] = {
            "labor_requirement_multiplier": max(1, employed(country) // (sum(installed.values()) * 10)),
            "installed": installed, "capital": {"equipment": sum(installed.values())}, "stocks": stocks, "storage": storage,
            "service_units_per_percent": unit, "maintenance_units_per_step": unit, "construction_unit": unit,
        }
    framework = {"source": SOURCE, "source_version": VERSION, "description": DESCRIPTION,
                 "goods": goods, "capital_kinds": ["equipment"], "sectors": sectors}
    world["production"] = {"source": SOURCE, "source_version": VERSION, "description": DESCRIPTION,
                           "framework": framework, "assembly_good": "assembly_packages", "service_good": "contracted_services",
                           "service_sector": "services", "worker_share": 2000,
                           "construction_policy_weights": {"energy": 100, "agriculture": 80, "administration": 40}, "countries": countries}
    return world


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    for source, target, fictional in [("initial-2000-overlapping-memberships.json", "initial-2000-sector-production.json", False),
                                      ("synthetic-overlapping-memberships-test.json", "synthetic-sector-production-test.json", True)]:
        payload = (json.dumps(build(ROOT / "data" / source, fictional), ensure_ascii=False, indent=2) + "\n").encode()
        path = ROOT / "data" / target
        if args.check:
            if not path.exists() or path.read_bytes() != payload:
                raise SystemExit(f"Generated auxiliary production pack differs: {target}")
        else:
            path.write_bytes(payload)
        print(f"{target}: {'verified' if args.check else 'written'} ({len(payload)} bytes)")


if __name__ == "__main__":
    main()
