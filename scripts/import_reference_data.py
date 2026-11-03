"""Build the starter's attributed observations without rewriting published world packs.

Uses only the standard library. Offline CSVs must match the pinned source digests.
Missing North Korean GDP retains its explicitly authored calibration; never substitute zero.
"""
import argparse
import csv
from decimal import Decimal, InvalidOperation
import hashlib
import io
import json
from pathlib import Path
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[1]
REFERENCES = {
    'population': ('d4b1a4a9a17e2123baf0ef119de05bceced1ab1b', '2c697c3e668c8cb06ee467cd0d475b37b52844fdc48cb7725b79b58f18bf0b02', 'persons'),
    'gdp': ('36d761dc5617982d3e50b213e02b9a1b47c4e958', '8d84ef6bcaccf740c01c5ed7566aae8f7b149d0e2c20001e441efd3cb5b2fa86', 'current USD'),
}
TRANSFORMATIONS = {
    'population': 'Use annual population observation as bookmark population proxy; not a 1 January census.',
    'gdp': 'Truncate annual current-USD GDP to million USD; use as opening game GDP. No PPP or exchange-rate transformation.',
}


def applied_value(raw_value, field):
    try:
        value = Decimal(raw_value)
    except InvalidOperation as error:
        raise ValueError(f'Invalid numeric {field} observation') from error
    if not value.is_finite() or value <= 0:
        raise ValueError(f'Nonpositive or nonfinite {field} observation')
    if field == 'population' and value != value.to_integral_value():
        raise ValueError('Population observation must contain whole persons')
    divisor = 1_000_000 if field == 'gdp' else 1
    maximum = 100_000_000 if field == 'gdp' else 2_000_000_000
    if not 1_000 * divisor <= value < (maximum + 1) * divisor:
        raise ValueError(f'{field} observation exceeds the model range')
    return int(value / divisor)


def observations(raw, field, countries):
    reader = csv.DictReader(io.StringIO(raw.decode('utf-8-sig')))
    if not {'Country Code', 'Year', 'Value'}.issubset(reader.fieldnames or []):
        raise ValueError('Missing required source columns')
    rows = list(reader)
    result, missing = [], []
    for country in countries:
        matches = [row for row in rows if row['Country Code'] == country['id'] and row['Year'] == '2000']
        if len(matches) > 1:
            raise ValueError(f"{country['id']}: duplicate {field} observations")
        if not matches or not matches[0]['Value']:
            if field != 'gdp' or country['id'] != 'PRK':
                raise ValueError(f"{country['id']}: missing required {field} observation")
            missing.append(dict(country=country['id'], field=field,
                                reason='No 2000 GDP observation in the pinned mirror; authored calibration retained, not zero or an inferred observation.',
                                fallback_value=country['gdp'], fallback_source=country['economic_source']))
            continue
        raw_value = matches[0]['Value']
        result.append(dict(country=country['id'], field=field, observation_period='2000',
                           original_value=raw_value, original_units=REFERENCES[field][2],
                           applied_value=applied_value(raw_value, field),
                           source=f'world-bank-mirror-{field}', source_version=REFERENCES[field][0],
                           quality='published_mirror_observation', transformation=TRANSFORMATIONS[field]))
    return result, missing


def build_manifest(raw_files, countries):
    if set(raw_files) != set(REFERENCES):
        raise ValueError("Both pinned source files are required")
    facts, missing = [], []
    for field, raw in raw_files.items():
        if hashlib.sha256(raw).hexdigest() != REFERENCES[field][1]:
            raise ValueError(f'{field}: bytes do not match the pinned source digest')
        records, gaps = observations(raw, field, countries)
        facts.extend(records)
        missing.extend(gaps)
    # Preserve the original source/licensing evidence; new coverage does not certify a primary API.
    sources = json.loads((ROOT / 'data/reference-observations.json').read_text(encoding='utf-8'))['sources']
    return dict(sources=sources, observations=facts, missing=missing)


def apply_manifest(pack, manifest):
    """Validate the committed extraction offline before assembling its derived pack."""
    expected = {(c['id'], field) for c in pack['countries'] for field in REFERENCES
                if (c['id'], field) != ('PRK', 'gdp')}
    records = manifest['observations']
    keys = [(fact['country'], fact['field']) for fact in records]
    if set(keys) != expected or len(keys) != len(expected):
        raise ValueError('Starter observations have missing, duplicate or unexpected fields')
    expected_sources = json.loads((ROOT / 'data/reference-observations.json').read_text(encoding='utf-8'))['sources']
    if manifest['sources'] != expected_sources:
        raise ValueError('Starter source evidence differs from the pinned source manifest')
    for fact in records:
        field = fact['field']
        if (fact['observation_period'] != '2000'
                or fact['original_units'] != REFERENCES[field][2]
                or fact['source_version'] != REFERENCES[field][0]
                or fact['source'] != f'world-bank-mirror-{field}'
                or fact['quality'] != 'published_mirror_observation'
                or fact['transformation'] != TRANSFORMATIONS[field]
                or fact['applied_value'] != applied_value(fact['original_value'], field)):
            raise ValueError('Invalid starter observation provenance or transformation')
        country = next(c for c in pack['countries'] if c['id'] == fact['country'])
        country[field] = fact['applied_value']
        country['quality'] = 'mixed_observed_and_authored'
    fallback = next(c for c in pack['countries'] if c['id'] == 'PRK')
    if (len(manifest['missing']) != 1
            or manifest['missing'][0]['country'] != 'PRK'
            or manifest['missing'][0]['field'] != 'gdp'
            or not manifest['missing'][0]['reason']
            or manifest['missing'][0]['fallback_value'] != fallback['gdp']
            or manifest['missing'][0]['fallback_source'] != fallback['economic_source']):
        raise ValueError('Missing GDP must retain its explicit authored fallback')
    pack['facts'] = records
    for source in pack['sources']:
        if source['id'].startswith('world-bank-mirror-'):
            source['description'] = source['description'].replace('data/reference-observations.json', 'data/starter-observations.json')
    return pack


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--population', type=Path)
    parser.add_argument('--gdp', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    countries = json.loads((ROOT / 'data/initial-2000-elections.json').read_text(encoding='utf-8'))['countries']
    raw_files = {}
    for field, (revision, _, _) in REFERENCES.items():
        local = getattr(args, field)
        if local:
            raw_files[field] = local.read_bytes()
        else:
            with urlopen(f'https://raw.githubusercontent.com/datasets/{field}/{revision}/data/{field}.csv', timeout=60) as response:
                raw_files[field] = response.read()
    manifest = build_manifest(raw_files, countries)
    output = ROOT / 'data/starter-observations.json'
    expected = json.dumps(manifest, ensure_ascii=False, indent=2) + '\n'
    if args.check:
        if output.read_text(encoding='utf-8') != expected:
            raise ValueError('Starter observations differ from pinned source extraction')
    else:
        output.write_text(expected, encoding='utf-8')
    print(f"Verified {len(manifest['observations'])} observations and {len(manifest['missing'])} explicit gap; published packs remain unchanged.")


if __name__ == '__main__':
    main()
