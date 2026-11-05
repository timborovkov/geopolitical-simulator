"""Compile reference subdivisions from pinned Natural Earth originals, without game authority.

Uses only Python's standard library. --source-dir accepts offline SHP/DBF/PRJ/CPG.
--validate checks the committed assets offline and never downloads or writes files.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
from urllib.request import urlopen

from import_country_geometry import inside, polygons, records, serialize, shapes

ROOT = Path(__file__).resolve().parents[1]
REVISION = "aa15d45ca649047105c9fd38c5d58e95ada27fca"
NAME = "ne_10m_admin_1_states_provinces"
BASE = f"https://raw.githubusercontent.com/nvkelso/natural-earth-vector/{REVISION}/10m_cultural/{NAME}"
DIGESTS = {
    "shp": "29314ae535d16b011b195b79aec19c77f8a5787ac5b340f5aa46e526aaaa95a3",
    "dbf": "112acb875c76f17f7675025bdca1e737714a534aff2f77c8ced10922106d3c14",
    "prj": "a02a27b1d1982c8516d83398e85a3c8b1aef1713c13ef4d84d7bde17430c07c4",
    "cpg": "3ad3031f5503a4404af825262ee8232cc04d4ea6683d42c5dd0a2f2a27ac9824",
}
ASSET = ROOT / "data/administrative-references-10m.json"
MANIFEST = ROOT / "data/administrative-references-manifest.json"


def roster():
    return {country["id"] for country in json.loads(
        (ROOT / "data/initial-2000.json").read_text(encoding="utf-8"))["countries"]}


def contains(feature, point):
    return any(inside(polygon[0], point) and not any(inside(hole, point) for hole in polygon[1:])
               for polygon in feature["polygons"])


def check_features(features):
    countries = roster()
    ids = set()
    for feature in features:
        if (not isinstance(feature["id"], str) or not feature["id"] or feature["id"] in ids or feature["country"] not in countries
                or not isinstance(feature["name"], str) or not isinstance(feature["alternate_name"], str)
                or not isinstance(feature["kind"], str) or not feature["polygons"]):
            raise ValueError("Duplicate or incomplete administrative reference")
        ids.add(feature["id"])
        for polygon in feature["polygons"]:
            if not polygon:
                raise ValueError("Missing subdivision exterior")
            for ring in polygon:
                if len(ring) < 4 or ring[0] != ring[-1]:
                    raise ValueError("Subdivision ring is not closed")
                for x, y in ring:
                    if not (-180 <= x <= 180 and -90 <= y <= 90):
                        raise ValueError("Subdivision coordinate outside WGS84")
                if any(abs(a[0] - b[0]) > 180 for a, b in zip(ring, ring[1:])):
                    raise ValueError("Unsplit subdivision antimeridian edge")
    counts = dict(sorted(Counter(feature["country"] for feature in features).items()))
    if counts.keys() != countries:
        raise ValueError("Subdivision reference coverage differs from starter roster")
    if [feature["id"] for feature in features] != sorted(ids):
        raise ValueError("Subdivision references must be ordered by stable source ID")
    return counts


def build(raw):
    if raw["cpg"].decode("ascii") != "UTF-8" or b"WGS_1984" not in raw["prj"]:
        raise ValueError("Expected attributed UTF-8 / WGS84 source")
    attributes = records(raw["dbf"], encoding="utf-8")
    geometry = shapes(raw["shp"])
    if len(attributes) != len(geometry):
        raise ValueError("Subdivision attributes and geometry have different row counts")
    countries = roster()
    features = []
    for index, (row, rings) in enumerate(zip(attributes, geometry), 1):
        if row["adm0_a3"] not in countries:
            continue
        features.append(dict(id=row["adm1_code"], country=row["adm0_a3"], name=row["name"],
                             alternate_name=row["name_alt"], kind=row["type_en"], source_row=index,
                             polygons=polygons(rings)))
    features.sort(key=lambda feature: feature["id"])
    counts = check_features(features)
    compiled = dict(subdivisions=features)
    manifest = dict(
        id="starter-administrative-references-v1",
        source="Natural Earth admin-1 states/provinces 1:10m, v3.3.0",
        revision=REVISION, retrieved_at="2026-11-05", source_sha256=DIGESTS,
        source_urls={ext: f"{BASE}.{ext}" for ext in DIGESTS},
        license="Public domain", license_url="https://www.naturalearthdata.com/about/terms-of-use/",
        attribution="Natural Earth",
        creator="Natural Earth contributors; game compiler in scripts/import_administrative_geometry.py",
        transformation="Select the 15 starter ADM0_A3 records; keep ADM1_CODE identities, attributed UTF-8 names, separate polygon parts and holes; round WGS84 longitude/latitude to 0.001 degrees. No administrative powers or statistical values are inferred.",
        applicable_dates="Reference source release v3.3.0; administrative coverage is not certified for the campaign's historical bookmark.",
        limitations="Geographic references are separate from scenario-defined governing jurisdictions, sovereignty, control and recognition. Coverage and administrative levels reflect this source release. Eight source records have no name or type and remain explicitly unnamed reference areas. Country outlines and land use a different generalized scale. Small features remain accessible through reference search. A dateline-spanning feature focuses its largest contiguous part.",
        country_counts=counts, feature_count=len(features),
        unnamed_count=sum(not feature["name"] for feature in features),
        review_checks="Known samples cover Westminster, Bayern, São Paulo, Alaska, both Chukchi dateline parts, Corsica and French Guiana. Lesotho, Ireland, Crimea and Taiwan do not acquire a starter-country subdivision. These samples do not certify all historical boundaries.",
        compiled_sha256=hashlib.sha256(serialize(compiled).encode("utf-8")).hexdigest(),
    )
    return compiled, manifest


def validate_compiled():
    raw = ASSET.read_bytes()
    compiled = json.loads(raw)
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if (hashlib.sha256(raw).hexdigest() != manifest["compiled_sha256"]
            or manifest["source_sha256"] != DIGESTS or manifest["revision"] != REVISION
            or manifest["source_urls"] != {ext: f"{BASE}.{ext}" for ext in DIGESTS}):
        raise ValueError("Subdivision bytes or provenance differ from pinned inputs")
    counts = check_features(compiled["subdivisions"])
    if (counts != manifest["country_counts"] or len(compiled["subdivisions"]) != manifest["feature_count"]
            or sum(not feature["name"] for feature in compiled["subdivisions"]) != manifest["unnamed_count"]):
        raise ValueError("Subdivision count differs from attributed coverage")
    print(f"Administrative references OK: {manifest['feature_count']} subdivisions, {len(counts)} countries; checksum, provenance and rings validated.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", type=Path)
    parser.add_argument("--validate", action="store_true")
    args = parser.parse_args()
    if args.validate:
        validate_compiled()
        return
    raw = {}
    for ext, digest in DIGESTS.items():
        if args.source_dir:
            raw[ext] = (args.source_dir / f"{NAME}.{ext}").read_bytes()
        else:
            with urlopen(f"{BASE}.{ext}", timeout=30) as response:
                raw[ext] = response.read()
        if hashlib.sha256(raw[ext]).hexdigest() != digest:
            raise ValueError(f"{ext}: administrative input differs from pinned source")
    compiled, manifest = build(raw)
    for path, value in [(ASSET, compiled), (MANIFEST, manifest)]:
        path.write_text(serialize(value), encoding="utf-8", newline="\n")
        print(f"Built {path.relative_to(ROOT)} without changing scenario definitions.")


if __name__ == "__main__":
    main()
