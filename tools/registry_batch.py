#!/usr/bin/env python3
"""Commercial Register queue import and bounded, resumable development collection."""
import argparse
from contextlib import contextmanager
from collections import Counter
from datetime import datetime, timezone
import fcntl
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
POLICY = {'provider': 'BulgarianCommercialRegister', 'startIntervalSeconds': 30,
          'timeoutSeconds': 30, 'retries': 0, 'bulstatFallback': False}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix('.tmp')
    with temporary.open('w') as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.flush()
        os.fsync(stream.fileno())
    temporary.replace(path)


def import_queue(source, output):
    ids, seen, rejected, duplicates = [], set(), [], 0
    for line, raw in enumerate(source.read_text().splitlines(), 1):
        value = raw.strip()
        if not value:
            continue
        if not (value.isascii() and value.isdigit() and len(value) in (9, 13)):
            rejected.append({'line': line, 'reason': 'unsupportedSyntax'})
        elif value in seen:
            duplicates += 1
        else:
            seen.add(value)
            ids.append(value)
    result = {'schemaVersion': 1, 'sourceFile': source.name, 'sourceSha256': sha(source),
              'importedAt': datetime.now(timezone.utc).isoformat(), 'ids': ids,
              'duplicatesSkipped': duplicates, 'rejected': rejected}
    write(output, result)
    return {'queued': len(ids), 'duplicatesSkipped': duplicates, 'rejected': len(rejected)}


MISSING = object()


def differences(expected, actual):
    result = []
    for path, value in expected.items():
        found = field(actual, path)
        if found is MISSING or type(value) is not type(found) or value != found:
            result.append({'field': path, 'expected': value, 'actualPresent': found is not MISSING,
                           'actual': None if found is MISSING else found})
    return result


@contextmanager
def provider_guard(root=ROOT):
    """One collector per checkout, regardless of run directory; OS releases on exit."""
    path = root / 'target/registry-provider.lock'
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('a') as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as error:
            raise ValueError('Commercial Register collector already active in this checkout') from error
        state = path.with_suffix('.json')
        if not state.exists():
            write(state, {'nextStartAt': None, 'cooldownUntil': None})
        until = json.loads(state.read_text()).get('cooldownUntil')
        if until and datetime.fromisoformat(until.replace('Z', '+00:00')) > datetime.now(timezone.utc):
            raise ValueError('shared provider cooldown remains until ' + until)
        yield lock, state


def field(record, path):
    for part in path.split('.'):
        if not isinstance(record, dict) or part not in record:
            return MISSING
        record = record[part]
    return record


def report(folder):
    manifest = json.loads((folder / 'manifest.json').read_text())
    rows = []
    for tax_id in manifest['ids']:
        path = folder / 'observations' / (tax_id + '.json')
        if path.exists():
            row = json.loads(path.read_text())
            if row['taxId'] != tax_id or row['schemaVersion'] not in (1, 2):
                raise ValueError('observation identity/schema mismatch')
            rows.append(row)
    original_counts = Counter(row['outcome'] for row in rows)
    replay_path = folder / 'replay.json'
    replay = {}
    provenance = None
    if replay_path.exists():
        envelope = json.loads(replay_path.read_text())
        if isinstance(envelope, list):
            replay = {r['taxId']: r for r in envelope}
            provenance = {'schemaVersion': 1, 'bound': False}
        else:
            if envelope.get('schemaVersion') != 2:
                raise ValueError('unsupported replay schema')
            for key in ('parserSha256', 'binarySha256'):
                if len(envelope.get(key, '')) != 64 or any(c not in '0123456789abcdef' for c in envelope[key]):
                    raise ValueError('invalid replay provenance')
            provenance = {k:v for k,v in envelope.items() if k != 'observations'}
            provenance['bound'] = True
            for updated in envelope['observations']:
                if updated['taxId'] in replay:
                    raise ValueError('duplicate replay identity')
                original = folder / 'observations' / (updated['taxId'] + '.json')
                if updated['observationSha256'] != sha(original) or updated['sourceSha256'] != hashlib.sha256(updated['rawResponse'].encode()).hexdigest():
                    raise ValueError('replay source/observation hash mismatch')
                replay[updated['taxId']] = updated
    for row in rows:
        if row['taxId'] in replay:
            updated = replay[row['taxId']]
            if updated['rawResponse'] != row['rawResponse']:
                raise ValueError('replay source differs from original observation')
            row.update({k: updated[k] for k in ['record', 'outcome', 'error']})
    counts = Counter(row['outcome'] for row in rows)
    review_path = folder / 'review.json'
    reviews = json.loads(review_path.read_text()) if review_path.exists() else {}
    cases = []
    for row in rows:
        review = reviews.get(row['taxId'])
        diffs = []
        status = 'needsReview' if row['outcome'] == 'found' else row['outcome']
        if review and row['outcome'] == 'found':
            if review.get('schemaVersion') != 1 or review['status'] not in ('passed', 'ambiguous') or not review['expected']:
                raise ValueError('invalid review contract')
            digest = hashlib.sha256(row['rawResponse'].encode()).hexdigest()
            if review['sourceSha256'] != digest:
                status = 'sourceChanged'
            else:
                diffs = differences(review['expected'], row['record'])
                status = 'parserMismatch' if diffs else review['status']
        cases.append({'taxId': row['taxId'], 'outcome': row['outcome'], 'reviewStatus': status,
                      'companyName': (row.get('record') or {}).get('companyName'), 'elapsedMs': row['elapsedMs'],
                      'observationSchemaVersion': row['schemaVersion'], 'schedulerWaitMs': row.get('schedulerWaitMs'),
                      'transport': row.get('transport'), 'errorKind': row.get('errorKind'), 'unavailabilityReason': row.get('unavailabilityReason'), 'differences': diffs})
    result = {'schemaVersion': 1, 'selected': len(manifest['ids']), 'attempted': len(rows),
              'remaining': len(manifest['ids']) - len(rows), 'complete': len(rows) == len(manifest['ids']),
              'replayProvenance': provenance, 'initialOutcomes': dict(original_counts), 'outcomes': dict(counts), 'reviews': dict(Counter(c['reviewStatus'] for c in cases)), 'cases': cases}
    write(folder / 'report.json', result)
    return result


def collect(args):
    with provider_guard() as (lock, state):
        return collect_locked(args, lock, state)


def collect_locked(args, provider_lock, provider_state):
    policy = {**POLICY, 'startIntervalSeconds': args.interval}
    if not 10 <= args.interval <= 300:
        raise ValueError('collection interval must be 10..300 seconds')
    folder = args.run_dir.resolve()
    folder.mkdir(parents=True, exist_ok=True)
    with (folder / '.collector.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        queue = json.loads(args.queue.read_text())['ids']
        if args.offset < 0 or not 1 <= args.count <= 100:
            raise ValueError('offset >= 0 and count 1..100 required')
        selected = queue[args.offset:args.offset + args.count]
        if len(selected) != args.count:
            raise ValueError('selection exceeds queue')
        manifest_path = folder / 'manifest.json'
        if manifest_path.exists():
            if not args.resume:
                raise ValueError('run exists; use --resume')
            manifest = json.loads(manifest_path.read_text())
            if (manifest['ids'] != selected or manifest['queueSha256'] != sha(args.queue)
                    or manifest['dockerImage'] != args.docker_image or manifest['policy'] != policy):
                raise ValueError('resume selection/queue/image/policy changed')
        else:
            if list(folder.iterdir()) != [folder / '.collector.lock']:
                raise ValueError('new run directory must be empty')
            binary = args.binary.resolve()
            manifest = {'schemaVersion': 2, 'ids': selected, 'queueSha256': sha(args.queue),
                        'policy': policy, 'createdAt': datetime.now(timezone.utc).isoformat(),
                        'binarySha256': sha(binary), 'dockerImage': args.docker_image,
                        'sourceHashes': {str(p.relative_to(ROOT)): sha(p) for p in sorted((ROOT / 'src').rglob('*.rs'))}}
            shutil.copyfile(binary, folder / 'probe-bin')
            (folder / 'probe-bin').chmod(0o700)
            (folder / 'observations').mkdir()
            write(manifest_path, manifest)
        current = report(folder)
        if current['complete']:
            return current
        if manifest.get('schemaVersion') != 2:
            raise ValueError('legacy incomplete runs require a new coordinated run; original observations are preserved')
        for path in (folder / 'observations').glob('*.json'):
            until = json.loads(path.read_text()).get('resumeNotBefore')
            if until and datetime.fromisoformat(until.replace('Z', '+00:00')) > datetime.now(timezone.utc):
                raise ValueError('provider cooldown remains until ' + until)
        binary = folder / 'probe-bin'
        if sha(binary) != manifest['binarySha256']:
            raise ValueError('frozen binary changed')
        relative = folder.relative_to(ROOT)
        cmd = ['docker', 'run', '--rm', '--pull=never', '-v', str(ROOT) + ':/work', '-w', '/work',
               '-e', 'REGISTRY_RUN_DIR=/work/' + str(relative), '-e', 'REGISTRY_PROVIDER_STATE=/work/' + str(provider_state.relative_to(ROOT)), '--entrypoint', '/work/' + str(relative / 'probe-bin'),
               args.docker_image, 'live_commercial_batch', '--ignored', '--nocapture']
        try:
            subprocess.run(cmd, check=True, pass_fds=(provider_lock.fileno(),))
        finally:
            current = report(folder)
        if current['complete']:
            binary.unlink()
        return current


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='action', required=True)
    p = sub.add_parser('import')
    p.add_argument('source', type=Path)
    p.add_argument('output', type=Path)
    p = sub.add_parser('collect')
    p.add_argument('--queue', type=Path, required=True)
    p.add_argument('--run-dir', type=Path, required=True)
    p.add_argument('--binary', type=Path, required=True)
    p.add_argument('--docker-image', required=True)
    p.add_argument('--offset', type=int, default=0)
    p.add_argument('--count', type=int, default=10)
    p.add_argument('--interval', type=int, default=30, help='seconds between starts, frozen per run; 10..300')
    p.add_argument('--resume', action='store_true')
    p = sub.add_parser('report')
    p.add_argument('run_dir', type=Path)
    args = parser.parse_args()
    try:
        result = import_queue(args.source, args.output) if args.action == 'import' else collect(args) if args.action == 'collect' else report(args.run_dir)
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return 0 if result.get('complete', True) else 2
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        print(str(error), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
