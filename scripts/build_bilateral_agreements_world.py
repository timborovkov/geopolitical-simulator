"""Author a reproducible fictional pact exercise from preserved payroll inputs."""
import argparse
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = 'authored-bilateral-agreements'
DESCRIPTION = (
    'Authored bilateral non-aggression agreement exercise, not historical treaties or foreign policy. '
    'The government pays one basis point of opening annual GDP per accepted offer, counteroffer, '
    'signature and domestic ratification submission, with a minimum one million credits. '
    'Both signatures and both separately reviewed domestic ratifications precede scheduled activation. '
    'Pending sponsorship lapses after a change of government; active state obligations survive. '
    'Duration, notice, response windows, cooldowns and basic three-day response policies are teaching assumptions. '
    'BOR considers terms up to 365 days with at least 30 days notice; COR declines this pact. '
    'Unaccepted negotiations are private to their participants; signed agreements have a public inspector. '
    'No attacks, armed forces, military enforcement, secret clauses, proxy intervention, '
    'territorial transfers, historical diplomatic coverage or complete strategic AI are implemented by this exercise.'
)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    parent = ROOT / 'data/synthetic-private-payroll-test.json'
    raw = parent.read_bytes()
    world = json.loads(raw)
    world.update(schema=41, rules=41, id='synthetic-bilateral-agreements-v41',
                 name='Fictional diplomacy · non-aggression agreements')
    world['description'] += ' ' + DESCRIPTION
    world['sources'].append(dict(id=SOURCE, quality='Authored diplomatic procedures and teaching policies',
                                description=DESCRIPTION + ' Preserved parent SHA-256 ' + hashlib.sha256(raw).hexdigest()))
    world['diplomacy'] = dict(source=SOURCE, source_version='bilateral-agreements-v1', description=DESCRIPTION,
        minimum_duration=30, maximum_duration=3650, response_days=30, pair_cooldown=30,
        ai_cadence=3, administration_bps=1, maximum_records=4096, maximum_revisions=16,
        ai={country['id']:dict(stance='decline' if country['id']=='COR' else 'consider',
                              maximum_duration=730 if country['id']=='AUR' else 365,
                              minimum_notice=30) for country in world['countries']})
    output = (json.dumps(world, ensure_ascii=False, indent=2) + '\n').encode()
    target = ROOT / 'data/synthetic-bilateral-agreements-test.json'
    if args.check:
        if not target.is_file() or target.read_bytes() != output:
            raise SystemExit('Fictional bilateral agreement fixture differs from attributed offline inputs')
        print(f'{target.name}: verified ({len(output)} bytes)')
    else:
        target.write_bytes(output)
        print(f'{target.name}: wrote {len(output)} bytes')


if __name__ == '__main__':
    main()
