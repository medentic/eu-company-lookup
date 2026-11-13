#!/usr/bin/env python3
"""Bounded, resumable VIES development collection and offline reports (stdlib only)."""
import argparse
import collections
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import sys
from datetime import datetime, timezone
from vies_reporting import Facts, classify

ROOT = Path(__file__).resolve().parents[1]
POLICY = {"deadlineSeconds": 6, "maxAttempts": 3, "retryStatus": 502,
          "retryPausesSeconds": [0, 1], "successGapSeconds": 5, "errorGapSeconds": 60}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path, value):
    temp = path.with_suffix(path.suffix + '.tmp')
    temp.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')
    temp.replace(path)


def selection(path, offset, count):
    ids = json.loads(path.read_text())
    if offset < 0 or not 1 <= count <= 100 or offset + count > len(ids):
        raise ValueError('offset/count outside queue; count must be 1..100')
    selected = ids[offset:offset + count]
    if len(set(ids)) != len(ids) or any(not isinstance(x, str) or not x.startswith('BG') or not x[2:].isascii() or not x[2:].isdigit() for x in ids):
        raise ValueError('queue contains duplicate or malformed BG identifiers')
    return selected


def load_rows(folder):
    paths = sorted(folder.glob('batch-*.jsonl'))
    if (folder / 'observations.jsonl').exists():
        paths.append(folder / 'observations.jsonl')
    rows = []
    for path in paths:
        text = path.read_text()
        if text and not text.endswith('\n'):
            raise ValueError(f'partial journal tail: preserve and inspect {path}')
        rows.extend(json.loads(line) for line in text.splitlines())
    if len({r['vatId'] for r in rows}) != len(rows):
        raise ValueError('duplicate observed IDs')
    return rows


def report(folder):
    manifest_path = folder / 'manifest.json'
    ids = json.loads(manifest_path.read_text())['ids'] if manifest_path.exists() else json.loads((folder / 'ids.json').read_text())
    rows = load_rows(folder)
    if any(r['vatId'] not in ids for r in rows):
        raise ValueError('journal contains IDs outside selection')
    fixtures = {c['vatId']: c for c in json.loads((ROOT / 'src/vies/fixtures/countries/bg.json').read_text())}
    reviews_path = ROOT / 'src/vies/fixtures/countries/reviews/bg.json'
    reviews = {r['vatId']: r for r in json.loads(reviews_path.read_text())} if reviews_path.exists() else {}
    replay_path = folder / 'offline-replay.json'
    replay = {r['vatId']: r['actual'] for r in json.loads(replay_path.read_text())} if replay_path.exists() else {}
    cases = []
    for row in rows:
        last = row['attempts'][-1]
        response = last.get('response')
        status, diffs = classify(Facts(observed=True)).value, []
        review = reviews.get(row['vatId'], {})
        if response:
            fixture = fixtures.get(row['vatId'])
            # Original observations are immutable. Replay results are separately labelled.
            actual = replay.get(row['vatId'], response['address'])
            if actual.get('fullAddressByVies') != response['address'].get('fullAddressByVies'):
                raise ValueError('replay source differs from journal; regenerate offline replay')
            if fixture:
                diffs = [{'field': k, 'expected': v, 'actual': actual.get(k)}
                         for k, v in fixture['expected']['address'].items() if actual.get(k) != v]
            status = classify(Facts(
                observed=True, response=True, expectation=bool(fixture),
                validityChanged=bool(fixture and fixture['valid'] != response['valid']),
                sourceChanged=bool(fixture and fixture['viesOutput'] != {'company': response['company'], 'address': response['address']['fullAddressByVies']}),
                parserMismatch=bool(diffs), ambiguous=review.get('status') == 'ambiguous',
            )).value
        cases.append({'vatId': row['vatId'], 'status': status, 'differences': diffs,
                      'reviewNote': review.get('note'), 'reviewVersion': review.get('reviewVersion'),
                      'reviewedAt': review.get('reviewedAt'), 'normalizationVersion': review.get('normalizationVersion'),
                      'error': last.get('error'),
                      'totalMs': row['totalMs'], 'parsedFrom': 'offlineReplay' if row['vatId'] in replay else 'liveObservation'})
    counts = collections.Counter(c['status'] for c in cases)
    times = sorted(r['totalMs'] for r in rows)
    passed = counts['passed']
    result = {'selected': len(ids), 'completed': len(rows), 'pending': len(ids) - len(rows),
              'complete': len(ids) == len(rows), 'statuses': dict(counts),
              'passed': passed, 'failed': counts['parserMismatch'],
              'ambiguous': counts['ambiguous'], 'needsReview': counts['needsReview'],
              'providerErrors': counts['providerError'],
              'sourceChanges': counts['sourceChanged'], 'validityChanges': counts['validityChanged'],
              'overallPassPercentage': 100 * passed / len(ids),
              'responses': sum('response' in r['attempts'][-1] for r in rows),
              'attempts': sum(len(r['attempts']) for r in rows),
              'validResponses': sum(r['attempts'][-1].get('response', {}).get('valid') is True for r in rows),
              'invalidResponses': sum(r['attempts'][-1].get('response', {}).get('valid') is False for r in rows),
              'retriedLookups': sum(len(r['attempts']) > 1 for r in rows),
              'recoveredLookups': sum(len(r['attempts']) > 1 and 'response' in r['attempts'][-1] for r in rows),
              'attemptHttpStatuses': dict(collections.Counter(str(a.get('httpStatus', 'none')) for r in rows for a in r['attempts'])),
              'attemptErrors': dict(collections.Counter((a['error'].get('code', 'unknown') if isinstance(a['error'], dict) else a['error']) for r in rows for a in r['attempts'] if 'error' in a)),
              'maxMs': max(times) if times else None,
              'medianMs': statistics.median(times) if times else None,
              'p95Ms': times[math.ceil(.95 * len(times)) - 1] if times else None,
              'cases': cases}
    write_json(folder / 'report.json', result)
    return result


def collect(args):
    folder = args.run_dir.resolve()
    folder.mkdir(parents=True, exist_ok=True)
    # OS-released lock protects manifests and snapshots even if the wrapper crashes.
    with (folder / '.collector.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        manifest_path = folder / 'manifest.json'
        selected = selection(args.queue, args.offset, args.count)
        if manifest_path.exists():
            if not args.resume:
                raise ValueError('run already exists; use --resume')
            manifest = json.loads(manifest_path.read_text())
            if manifest['ids'] != selected or manifest['queueSha256'] != sha(args.queue):
                raise ValueError('resume queue/selection changed')
        else:
            if list(folder.glob('*.jsonl')):
                raise ValueError('unmanaged journals exist; choose a fresh run directory')
            binary = args.binary.resolve()
            source = {str(p.relative_to(ROOT)): sha(p) for p in sorted((ROOT / 'src').rglob('*.rs'))}
            source.update({p: sha(ROOT / p) for p in ['Cargo.toml', 'Cargo.lock']})
            manifest = {'ids': selected, 'offset': args.offset, 'queueSha256': sha(args.queue),
                        'binarySha256': sha(binary), 'sourceHashes': source,
                        'policy': POLICY, 'journalSchemaVersion': 2,
                        'reviewPolicy': json.loads((ROOT / 'src/vies/fixtures/review-policy.json').read_text()),
                        'createdAt': datetime.now(timezone.utc).isoformat(),
                        'gitHead': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                        'dockerImage': args.docker_image}
            shutil.copyfile(binary, folder / 'probe-bin')
            (folder / 'probe-bin').chmod(0o700)
            write_json(manifest_path, manifest)
        current = report(folder)
        if current['complete']:
            print(json.dumps({k: v for k, v in current.items() if k != 'cases'}, indent=2))
            return 0
        binary = folder / 'probe-bin'
        if sha(binary) != manifest['binarySha256']:
            raise ValueError('frozen binary changed')
        if args.docker_image != manifest['dockerImage']:
            raise ValueError('resume Docker image changed')
        journal = folder / 'observations.jsonl'
        env = {'VIES_PROBE_IDS': ','.join(selected), 'VIES_PROBE_RESUME': '1'}
        test_args = ['live_six_second_probe', '--ignored', '--nocapture']
        if args.docker_image:
            relative = folder.relative_to(ROOT)
            env['VIES_PROBE_REPORT'] = '/work/' + str(relative / journal.name)
            cmd = ['docker', 'run', '--rm', '--pull=never', '--entrypoint', '/work/' + str(relative / 'probe-bin'),
                   '-v', str(ROOT) + ':/work', '-w', '/work']
            for k, v in env.items():
                cmd += ['-e', k + '=' + v]
            cmd += [args.docker_image] + test_args
        else:
            env['VIES_PROBE_REPORT'] = str(journal)
            cmd = [str(binary)] + test_args
        try:
            code = subprocess.call(cmd, cwd=ROOT, env={**os.environ, **env})
        finally:
            current = report(folder)
            print(json.dumps({k: v for k, v in current.items() if k != 'cases'}, indent=2))
        if current['complete']:
            binary.unlink()  # Required only while a resumable run remains incomplete.
        return code if code else (0 if current['complete'] else 2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='action', required=True)
    p = sub.add_parser('collect')
    p.add_argument('--queue', type=Path, default=ROOT / 'src/vies/fixtures/ids/bg.json')
    p.add_argument('--offset', type=int, required=True, help='zero-based; 100 starts at ID 101')
    p.add_argument('--count', type=int, default=100)
    p.add_argument('--run-dir', type=Path, required=True)
    p.add_argument('--binary', type=Path, required=True, help='compiled Rust library test executable')
    p.add_argument('--docker-image', help='existing local Linux builder image; no pull')
    p.add_argument('--resume', action='store_true')
    p = sub.add_parser('report')
    p.add_argument('run_dir', type=Path)
    args = parser.parse_args()
    try:
        if args.action == 'collect':
            return collect(args)
        print(json.dumps({k: v for k, v in report(args.run_dir).items() if k != 'cases'}, indent=2))
        return 0
    except (ValueError, OSError, KeyError) as error:
        print(str(error), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
