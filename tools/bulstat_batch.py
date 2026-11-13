#!/usr/bin/env python3
"""Bounded BULSTAT fallback checks; source observations and provenance stay local."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

from registry_batch import ROOT, differences, provider_guard, sha, write

POLICY = {'timeoutSeconds': 30, 'requestSpacingSeconds': 0,
          'betweenLookupsSeconds': 30, 'retries': 0, 'maximumIds': 10}


def selection(queue):
    cases = queue['cases']
    if not isinstance(cases, list) or not 1 <= len(cases) <= 10:
        raise ValueError('select 1..10 explicitly sourced cases')
    ids = [case['taxId'] for case in cases]
    for case in cases:
        value = case['taxId']
        if not (isinstance(value, str) and value.isascii() and value.isdigit()
                and len(value) in (9, 13)):
            raise ValueError('unsupported business identifier')
        if not case.get('name') or not case.get('sourceUrl', '').startswith('https://'):
            raise ValueError('name and public HTTPS provenance required')
    if len(set(ids)) != len(ids):
        raise ValueError('duplicate IDs')
    return ids


def report(folder):
    manifest = json.loads((folder / 'manifest.json').read_text())
    rows = []
    reviews_path = folder / 'review.json'
    reviews = json.loads(reviews_path.read_text()) if reviews_path.exists() else {}
    replay_path = folder / 'replay.json'
    replays = {}
    if replay_path.exists():
        for row in json.loads(replay_path.read_text())['rows']:
            if row['taxId'] in replays:
                raise ValueError('duplicate replay ID')
            replays[row['taxId']] = row
    for tax_id in manifest['ids']:
        path = folder / 'observations' / (tax_id + '.json')
        if path.exists():
            row = json.loads(path.read_text())
            if row['taxId'] != tax_id or row['schemaVersion'] != 1:
                raise ValueError('observation identity/schema mismatch')
            summary = {k: row[k] for k in ('taxId', 'outcome', 'elapsedMs', 'error')}
            summary['reviewStatus'] = 'unreviewed'
            replay = replays.get(tax_id)
            if replay:
                if replay['observationSha256'] != sha(path):
                    raise ValueError('replay observation hash mismatch')
                summary['replayMatchesLive'] = (replay['error'] is None and replay['record'] == row['record'])
                summary['replayError'] = replay['error']
            review = reviews.get(tax_id)
            if review:
                if review['observationSha256'] != sha(path):
                    raise ValueError('review observation hash mismatch')
                if review.get('schemaVersion') != 1 or not review.get('expected') or not review.get('scope'):
                    raise ValueError('invalid scoped review')
                actual = replay['record'] if replay and replay['error'] is None else row['record']
                summary['differences'] = differences(review['expected'], actual)
                summary['reviewStatus'] = 'mismatch' if summary['differences'] else 'passed'
            rows.append(summary)
    counts = Counter(row['outcome'] for row in rows)
    result = {'selected': len(manifest['ids']), 'attempted': len(rows),
              'complete': len(rows) == len(manifest['ids']),
              'allBulstatFound': counts['bulstatFound'] == len(manifest['ids']),
              'outcomes': dict(counts), 'reviews': dict(Counter(r['reviewStatus'] for r in rows)), 'cases': rows}
    result['checksPassed'] = (result['complete'] and result['allBulstatFound']
                              and all(r['reviewStatus'] != 'mismatch'
                                      and r.get('replayError') is None
                                      and (r.get('replayMatchesLive', True) or r['reviewStatus'] == 'passed') for r in rows))
    write(folder / 'report.json', result)
    return result


def collect(args):
    queue = json.loads(args.queue.read_text())
    ids = selection(queue)
    folder = args.run_dir.resolve()
    relative = folder.relative_to(ROOT / 'target')  # All evidence stays ignored.
    if not relative.parts:
        raise ValueError('use a dedicated directory beneath target')
    with provider_guard() as (lock, state):
        if folder.exists():
            if not args.resume:
                raise ValueError('existing run; explicit --resume required')
            manifest = json.loads((folder / 'manifest.json').read_text())
            if (manifest['ids'] != ids or manifest['queueSha256'] != sha(args.queue)
                    or manifest['dockerImage'] != args.docker_image or manifest['policy'] != POLICY):
                raise ValueError('resume selection/image/policy changed')
        else:
            manifest = {'schemaVersion': 1, 'ids': ids, 'queueSha256': sha(args.queue),
                        'cases': queue['cases'], 'createdAt': datetime.now(timezone.utc).isoformat(),
                        'policy': POLICY, 'dockerImage': args.docker_image,
                        'binarySha256': sha(args.binary),
                        'sourceHashes': {str(p.relative_to(ROOT)): sha(p) for p in sorted((ROOT / 'src').rglob('*.rs'))}}
            folder.mkdir(parents=True)
            (folder / 'observations').mkdir()
            shutil.copyfile(args.binary, folder / 'probe-bin')
            (folder / 'probe-bin').chmod(0o700)
            write(folder / 'manifest.json', manifest)
        current = report(folder)
        if not current['complete']:
            binary = folder / 'probe-bin'
            if sha(binary) != manifest['binarySha256']:
                raise ValueError('frozen binary changed')
            test = ['bg::bulstat_probe::live_bulstat_batch', '--exact', '--ignored', '--nocapture']
            if args.docker_image:
                cmd = ['docker', 'run', '--rm', '--pull=never', '-v', str(ROOT)+':/work', '-w', '/work',
                       '-e', 'BULSTAT_RUN_DIR=/work/'+str(folder.relative_to(ROOT)),
                       '-e', 'REGISTRY_PROVIDER_STATE=/work/'+str(state.relative_to(ROOT)),
                       '--entrypoint', '/work/'+str(binary.relative_to(ROOT)), args.docker_image, *test]
                env = None
            else:
                cmd = [str(binary), *test]
                env = {**os.environ, 'BULSTAT_RUN_DIR': str(folder), 'REGISTRY_PROVIDER_STATE': str(state)}
            try:
                subprocess.run(cmd, check=True, env=env, pass_fds=(lock.fileno(),))
            finally:
                current = report(folder)
        if current['complete']:
            (folder / 'probe-bin').unlink(missing_ok=True)
        return current


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='action', required=True)
    p = sub.add_parser('collect')
    p.add_argument('--queue', type=Path, required=True)
    p.add_argument('--run-dir', type=Path, required=True)
    p.add_argument('--binary', type=Path, required=True)
    p.add_argument('--docker-image')
    p.add_argument('--resume', action='store_true')
    p = sub.add_parser('report')
    p.add_argument('run_dir', type=Path)
    args = parser.parse_args()
    try:
        result = collect(args) if args.action == 'collect' else report(args.run_dir)
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return 0 if result['checksPassed'] else 2
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        print(str(error), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
