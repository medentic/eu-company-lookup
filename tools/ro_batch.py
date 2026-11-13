#!/usr/bin/env python3
"""Freeze and run an explicitly authorized, local-only ANAF selection."""
import argparse
import fcntl
import hashlib
import json
from pathlib import Path
import shutil
import shlex
import time
import re
from email.utils import parsedate_to_datetime
import subprocess
from datetime import datetime, timezone, timedelta
from zoneinfo import ZoneInfo

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path, value):
    tmp = path.with_suffix(path.suffix + '.tmp')
    tmp.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')
    tmp.replace(path)


def select(queue, offset, count):
    cases = json.loads(queue.read_text())['cases']
    if not 1 <= count <= 100 or offset < 0 or offset + count > len(cases):
        raise ValueError('selection must contain 1..100 existing cases')
    ids = [c['taxId'] for c in cases]
    if len(ids) != len(set(ids)) or any(not isinstance(s, str) or not s.isascii()
            or not s.isdigit() or not 2 <= len(s) <= 10 for s in ids):
        raise ValueError('invalid or duplicate CUI')
    if any(not c.get('sourceUrl', '').startswith('https://') for c in cases):
        raise ValueError('public provenance required')
    return cases[offset:offset + count]


def timestamp(value):
    """Read Rust RFC3339 nanoseconds on Python 3.9, rounding pacing upward."""
    value = value.replace('Z', '+00:00')
    pattern = r'(\.\d{6})\d+(?=[+-])'
    rounded = bool(re.search(pattern, value))
    parsed = datetime.fromisoformat(re.sub(pattern, r'\1', value))
    if parsed.tzinfo is None:
        raise ValueError('observation timestamp needs a timezone')
    return (parsed + timedelta(microseconds=int(rounded))).timestamp()


def collect(args):
    folder = args.run_dir.resolve()
    folder.relative_to(ROOT / 'target')
    folder.mkdir(parents=True, exist_ok=True)
    with (ROOT / 'target/anaf-collector.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        state_path = ROOT / 'target/anaf-provider-state.json'
        if state_path.exists():
            state = json.loads(state_path.read_text())
            time.sleep(max(0, state['nextAllowed'] - time.time()))
        cases = select(args.queue, args.offset, args.count)
        manifest_path = folder / 'manifest.json'
        frozen = folder / 'probe-bin'
        if manifest_path.exists():
            if not args.resume:
                raise ValueError('existing run; explicit --resume required')
            manifest = json.loads(manifest_path.read_text())
            if manifest['cases'] != cases or manifest['queueSha256'] != sha(args.queue) or manifest['dockerImage'] != args.docker_image:
                raise ValueError('resume provenance or image changed')
            if not frozen.exists():
                raise ValueError('completed frozen binary removed; use offline replay')
        else:
            if (folder / 'observations.jsonl').exists():
                raise ValueError('unmanaged observations; preserve and inspect')
            manifest = {'schemaVersion': 1, 'ids': [c['taxId'] for c in cases],
                'cases': cases, 'queueSha256': sha(args.queue), 'binarySha256': sha(args.binary),
                'requestDate': datetime.now(ZoneInfo('Europe/Bucharest')).date().isoformat(),
                'createdAt': datetime.now(timezone.utc).isoformat(),
                'dockerImage': args.docker_image,
                'sourceHashes': {str(p.relative_to(ROOT)): sha(p) for p in sorted((ROOT/'src').rglob('*.rs'))},
                'policy': {'maxRequests': len(cases), 'gapSeconds': 5, 'stopOnError': True, 'automaticRetries': False}}
            shutil.copyfile(args.binary, frozen)
            frozen.chmod(0o700)
            write_json(manifest_path, manifest)
        if sha(frozen) != manifest['binarySha256']:
            raise ValueError('frozen executable changed')
        # All invocations require manual review after errors/Retry-After; resuming
        # never reissues an attempted identifier. The Rust journal also preserves pacing.
        command = ['docker', 'run', '--rm', '--pull=never', '--entrypoint', 'sh',
            '-v', f'{ROOT}:/work', '-w', '/work', '-e', f'RO_RUN_DIR={folder.relative_to(ROOT)}',
            args.docker_image, '-c', shlex.join([str(frozen.relative_to(ROOT)), 'ro::probe::live_anaf_batch', '--exact', '--ignored', '--nocapture'])]
        write_json(state_path, {'nextAllowed': time.time() + 90, 'runDir': str(folder.relative_to(ROOT)), 'interruptedReservation': True})
        with (folder / 'runner.log').open('ab') as log:
            if log.tell() > 20 * 1024 * 1024:
                raise ValueError('runner log bound exceeded')
            code = subprocess.call(command, stdout=log, stderr=subprocess.STDOUT, cwd=ROOT)
        journal = folder / 'observations.jsonl'
        if journal.exists():
            text = journal.read_text()
            if text and not text.endswith('\n'):
                raise ValueError('partial journal tail; preserve and inspect')
            if text:
                row = json.loads(text.splitlines()[-1])
                completed = timestamp(row['completedAt'])
                until = completed + (60 if 'error' in row else 5)
                retry = (row.get('transport') or {}).get('retryAfter')
                if retry:
                    until = max(until, completed + int(retry) if retry.isdigit() else parsedate_to_datetime(retry).timestamp())
                write_json(state_path, {'nextAllowed': until, 'runDir': str(folder.relative_to(ROOT)), 'interruptedReservation': False})
        if code == 0:
            frozen.unlink()
        return code


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--queue', type=Path, required=True)
    parser.add_argument('--offset', type=int, required=True)
    parser.add_argument('--count', type=int, required=True)
    parser.add_argument('--run-dir', type=Path, required=True)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--docker-image', required=True)
    parser.add_argument('--resume', action='store_true')
    raise SystemExit(collect(parser.parse_args()))


if __name__ == '__main__':
    main()
