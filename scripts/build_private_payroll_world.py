"""Generate attributed private employment and payroll exercises offline."""
import argparse
import hashlib
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
SOURCE = 'authored-private-payroll'
DESCRIPTION = (
    'Authored aggregate private employment contracts, not historical labor-force or salary data. '
    'Private contracts initially cover 70% of working-age residents in each non-public cohort, '
    'and 20% in the public cohort. Another 60% of the public cohort are possible public positions, '
    'estimated from actual service funding; these are not a second private payroll. '
    'Private positions are bounded by opening contract counts and current working-age residents. '
    '70% of the existing opening private-factor income benchmark becomes a fixed aggregate wage bill, '
    'apportioned using existing income weights, then scaled to current contracted positions. '
    'Aggregate wage benchmarks and job-scaled daily bills round down to whole credit millions; '
    'largest remainders preserve each apportioned cash total exactly. '
    'The remaining 30% is non-wage factor income, still conditional on actual output and cash. '
    'Wage claims accrue before payment; actual available cash settles eligible cohort claims proportionally, '
    'with older claims first inside each cohort. Unpaid claims persist without creating spendable cash. '
    'Payments enter ordinary household income and withholding. Unpaid labor is expensed as period cost; '
    'later arrears payments do not fund current production or enter current inventory allocation. '
    'Payroll liabilities reduce business net earnings and the cash bridge, and protect cash from dividends and equipment purchases. '
    'Recruitment, dismissals, wage bargaining, individual contracts, private credit and formal insolvency remain unfinished. '
    'No residents, goods, cash or GDP are created by recording a contract or unpaid claim.'
)
def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--check', action='store_true')
    args = p.parse_args()
    for parent, target, fictional in [
        ('initial-2000-private-investment.json', 'initial-2000-private-payroll.json', False),
        ('synthetic-private-investment-test.json', 'synthetic-private-payroll-test.json', True),
    ]:
        raw = (ROOT / 'data' / parent).read_bytes()
        world = json.loads(raw)
        world.update(schema=40, rules=40,
                     id='synthetic-private-payroll-v40' if fictional else 'initial-2000-private-payroll-v40',
                     name='Fictional private payroll' if fictional else 'A New Century · private employment & payroll')
        world['description'] += ' ' + DESCRIPTION
        world['sources'].append(dict(id=SOURCE, quality='Authored employment and payroll assumptions',
                                    description=DESCRIPTION + ' Preserved equipment pack SHA-256 ' + hashlib.sha256(raw).hexdigest()))
        world['private_payroll'] = dict(source=SOURCE, source_version='private-payroll-v1', description=DESCRIPTION,
            wage_share_bps=7000, countries={c['id']:{h['id']:dict(private_jobs_bps=2000 if h['id']=='public' else 7000,
                                                             public_jobs_bps=6000 if h['id']=='public' else 0)
                                                 for h in c['households']} for c in world['countries']})
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
