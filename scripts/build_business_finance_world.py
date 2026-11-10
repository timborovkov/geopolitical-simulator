"""Build attributed earnings-based corporate payout exercises offline."""
import argparse
import hashlib
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
SOURCE = 'authored-business-finance'
DESCRIPTION = (
    'Authored corporate payout policy, not historical financial statements or a complete insolvency model. '
    'This version supersedes the inherited operating-cash distribution base: every 30 elapsed days, '
    'businesses distribute at most the configured 25% of positive retained modeled earnings from the previous complete day. '
    'Retained earnings are cumulative modeled result less prior owner distributions; opening capital is excluded. '
    'Cash must additionally cover both actual trailing 30-day payments and all currently undelivered paid policy-contract capital. '
    'Only remaining cash can fund dividends; absent owning cohorts retain their ownership and block payouts. '
    'Payments enter ordinary household income and withholding. No money, goods, jobs, wages, GDP or public assets are created or converted. '
    'The payment buffer is a corporate liquidity policy, not a contractual debt or formal insolvency finding. '
    'Private investment, contractual unpaid payroll, private fixed assets, depreciation, corporate tax and insolvency remain unfinished.'
)
def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    for parent,target,fictional in [
        ('initial-2000-business-costs.json','initial-2000-business-finance.json',False),
        ('synthetic-business-costs-test.json','synthetic-business-finance-test.json',True),
    ]:
        raw=(ROOT/'data'/parent).read_bytes();world=json.loads(raw)
        world.update(schema=38,rules=38,id='synthetic-business-finance-v38'if fictional else'initial-2000-business-finance-v38',name='Fictional business finance'if fictional else'A New Century · business finance')
        world['description']+=' '+DESCRIPTION
        world['sources'].append(dict(id=SOURCE,quality='Authored corporate finance policy',description=DESCRIPTION+' Preserved business cost pack SHA-256 '+hashlib.sha256(raw).hexdigest()))
        world['business_finance']=dict(source=SOURCE,source_version='business-finance-v1',description=DESCRIPTION)
        data=(json.dumps(world,ensure_ascii=False,indent=2)+'\n').encode();path=ROOT/'data'/target
        if args.check:
            if not path.exists()or path.read_bytes()!=data:raise SystemExit(f'{target} differs from attributed offline inputs')
            print(f'{target}: verified ({len(data)} bytes)')
        else:
            path.write_bytes(data);print(f'{target}: wrote {len(data)} bytes')
if __name__=='__main__':main()
