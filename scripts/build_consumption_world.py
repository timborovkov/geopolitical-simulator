"""Build attributed delivery-based household settlement exercises offline."""
import argparse
import hashlib
import json
from pathlib import Path
ROOT = Path(__file__).resolve().parents[1]
SOURCE = 'authored-delivered-consumption'
DESCRIPTION = (
    'Authored batch household settlement, not historical retail transactions. '
    'Desired basket spending, cash-limited planned budget, affordable food demand and physically delivered food remain distinct. '
    'The food part of the essential basket uses the actual settled food price and configured food budget share; spending above the essential basket is abstract other consumption. '
    'Households retain the quoted food budget corresponding to missing deliveries. Only delivered food is charged, rounded down within whole million-credit reporting units; the rounding discount is less than one reporting unit per group/day. '
    'The other basket component remains abstract, without an additional physical product market. '
    'Plans clear once per day in a single batch; retained cash is available for later days, without a same-day second auction or borrowing. '
    'Actual payments transfer existing household cash to businesses and enter ordinary business cash reconciliation. No cash, food, wages, jobs or GDP is created. '
    'Public industrial assets, private ownership weights and ordinary government authority remain unchanged. Accrual profit and production-cost valuation remain unfinished.'
)
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    for parent,target,fictional in [
        ('initial-2000-businesses.json','initial-2000-delivered-consumption.json',False),
        ('synthetic-businesses-test.json','synthetic-delivered-consumption-test.json',True),
    ]:
        raw = (ROOT/'data'/parent).read_bytes()
        world = json.loads(raw)
        world.update(schema=36,rules=36,id='synthetic-delivered-consumption-v36'if fictional else'initial-2000-delivered-consumption-v36',name='Fictional delivered consumption'if fictional else'A New Century · delivered household consumption')
        world['description'] += ' '+DESCRIPTION
        world['sources'].append(dict(id=SOURCE,quality='Authored delivered consumption exercise',description=DESCRIPTION+' Preserved business pack SHA-256 '+hashlib.sha256(raw).hexdigest()))
        world['consumption'] = dict(source=SOURCE,source_version='delivered-consumption-v1',description=DESCRIPTION)
        data=(json.dumps(world,ensure_ascii=False,indent=2)+'\n').encode()
        path=ROOT/'data'/target
        if args.check:
            if not path.exists()or path.read_bytes()!=data:raise SystemExit(f'{target} differs from attributed offline inputs')
            print(f'{target}: verified ({len(data)} bytes)')
        else:
            path.write_bytes(data)
            print(f'{target}: wrote {len(data)} bytes')
if __name__=='__main__':main()
