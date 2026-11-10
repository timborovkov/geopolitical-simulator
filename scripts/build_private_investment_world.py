"""Build attributed private equipment investment exercises without changing predecessors."""
import argparse
import hashlib
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
SOURCE = 'authored-private-equipment'
DESCRIPTION = (
    'Authored private equipment market and corporate investment heuristic, not historical firm accounts. '
    'Opening private equipment is explicitly zero; opening civilian ownership remains unspecified and public assets are not converted. '
    'An external equipment supplier can deliver at most one equipment lot per country at each 30-day review, '
    'separate from public auxiliary-stock import quotas and public inventories. Firms pay the full 0.5% opening annual GDP price in cash. '
    'A delivered lot installs for 30 days, then contributes five opening-basis capacity points to food or energy for 3650 days. '
    'A good can have at most four unexpired lots including installation. Incremental output uses already paid civilian factors, '
    'with readiness derived from the last completed production-cost receipt against 1% of opening daily GDP; '
    'storage, shocks and the existing physical market settlement still constrain actual production. '
    'Straight-line whole-credit depreciation accumulates from commissioning to full cost at retirement; '
    'capital outlays are assets rather than factor expenses and net assets reconcile the cash-to-result bridge. '
    'At review firms prefer the greatest observed last-day avoidable import expense, with food winning exact ties. '
    'The quoted equipment price must be recovered within 730 days at that conditional saving, '
    'covered by prior-day retained earnings after actual dividends and current cash after both trailing 30-day payments and unfinished client advances. '
    'This is a conditional import-substitution heuristic, not guaranteed sales or a government order. '
    'Equipment creates no cash, residents, jobs, wages or GDP. A maximum of 256 retained acquisition receipts bounds history. '
    'Corporate taxation, contractual unpaid payroll, private finance, mergers and formal insolvency remain unfinished.'
)
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for parent, target, fictional in [
        ('initial-2000-business-finance.json', 'initial-2000-private-investment.json', False),
        ('synthetic-business-finance-test.json', 'synthetic-private-investment-test.json', True),
    ]:
        raw = (ROOT / 'data' / parent).read_bytes()
        world = json.loads(raw)
        world.update(schema=39, rules=39,
                     id='synthetic-private-investment-v39' if fictional else 'initial-2000-private-investment-v39',
                     name='Fictional private equipment investment' if fictional else 'A New Century · private equipment investment')
        world['description'] += ' ' + DESCRIPTION
        world['sources'].append(dict(id=SOURCE, quality='Authored corporate equipment assumptions',
                                    description=DESCRIPTION + ' Preserved finance pack SHA-256 ' + hashlib.sha256(raw).hexdigest()))
        world['private_investment'] = dict(source=SOURCE, source_version='private-equipment-v1', description=DESCRIPTION,
                                          capital_cost_bps=50, capacity_points=5, installation_days=30,
                                          lifetime_days=3650, maximum_payback_days=730, maximum_active_per_good=4)
        data = (json.dumps(world, ensure_ascii=False, indent=2) + '\n').encode()
        path = ROOT / 'data' / target
        if args.check:
            if not path.exists() or path.read_bytes() != data:
                raise SystemExit(f'{target} differs from attributed offline inputs')
            print(f'{target}: verified ({len(data)} bytes)')
        else:
            path.write_bytes(data)
            print(f'{target}: wrote {len(data)} bytes')
if __name__ == '__main__':
    main()
