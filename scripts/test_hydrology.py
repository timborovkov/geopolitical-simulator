"""Offline checks for pinned physical geometry and malformed transformations."""
import copy
import json
from pathlib import Path
import struct
import unittest
from import_country_geometry import shapes
from import_hydrology import ROOT, DIGESTS, compile_geometry, label, manifest, validate_geometry


class HydrologyTests(unittest.TestCase):
    def test_pinned_counts_names_and_provenance(self):
        data = json.loads((ROOT / "data/hydrology-110m.json").read_text(encoding="utf-8"))
        validate_geometry(data)
        metadata = json.loads((ROOT / "data/hydrology-manifest.json").read_text(encoding="utf-8"))
        self.assertEqual(metadata, manifest(data))
        self.assertEqual(label("Lake\\rBaikal\r\n"), "Lake Baikal")
        self.assertEqual(next(x for x in data['lakes'] if x['id'] == 'lake-0001')['name'], 'Lake Baikal')
        self.assertEqual(sum(x['name'] == 'Peace' for x in data['rivers']), 2)
        self.assertEqual(len({x['id'] for kind in data.values() for x in kind}), 39)
        with self.assertRaises(ValueError):
            compile_geometry({})
        with self.assertRaisesRegex(ValueError, 'Unpinned'):
            compile_geometry({name: b'wrong source bytes' for name in DIGESTS})

    def test_invalid_coordinates_gaps_and_missing_geometry_fail(self):
        data = json.loads((ROOT / "data/hydrology-110m.json").read_text(encoding="utf-8"))
        for case in range(8):
            bad = copy.deepcopy(data)
            if case == 0:
                bad['lakes'][0]['polygons'][0][0][0] = [181, 0]
            elif case == 1:
                bad['rivers'][0]['parts'][0][0] = [float('nan'), 0]
            elif case == 2:
                bad['rivers'][0]['parts'][0] = [[179, 0], [-179, 0]]
            elif case == 3:
                bad['rivers'][0]['parts'][0] = [[0, 0]]
            elif case == 4:
                bad['lakes'][0]['polygons'][0][0].pop()
            elif case == 5:
                bad['rivers'][0]['parts'][0][0] = [True, 1]
            elif case == 6:
                bad['lakes'][1]['id'] = bad['lakes'][0]['id']
            else:
                bad['lakes'][0]['polygons'] = []
            with self.assertRaises(ValueError, msg=f'case {case}'):
                validate_geometry(bad)

    def test_shape_parser_preserves_separate_river_parts_and_rejects_polygon_confusion(self):
        # One PolyLine record with two disconnected parts. No segment may join them.
        points = [(0, 0), (1, 1), (10, 10), (11, 11)]
        record = struct.pack('<i4d2i2i8d', 3, 0, 0, 11, 11, 2, 4, 0, 2, *[v for p in points for v in p])
        header = bytearray(100)
        struct.pack_into('>i', header, 0, 9994)
        struct.pack_into('<i', header, 32, 3)
        raw = bytes(header) + struct.pack('>ii', 1, len(record) // 2) + record
        self.assertEqual(shapes(raw, 3), [[[list(p) for p in points[:2]], [list(p) for p in points[2:]]]])
        with self.assertRaises(ValueError):
            shapes(raw)  # Default country parser remains polygon-only.
        with self.assertRaises((ValueError, struct.error)):
            shapes(raw[:-4], 3)


if __name__ == '__main__':
    unittest.main()
