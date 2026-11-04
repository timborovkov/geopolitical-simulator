"""Compile pinned Natural Earth reference lakes/rivers; no simulation authority or water resources.

Offline rebuild: --source-dir DIRECTORY containing the six pinned SHP/DBF/PRJ files.
--validate checks committed assets without downloads; --check verifies exact offline regeneration.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
from urllib.request import urlopen
from import_country_geometry import records, shapes, polygons, serialize

ROOT = Path(__file__).resolve().parents[1]
REVISION = "aa15d45ca649047105c9fd38c5d58e95ada27fca"
RETRIEVED = "2026-11-04"
NAMES = {"lakes": "ne_110m_lakes", "rivers": "ne_110m_rivers_lake_centerlines"}
DIGESTS = {
    "ne_110m_lakes.shp": "d0a540ac74d9d9526f61933e95a1ab6508ae3aa56bd79fc851a303c60948dfb7",
    "ne_110m_lakes.dbf": "54ba29061b5d3a6d2d09dd029a1750bfeac91af73dd5ee76d675cc2380c8a9ec",
    "ne_110m_lakes.prj": "3259f0e55290a82b1350646f604e8a7ee1e2136c0320a40fad838ab40819fff8",
    "ne_110m_rivers_lake_centerlines.shp": "e23ed42728885cc3acf00d48d904ee1b75cdd3e76b0a6b8b15a11f520a500a01",
    "ne_110m_rivers_lake_centerlines.dbf": "67768a414536347a3428c03d11d4bdb20fe0cee7d1bb0d8278cae513cee0988e",
    "ne_110m_rivers_lake_centerlines.prj": "98aaf3d1c0ecadf1a424a4536de261c3daf4e373697cb86c40c43b989daf52eb",
}
URLS = {name: f"https://raw.githubusercontent.com/nvkelso/natural-earth-vector/{REVISION}/110m_physical/{name}" for name in DIGESTS}


def label(raw):
    return " ".join(raw.replace("\\r", " ").replace("\\n", " ").split())


def validate_points(points, minimum, closed=False):
    if not isinstance(points, list) or len(points) < minimum or (closed and points[0] != points[-1]):
        raise ValueError("Missing points or unclosed lake ring")
    for p in points:
        if not isinstance(p, list) or len(p) != 2 or not all(type(v) in (int, float) and math.isfinite(v) for v in p):
            raise ValueError("Invalid physical-feature coordinate")
        if not (-180.001 <= p[0] <= 180.001 and -90 <= p[1] <= 90):
            raise ValueError("Physical-feature coordinate out of range")
    if any(abs(a[0] - b[0]) > 180.001 for a, b in zip(points, points[1:])):
        raise ValueError("Unsplit physical-feature antimeridian edge")


def validate_geometry(data):
    if set(data) != {"lakes", "rivers"} or len(data["lakes"]) != 25 or len(data["rivers"]) != 14:
        raise ValueError("Physical-feature counts differ from the pinned source")
    for kind, features in data.items():
        for index, feature in enumerate(features, 1):
            if feature["id"] != f"{kind[:-1]}-{index:04}" or feature["source_row"] != index or not feature["name"].strip() or not 0 <= feature["scalerank"] <= 10:
                raise ValueError("Invalid physical-feature identity, name or rank")
            if kind == "lakes":
                if not feature["polygons"]:
                    raise ValueError("Lake has no polygons")
                for polygon in feature["polygons"]:
                    if not polygon:
                        raise ValueError("Lake has no exterior ring")
                    for ring in polygon:
                        validate_points(ring, 4, closed=True)
            else:
                if not feature["parts"]:
                    raise ValueError("River has no parts")
                for part in feature["parts"]:
                    validate_points(part, 2)


def compile_geometry(raw):
    if set(raw) != set(DIGESTS):
        raise ValueError("All six pinned input files are required")
    for name, digest in DIGESTS.items():
        if hashlib.sha256(raw[name]).hexdigest() != digest:
            raise ValueError(f"Unpinned input bytes: {name}")
    result = {}
    for kind, name in NAMES.items():
        projection = raw[name + ".prj"].decode("ascii")
        if not projection.startswith('GEOGCS["GCS_WGS_1984"'):
            raise ValueError("Expected WGS84 physical-feature coordinates")
        attributes = records(raw[name + ".dbf"])
        geometry = shapes(raw[name + ".shp"], 5 if kind == "lakes" else 3)
        if len(attributes) != len(geometry):
            raise ValueError("Physical attributes/geometry count mismatch")
        features = []
        for index, (row, parts) in enumerate(zip(attributes, geometry), 1):
            if row["featurecla"] != ("Lake" if kind == "lakes" else "River"):
                raise ValueError("Unexpected physical-feature class")
            feature = dict(id=f"{kind[:-1]}-{index:04}", source_row=index, name=label(row["name"]), alternate_name=label(row["name_alt"]), scalerank=int(row["scalerank"]))
            if kind == "lakes":
                feature["polygons"] = polygons(parts)
            else:
                rounded = []
                for part in parts:
                    points = []
                    for point in part:
                        p = [round(v, 3) for v in point]
                        if not points or points[-1] != p:
                            points.append(p)
                    rounded.append(points)
                feature["parts"] = rounded
            features.append(feature)
        result[kind] = features
    validate_geometry(result)
    return result


def manifest(compiled):
    return dict(id="reference-hydrology-v1", source="Natural Earth physical lakes and rivers/lake centerlines 1:110m, v3.3.0", revision=REVISION, retrieved_at=RETRIEVED,
                source_sha256=DIGESTS, source_urls=URLS, attribution="Natural Earth", license="Public domain", license_url="https://www.naturalearthdata.com/about/terms-of-use/",
                creator="Natural Earth contributors; compiler in scripts/import_hydrology.py", feature_counts=dict(lakes=25, rivers=14),
                transformation="Preserve SHP polygon rings/holes and separate river parts; round WGS84 lon/lat to 0.001 degrees; collapse adjacent duplicate river points; normalize DBF label line-break escapes/whitespace; retain alternate labels and source-row IDs. Ignore administrative attributes. IDs refer to rows in this pinned release, not persistent real-world entity IDs.",
                applicable_dates="Generalized reference release v3.3.0; not certified 2000 water extents or river courses.",
                limitations="Only 25 lake and 14 river features, not complete hydrology. Source river features may be separate parts with the same name. No river width, depth, discharge, navigation, water supply, legal sovereignty, borders or simulation-region assignments are inferred. Lake extents are not dated observations. Display layers do not change country picking or campaign state.",
                compiled_sha256=hashlib.sha256(serialize(compiled).encode("utf-8")).hexdigest())


def validate_compiled():
    raw = (ROOT / "data/hydrology-110m.json").read_bytes()
    data = json.loads(raw)
    metadata = json.loads((ROOT / "data/hydrology-manifest.json").read_text(encoding="utf-8"))
    validate_geometry(data)
    if raw != serialize(data).encode("utf-8") or metadata != manifest(data):
        raise ValueError("Physical geometry or provenance differs from its manifest/format")
    print("Reference hydrology OK: 25 lakes, 14 river features, manifest checksum and geometry validated.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", type=Path)
    parser.add_argument("--validate", action="store_true")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.validate:
        validate_compiled()
        return
    raw = {}
    for name, url in URLS.items():
        if args.source_dir:
            raw[name] = (args.source_dir / name).read_bytes()
        else:
            with urlopen(url, timeout=30) as response:
                raw[name] = response.read()
    data = compile_geometry(raw)
    for name, value in [("hydrology-110m.json", data), ("hydrology-manifest.json", manifest(data))]:
        path = ROOT / "data" / name
        expected = serialize(value)
        if args.check:
            if path.read_text(encoding="utf-8") != expected:
                raise ValueError(f"{name} differs from its pinned inputs")
        else:
            path.write_text(expected, encoding="utf-8", newline="\n")
    print("Reference hydrology matches pinned inputs." if args.check else "Compiled 25 reference lakes and 14 river features.")


if __name__ == "__main__":
    main()
