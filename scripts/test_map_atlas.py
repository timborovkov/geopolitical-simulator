"""Offline asset contract checks; no GIS dependencies or downloads required."""
import json
import tempfile
import unittest
from pathlib import Path
from build_map_atlas import OUT, validate, digest

class AtlasTests(unittest.TestCase):
    def test_global_assets(self):
        validate()
        data=json.loads((OUT/'atlas.json').read_text(encoding='utf-8'))
        self.assertGreater(len(data['cities']),7000)
        holes = 0
        def area(ring):
            return abs(sum(a[0]*b[1]-b[0]*a[1] for a,b in zip(ring,ring[1:]+ring[:1])))/2
        for country in data['countries']:
            for lod in country['lods']:
                for mesh in lod:
                    rings = mesh['rings']; points = mesh['points']; indices = mesh['triangles']
                    expected = area(rings[0]) - sum(map(area, rings[1:]))
                    actual = sum(area([points[j] for j in indices[i:i+3]]) for i in range(0,len(indices),3))
                    self.assertAlmostEqual(actual, expected, delta=1e-5, msg=country['name'])
                    holes += len(rings)>1
        self.assertGreater(holes, 0, 'Real enclave/lake holes must survive triangulation')
        self.assertEqual({x['kind'] for x in data['lines']},{'rivers','lakes','urban','roads','rail'})
        self.assertEqual(len({x['id'] for x in data['cities']}),len(data['cities']))
        manifest=json.loads((OUT/'manifest.json').read_text(encoding='utf-8'))
        self.assertEqual({t['level'] for t in manifest['tiles']},set(range(5)))
        for level in range(5):
            self.assertEqual(sum(t['level']==level for t in manifest['tiles']),2*4**level)

    def test_invalid_coordinate_and_triangle_index_fail(self):
        original=json.loads((OUT/'atlas.json').read_text(encoding='utf-8'))
        mesh=original['countries'][0]['lods'][0][0]
        countries=[dict(c, lods=[[],[],[]]) for c in original['countries']]
        countries[0]['lods'][0]=[mesh]
        data=dict(countries=countries,cities=[],lines=[])
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            def check(message):
                (root/'atlas.json').write_text(json.dumps(data))
                (root/'manifest.json').write_text(json.dumps({'version':1,'files':{'atlas.json':digest(root/'atlas.json')}}))
                with self.assertRaisesRegex(ValueError,message):validate(root)
            x=mesh['points'][0][0]; mesh['points'][0][0]=181.0
            check('Invalid coordinate')
            mesh['points'][0][0]=x; mesh['triangles'][0]=len(mesh['points'])
            check('Invalid mesh index')

    def test_missing_corrupt_and_unsafe_assets_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);asset=root/'tile.jpg';asset.write_bytes(b'original')
            m={'version':1,'files':{'tile.jpg':digest(asset)}}
            (root/'manifest.json').write_text(json.dumps(m))
            asset.write_bytes(b'corrupted')
            with self.assertRaisesRegex(ValueError,'checksum'):validate(root)
            asset.unlink()
            with self.assertRaises(FileNotFoundError):validate(root)
            m['files']={'../escape':'invalid'};(root/'manifest.json').write_text(json.dumps(m))
            with self.assertRaisesRegex(ValueError,'Unsafe'):validate(root)
            m['version']=2;(root/'manifest.json').write_text(json.dumps(m))
            with self.assertRaisesRegex(ValueError,'version'):validate(root)

if __name__=='__main__':unittest.main()
