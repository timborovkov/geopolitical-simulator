"""Compile reference outlines for the 15-country starter roster from pinned public-domain data.

Uses Python's standard library. --source-dir accepts offline SHP/DBF/PRJ originals.
This is reference display geometry, not a certified historical sovereignty layer.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[1]
REVISION = "aa15d45ca649047105c9fd38c5d58e95ada27fca"
NAME = "ne_110m_admin_0_countries"
BASE = f"https://raw.githubusercontent.com/nvkelso/natural-earth-vector/{REVISION}/110m_cultural/{NAME}"
DIGESTS = {
    "shp": "2aebdd66fde53694d536590da15def874cceb91e0bf6151bba7bae72f60949ab",
    "dbf": "5432c814c83169a2e6568400dfd4870e95869970e96f533c292eac9753d0c8e7",
    "prj": "a02a27b1d1982c8516d83398e85a3c8b1aef1713c13ef4d84d7bde17430c07c4",
}


def records(raw, encoding="latin1"):
    count = struct.unpack_from("<I", raw, 4)[0]
    header, width = struct.unpack_from("<HH", raw, 8)
    fields = []
    for offset in range(32, header - 1, 32):
        field = raw[offset:offset + 32]
        fields.append((field[:11].split(b"\0")[0].decode("ascii").lower(), field[16]))
    result = []
    for index in range(count):
        record = raw[header + index * width:header + (index + 1) * width]
        if len(record) != width or record[0] != 32:
            raise ValueError("Truncated or deleted DBF record")
        pos = 1
        values = {}
        for name, size in fields:
            values[name] = record[pos:pos + size].decode(encoding).strip()
            pos += size
        result.append(values)
    return result


def shapes(raw, expected_type=5):
    if struct.unpack_from(">i", raw)[0] != 9994 or struct.unpack_from("<i", raw, 32)[0] != expected_type:
        raise ValueError(f"Expected shapefile type {expected_type}")
    result = []
    offset = 100
    while offset < len(raw):
        _, length = struct.unpack_from(">ii", raw, offset)
        record = raw[offset + 8:offset + 8 + length * 2]
        offset += 8 + length * 2
        if len(record) != length * 2 or struct.unpack_from("<i", record)[0] != expected_type:
            raise ValueError("Truncated or unexpected shape type")
        parts, count = struct.unpack_from("<ii", record, 36)
        indexes = list(struct.unpack_from("<" + "i" * parts, record, 44)) + [count]
        points = [list(struct.unpack_from("<dd", record, 44 + parts * 4 + i * 16))
                  for i in range(count)]
        result.append([points[a:b] for a, b in zip(indexes, indexes[1:])])
    return result


def signed_area(ring):
    return sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(ring, ring[1:])) / 2


def inside(ring, point):
    x, y = point
    result = False
    for a, b in zip(ring, ring[1:]):
        if (a[1] > y) != (b[1] > y) and x < (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]) + a[0]:
            result = not result
    return result


def polygons(rings):
    # SHP specifies clockwise exterior rings and counterclockwise holes.
    result = [[ring] for ring in rings if signed_area(ring) < 0]
    for hole in (ring for ring in rings if signed_area(ring) > 0):
        owners = [polygon for polygon in result if inside(polygon[0], hole[0])]
        if not owners:
            raise ValueError("Unassigned interior ring")
        min(owners, key=lambda polygon: abs(signed_area(polygon[0]))).append(hole)
    for polygon in result:
        for ring in polygon:
            if len(ring) < 4 or ring[0] != ring[-1]:
                raise ValueError("Invalid polygon ring")
            for point in ring:
                if not (-180.001 <= point[0] <= 180.001 and -90 <= point[1] <= 90):
                    raise ValueError("Invalid geographic coordinate")
                point[:] = [round(v, 3) for v in point]
            if any(abs(a[0] - b[0]) > 180.001 for a, b in zip(ring, ring[1:])):
                raise ValueError("Unsplit antimeridian edge")
    return result


def serialize(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":")) + "\n"


def validate_compiled():
    path = ROOT / "data/country-outlines-110m.json"
    raw = path.read_bytes()
    manifest = json.loads((ROOT / "data/country-outlines-manifest.json").read_text(encoding="utf-8"))
    if hashlib.sha256(raw).hexdigest() != manifest["compiled_sha256"]:
        raise ValueError("Compiled geometry differs from its manifest checksum")
    compiled = json.loads(raw)
    roster = {c["id"] for c in json.loads((ROOT / "data/initial-2000.json").read_text(encoding="utf-8"))["countries"]}
    if compiled.keys() != roster:
        raise ValueError("Reference geometry roster differs from starter pack")
    for country, country_polygons in compiled.items():
        if not country_polygons:
            raise ValueError(f"{country}: missing polygons")
        for polygon in country_polygons:
            if not polygon:
                raise ValueError(f"{country}: missing exterior ring")
            for ring in polygon:
                if len(ring) < 4 or ring[0] != ring[-1]:
                    raise ValueError(f"{country}: unclosed ring")
                for point in ring:
                    if len(point) != 2 or not all(isinstance(v, (int, float)) and math.isfinite(v) for v in point):
                        raise ValueError(f"{country}: invalid point")
                    if not (-180.001 <= point[0] <= 180.001 and -90 <= point[1] <= 90):
                        raise ValueError(f"{country}: out-of-range coordinate")
                if any(abs(a[0] - b[0]) > 180.001 for a, b in zip(ring, ring[1:])):
                    raise ValueError(f"{country}: unsplit antimeridian edge")
    print(f"Reference geometry OK: {len(compiled)} countries, manifest checksum and rings validated.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", type=Path)
    parser.add_argument("--validate", action="store_true", help="validate committed output offline; do not download or write")
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
            raise ValueError(f"{ext}: input does not match the pinned source")
    attributes, geometry = records(raw["dbf"]), shapes(raw["shp"])
    if len(attributes) != len(geometry):
        raise ValueError("Geometry and attribute record counts differ")
    roster = {c["id"] for c in json.loads((ROOT / "data/initial-2000.json").read_text(encoding="utf-8"))["countries"]}
    compiled = {}
    for row, rings in zip(attributes, geometry):
        identifier = row["adm0_a3"]
        if identifier in roster:
            if identifier in compiled:
                raise ValueError("Duplicate source country")
            compiled[identifier] = polygons(rings)
    if compiled.keys() != roster:
        raise ValueError(f"Missing geometry: {sorted(roster - compiled.keys())}")
    compiled = dict(sorted(compiled.items()))
    manifest = dict(
        id="starter-reference-outlines-v1", source="Natural Earth admin-0 countries 1:110m, v3.3.0",
        revision=REVISION, retrieved_at="2026-11-03", source_sha256=DIGESTS,
        source_urls={ext: f"{BASE}.{ext}" for ext in DIGESTS},
        license="Public domain", license_url="https://www.naturalearthdata.com/about/terms-of-use/",
        attribution="Natural Earth", creator="Natural Earth contributors; game compiler in scripts/import_country_geometry.py",
        transformation="Select 15 starter ADM0_A3 records; preserve exteriors, islands and holes; round lon/lat to 0.001 degrees. WGS84 display geometry only.",
        applicable_dates="Reference source release v3.3.0, not certified as 2000 borders.",
        limitations="No simulation regions, legal sovereignty, occupation or recognition are inferred. Historical and disputed-boundary review remains incomplete. Non-roster countries have no selectable polygons. Small islands absent at 1:110m remain accessible through the country directory/markers.",
        review_checks="Pinned source assigns the Crimea sample to Ukraine and keeps Taiwan separate from CHN. These checks do not certify all historical boundaries.",
        compiled_sha256=hashlib.sha256(serialize(compiled).encode("utf-8")).hexdigest(),
    )
    for name, value in [("country-outlines-110m.json", compiled),
                        ("country-outlines-manifest.json", manifest)]:
        (ROOT / "data" / name).write_text(serialize(value), encoding="utf-8", newline="\n")
    print(f"Compiled reference outlines for {len(compiled)} countries, preserving interior holes.")


if __name__ == "__main__":
    main()
