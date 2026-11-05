"""Offline geometry/encoding checks against attributed administrative references."""
import json
from pathlib import Path
import struct
import unittest

from import_administrative_geometry import contains, validate_compiled
from import_country_geometry import records

ROOT = Path(__file__).resolve().parents[1]


class AdministrativeReferenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.features = json.loads((ROOT / "data/administrative-references-10m.json").read_text(
            encoding="utf-8"))["subdivisions"]

    def test_real_reference_samples_preserve_parts_holes_names_and_coverage(self):
        validate_compiled()
        self.assertEqual(len(self.features), 726)
        self.assertEqual(sum(not feature["name"] for feature in self.features), 8)
        samples = [
            ((-.12, 51.5), "GBR-2080"), ((11.58, 48.14), "DEU-1591"),
            ((-46.63, -23.55), "BRA-1311"), ((-150., 64.), "USA-3563"),
            ((179., 66.), "RUS-2321"), ((-179., 67.), "RUS-2321"),
            ((-53., 4.), "FRA-2000"), ((9., 42.), "FRA-5281"),
        ]
        for point, expected in samples:
            self.assertEqual([feature["id"] for feature in self.features if contains(feature, point)],
                             [expected], point)
        for point in [(28.2, -29.5), (-8., 53.), (34.1, 45.1), (121., 24.), (0., 0.)]:
            self.assertFalse(any(contains(feature, point) for feature in self.features), point)
        sao = next(feature for feature in self.features if feature["id"] == "BRA-1311")
        self.assertEqual(sao["name"], "São Paulo")

    def test_dbf_encoding_preserves_utf8_and_the_existing_latin1_default(self):
        def dbf(names, encoding):
            width = 40
            header = bytearray(65)
            header[0] = 3
            struct.pack_into("<IHH", header, 4, len(names), 65, width + 1)
            header[32:36] = b"name"
            header[43] = ord("C")
            header[48] = width
            header[64] = 13
            return bytes(header) + b"".join(b" " + name.encode(encoding).ljust(width, b" ")
                                           for name in names) + b"\x1a"
        names = ["São Paulo", "München"]
        self.assertEqual(records(dbf(names, "utf-8"), encoding="utf-8"),
                         [{"name": name} for name in names])
        self.assertEqual(records(dbf(names, "latin1")), [{"name": name} for name in names])


if __name__ == "__main__":
    unittest.main()
