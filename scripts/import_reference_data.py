"""Rebuild six reference-country observations from pinned World Bank data mirrors.

Uses only Python's standard library. Pass --population and --gdp for offline files.
All other country values and coefficients remain authored; no missing value becomes zero.
"""
import argparse
import csv
from decimal import Decimal
import hashlib
import io
import json
from pathlib import Path
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[1]
REFERENCES = {
    'population': ('population', 'd4b1a4a9a17e2123baf0ef119de05bceced1ab1b', 'SP.POP.TOTL', 'persons'),
    'gdp': ('gdp', '36d761dc5617982d3e50b213e02b9a1b47c4e958', 'NY.GDP.MKTP.CD', 'current USD'),
}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--population', type=Path)
    parser.add_argument('--gdp', type=Path)
    args = parser.parse_args()
    path = ROOT / 'data/initial-2000.json'
    pack = json.loads(path.read_text())
    facts = []
    manifest = []
    for field, (repo, revision, indicator, units) in REFERENCES.items():
        url = f'https://raw.githubusercontent.com/datasets/{repo}/{revision}/data/{repo}.csv'
        local = getattr(args, field)
        if local:
            raw = local.read_bytes()
        else:
            with urlopen(url, timeout=60) as response:
                raw = response.read()
        digest = hashlib.sha256(raw).hexdigest()
        expected = {'population': '2c697c3e668c8cb06ee467cd0d475b37b52844fdc48cb7725b79b58f18bf0b02', 'gdp': '8d84ef6bcaccf740c01c5ed7566aae8f7b149d0e2c20001e441efd3cb5b2fa86'}[field]
        if digest != expected:
            raise ValueError(f'{field}: bytes do not match the pinned source digest')
        rows = list(csv.DictReader(io.StringIO(raw.decode('utf-8-sig'))))
        source = f'world-bank-mirror-{field}'
        pack['sources'] = [s for s in pack['sources'] if s['id'] != source]
        pack['sources'].append(dict(id=source, quality='published_mirror_observation', description=f'World Bank {indicator}, 2000 observation via datasets/{repo}, revision {revision}. Primary API not independently rechecked. See data/reference-observations.json for provenance.'))
        for country in ['GBR', 'CHN', 'SAU']:
            matches = [r for r in rows if r['Country Code'] == country and r['Year'] == '2000']
            if len(matches) != 1 or not matches[0]['Value']:
                raise ValueError(f'{country}: missing or duplicate {indicator}')
            raw_value = matches[0]['Value']
            value = int(Decimal(raw_value) / (1_000_000 if field == 'gdp' else 1))
            definition = next(d for d in pack['countries'] if d['id'] == country)
            definition[field] = value
            definition['quality'] = 'mixed_observed_and_authored'
            transformation = ('Truncate annual current-USD GDP to million USD; use as opening game GDP. No PPP or exchange-rate transformation.' if field == 'gdp' else 'Use annual population observation as bookmark population proxy; not a 1 January census.')
            facts.append(dict(country=country, field=field, observation_period='2000', original_value=raw_value, original_units=units, applied_value=value, source=source, source_version=revision, quality='published_mirror_observation', transformation=transformation))
        manifest.append(dict(source=source, indicator=indicator, url=url, revision=revision, sha256=digest, retrieved_at='2026-11-03', attribution='World Bank; GDP also credits OECD in mirror metadata', license='CC BY 4.0 for World Bank-produced data; population mirror declares PDDL; retain upstream attribution', primary_source=f'https://data.worldbank.org/indicator/{indicator}', limitations='Single pinned mirror snapshot; later revisions may differ. No independent primary API or national-source reconciliation was performed.'))
    pack['facts'] = facts
    path.write_text(json.dumps(pack, ensure_ascii=False, indent=2) + '\n')
    (ROOT / 'data/reference-observations.json').write_text(json.dumps(dict(sources=manifest, observations=facts), ensure_ascii=False, indent=2) + '\n')
    print('Imported six dated, attributed population/GDP observations; remaining values stay authored.')


if __name__ == '__main__':
    main()
