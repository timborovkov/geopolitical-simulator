"""Offline regression checks for source parsing, missingness and derived content."""
import copy
import json
import unittest
from import_reference_data import ROOT, applied_value, observations, apply_manifest, build_manifest


class ReferenceDataTests(unittest.TestCase):
    def test_units_truncation_and_invalid_numbers(self):
        self.assertEqual(applied_value('1665534876683.3105', 'gdp'), 1665534)
        self.assertEqual(applied_value('58892514', 'population'), 58892514)
        for value in ['NaN', 'Infinity', '-1', '0', 'text', '999', '1e999999999']:
            with self.assertRaises(ValueError):
                applied_value(value, 'population')
        with self.assertRaises(ValueError):
            applied_value('12345.5', 'population')
        with self.assertRaises(ValueError):
            applied_value('2000000001', 'population')

    def test_missing_gdp_is_explicit_and_other_gaps_fail(self):
        countries = [dict(id='PRK', gdp=17000, economic_source='authored-balance-v1')]
        records, gaps = observations(b'Country Code,Year,Value\nUSA,2000,1000000000\n', 'gdp', countries)
        self.assertEqual(records, [])
        self.assertEqual(gaps[0]['fallback_value'], 17000)
        self.assertEqual(gaps[0]['country'], 'PRK')
        with self.assertRaises(ValueError):
            observations(b'Country Code,Year,Value\n', 'population', countries)
        with self.assertRaises(ValueError):
            observations(b'Country Code,Year,Value\n', 'gdp', [dict(id='USA')])

    def test_duplicates_columns_and_unpinned_bytes_fail(self):
        with self.assertRaises(ValueError):
            observations(b'Country Code,Year,Value\nUSA,2000,1000\nUSA,2000,1001\n', 'population', [dict(id='USA')])
        with self.assertRaises(ValueError):
            observations(b'Country Code,Value\nUSA,1000\n', 'population', [dict(id='USA')])
        with self.assertRaises(ValueError):
            build_manifest({'population': b'not the pinned file', 'gdp': b'also incorrect'}, [])
        with self.assertRaises(ValueError):
            build_manifest({}, [])

    def test_committed_manifest_transforms_only_observed_fields(self):
        original = json.loads((ROOT / 'data/initial-2000-elections.json').read_text(encoding='utf-8'))
        manifest = json.loads((ROOT / 'data/starter-observations.json').read_text(encoding='utf-8'))
        pack = apply_manifest(copy.deepcopy(original), manifest)
        self.assertEqual(len(pack['facts']), 29)
        for before, after in zip(original['countries'], pack['countries']):
            self.assertEqual({k: v for k, v in before.items() if k not in ['population', 'gdp', 'quality']},
                             {k: v for k, v in after.items() if k not in ['population', 'gdp', 'quality']})
        self.assertEqual(next(c for c in pack['countries'] if c['id'] == 'PRK')['gdp'], 17000)
        self.assertFalse(any(f['country'] == 'PRK' and f['field'] == 'gdp' for f in pack['facts']))

    def test_manifest_cannot_hide_gaps_duplicate_records_or_change_transformations(self):
        pack = json.loads((ROOT / 'data/initial-2000-elections.json').read_text(encoding='utf-8'))
        manifest = json.loads((ROOT / 'data/starter-observations.json').read_text(encoding='utf-8'))
        for change in ['gap', 'duplicate', 'value', 'source']:
            bad = copy.deepcopy(manifest)
            if change == 'gap':
                bad['missing'] = []
            elif change == 'duplicate':
                bad['observations'].append(bad['observations'][0])
            elif change == 'value':
                bad['observations'][0]['applied_value'] += 1
            else:
                bad['observations'][0]['source_version'] = 'unverified'
            with self.assertRaises(ValueError):
                apply_manifest(copy.deepcopy(pack), bad)


if __name__ == '__main__':
    unittest.main()
