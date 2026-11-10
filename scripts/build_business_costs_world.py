"""Build attributed civilian production-cost accounting exercises offline."""
import argparse
import hashlib
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
SOURCE = 'authored-business-production-costs'
DESCRIPTION = (
    'Authored aggregate civilian business cost accounts, not historical firm financial statements. '
    'Actual private household factor payments are allocated 35% to domestic food production and 15% to domestic energy production; '
    'these direct cost shares round down, with remaining payments and idle production allocations expensed today. '
    'All preexisting factor payments remain actual transfers, not added payroll or employment. '
    'Moving weighted-average carrying costs pool opening stock cost, actual domestic factor costs and actual import payments. '
    'Closing stock value rounds down; removed cost is split between delivered use and spoilage, with the remainder charged to spoilage. '
    'An import payment delivering no whole quantity is expensed rather than capitalized. '
    'Opening civilian stocks are zero; public reserve releases have zero business acquisition cost and remain identified as public in-kind supplies. '
    'Energy use is a common period production expense; it is not added again to food inventory. '
    'Public auxiliary industrial stocks and equipment are excluded from private inventories and assets. '
    'Government policy contracts earn the standard 10% administration component at proposal, including immediate-consent routes; '
    'paid capital is deferred until actual delivery or recognized separately as retained termination income upon funded withdrawal. '
    'Amendment fees are earned administrative receipts. Existing unpaid maintenance claims are receivables, not repeated sales when collected. '
    'The modeled business result, including separately identified retained termination income, reconciles exactly to cash change, owner distributions, stock carrying costs, receivables and deferred contract receipts. '
    'Owner distributions are equity payments, never operating expenses. No cash, goods, wages, jobs, GDP or ownership conversion is created by valuation. '
    'Private fixed-asset depreciation, contractual unpaid payroll, corporate tax, firm investment and insolvency remain unfinished; '
    'the corporate cash distribution rule is preserved and does not pretend this cost account is a complete solvency or payout policy.'
)
def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true')
    args=parser.parse_args()
    for parent,target,fictional in [
        ('initial-2000-delivered-consumption.json','initial-2000-business-costs.json',False),
        ('synthetic-delivered-consumption-test.json','synthetic-business-costs-test.json',True),
    ]:
        raw=(ROOT/'data'/parent).read_bytes();world=json.loads(raw)
        world.update(schema=37,rules=37,id='synthetic-business-costs-v37'if fictional else'initial-2000-business-costs-v37',name='Fictional business production costs'if fictional else'A New Century · business production costs')
        world['description']+=' '+DESCRIPTION
        world['sources'].append(dict(id=SOURCE,quality='Authored production-cost accounting exercise',description=DESCRIPTION+' Preserved consumption pack SHA-256 '+hashlib.sha256(raw).hexdigest()))
        world['business_costs']=dict(source=SOURCE,source_version='business-costs-v1',description=DESCRIPTION,food_factor_bps=3500,energy_factor_bps=1500)
        data=(json.dumps(world,ensure_ascii=False,indent=2)+'\n').encode();path=ROOT/'data'/target
        if args.check:
            if not path.exists()or path.read_bytes()!=data:raise SystemExit(f'{target} differs from attributed offline inputs')
            print(f'{target}: verified ({len(data)} bytes)')
        else:
            path.write_bytes(data);print(f'{target}: wrote {len(data)} bytes')
if __name__=='__main__':main()
