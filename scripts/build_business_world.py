"""Build attributed household-owned aggregate enterprise exercises offline."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = 'authored-household-businesses'
DESCRIPTION = (
    'Authored household-owned aggregate civilian business cash exercise, not historical ownership or national profit accounts. '
    'Each existing Firms account represents one enterprise cohort; existing public industrial stocks and equipment remain public. '
    'Household groups own fixed shares in proportion to their authored opening savings weights, without individual ownership records. '
    'Every 30 elapsed campaign days, businesses autonomously distribute up to 25% of positive undistributed operating cash, '
    'while protecting actual trailing 30-day business payments. Household and external receipts add to earned cash; '
    'all business payments, including past distributions, subtract. Opening capital and every Treasury receipt are excluded. '
    'A missing owning population cohort suspends payouts without reallocating ownership. Payments enter household income before ordinary withholding. '
    'This transfers existing cash and creates no GDP, jobs, wages or new money. The government cannot order a private payout. '
    'Cash generation is not accrual profit; commodity cost valuation, public/mixed/cooperative ownership, conversions, insolvency and investment remain unfinished.'
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for parent, target, fictional in [
        ('initial-2000-industrial-trade.json', 'initial-2000-businesses.json', False),
        ('synthetic-industrial-trade-test.json', 'synthetic-businesses-test.json', True),
    ]:
        raw = (ROOT / 'data' / parent).read_bytes()
        world = json.loads(raw)
        world.update(schema=35, rules=35,
                     id='synthetic-businesses-v35' if fictional else 'initial-2000-businesses-v35',
                     name='Fictional household businesses' if fictional else 'A New Century · household-owned businesses')
        world['description'] += ' ' + DESCRIPTION
        world['sources'].append(dict(id=SOURCE, quality='Authored business ownership exercise',
                                     description=DESCRIPTION + ' Preserved trade pack SHA-256 ' + hashlib.sha256(raw).hexdigest()))
        world['businesses'] = dict(source=SOURCE, source_version='household-businesses-v1',
                                   description=DESCRIPTION, distribution_bps=2500)
        data = (json.dumps(world, ensure_ascii=False, indent=2) + '\n').encode()
        path = ROOT / 'data' / target
        if args.check:
            if not path.exists() or path.read_bytes() != data:
                raise SystemExit(f'{target} differs from its attributed offline inputs')
            print(f'{target}: verified ({len(data)} bytes)')
        else:
            path.write_bytes(data)
            print(f'{target}: wrote {len(data)} bytes')


if __name__ == '__main__':
    main()
