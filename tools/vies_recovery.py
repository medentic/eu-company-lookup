#!/usr/bin/env python3
"""Bounded multi-country VIES recovery. Default is offline reporting; --execute is live."""
import argparse
from datetime import datetime, timezone
import fcntl
import json
import re
from pathlib import Path
import sys
import time
from types import SimpleNamespace
from urllib.request import urlopen

import vies_batch as batch
from vies_evidence import load_evidence, snapshot
from vies_reporting import build_report

STATUS_URL = 'https://ec.europa.eu/taxation_customs/vies/rest-api/check-status'


def now():
    return datetime.now(timezone.utc).isoformat()


def append(path, event):
    with path.open('a') as stream:
        stream.write(json.dumps({'at': now(), **event}) + '\n')


def selections(plan):
    """Validate the whole authorized selection before any network or mutation."""
    result = []
    countries = set()
    for item in plan['countries']:
        country = item['country']
        if country in countries:
            raise ValueError('duplicate country in recovery plan')
        countries.add(country)
        queue = Path(item['queue']).resolve()
        ids = batch.selection(queue, item.get('offset', 0), item['count'], country)
        result.append((country, queue, item.get('offset', 0), ids))
    if not result:
        raise ValueError('empty recovery plan')
    return result


def availability():
    with urlopen(STATUS_URL, timeout=10) as response:
        raw = response.read(65537)
    if len(raw) > 65536:
        raise ValueError('oversized availability response')
    data = json.loads(raw)
    if not isinstance(data['vow']['available'], bool):
        raise ValueError('invalid central availability')
    countries = {}
    for item in data['countries']:
        code, state = item['countryCode'], item['availability']
        if code in countries or state not in ('Available', 'Unavailable'):
            raise ValueError('invalid country availability')
        countries[code] = state
    return data, countries


def summary(root, selected, binary_manifest=None):
    current = snapshot(batch.ROOT, binary_manifest) if binary_manifest else None
    formats = batch.vat_formats()
    result = {}
    for country, _, offset, ids in selected:
        counts = dict(selected=len(ids), attempted=0, validObserved=0,
                      negativeObserved=0, providerErrors=0, verifiedValid=0, verifiedInvalid=0)
        for index, identifier in enumerate(ids, offset):
            folder = root / country / f'{index:04d}'
            if not (folder / 'manifest.json').exists():
                continue
            evidence = load_evidence(batch.ROOT, folder, formats, current=current)
            if evidence.ids != [identifier]:
                raise ValueError('batch selection changed')
            report = build_report(evidence)
            rows = evidence.rows
            counts['attempted'] += len(rows)
            if rows:
                response = rows[0]['attempts'][-1].get('response')
                if response:
                    counts['validObserved' if response['valid'] else 'negativeObserved'] += 1
                    case = report['cases'][0]
                    if case.get('verified'):
                        counts['verifiedValid' if response['valid'] else 'verifiedInvalid'] += 1
                else:
                    counts['providerErrors'] += 1
        counts['pending'] = len(ids) - counts['attempted']
        result[country] = counts
    if current and snapshot(batch.ROOT, binary_manifest) != current:
        raise ValueError('source/binary changed during report')
    return result


def next_delay(root, minimum=0):
    """Carry provider pacing across single-ID batches and process restarts."""
    latest = None
    for path in root.glob('*/*/observations.jsonl'):
        for row in batch.load_rows(path.parent):
            # Rust journals retain nanoseconds; Python 3.9 accepts microseconds.
            # Normalize only the working value, never the original observation.
            stamp = re.sub(r'(\.\d{6})\d+', r'\1', row['checkedAt'])
            at = datetime.fromisoformat(stamp.replace('Z', '+00:00'))
            gap = 5 if row['attempts'][-1].get('response') else 60
            end = at.timestamp() + gap
            latest = end if latest is None else max(latest, end)
    return max(minimum, 0 if latest is None else latest - time.time())


def run(args):
    plan = json.loads(args.plan.read_text())
    selected = selections(plan)
    root = args.run_dir.resolve()
    if not args.execute:
        return summary(root, selected, getattr(args, 'binary_manifest', None))
    if not 1 <= args.max_requests <= 100:
        raise ValueError('max-requests must be 1..100')
    if not args.binary or not args.binary.is_file():
        raise ValueError('--execute requires an existing test binary')
    # Bind normalized selections, not mutable file names alone.
    frozen = {'selections': [[c, str(q), o, ids] for c, q, o, ids in selected],
              'binarySha256': batch.sha(args.binary), 'dockerImage': args.docker_image,
              'deadlineSeconds': args.deadline_seconds}
    root.mkdir(parents=True, exist_ok=True)
    lockpath = batch.ROOT / 'target/vies-recovery.lock'
    lockpath.parent.mkdir(parents=True, exist_ok=True)
    with lockpath.open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        manifest = root / 'recovery-plan.json'
        if manifest.exists():
            if json.loads(manifest.read_text()) != frozen:
                raise ValueError('recovery selection, binary or policy changed; use a new run')
        else:
            batch.write_json(manifest, frozen)
        # Retry-After is a review boundary, including after a process restart.
        for path in root.glob('*/*/observations.jsonl'):
            if any(a.get('retryAfter') is not None for row in batch.load_rows(path.parent)
                   for a in row['attempts']):
                raise ValueError('Retry-After requires review; retain this run and prepare a new selection after waiting')
        journal = root / 'recovery-events.jsonl'
        append(journal, {'event': 'started', 'skip': args.skip, 'maxRequests': args.max_requests})
        requests = 0
        first = True
        for country, queue, offset, ids in selected:
            if country in args.skip:
                append(journal, {'event': 'deferred', 'country': country, 'reason': 'explicitSkip'})
                continue
            pending = []
            for index, identifier in enumerate(ids, offset):
                folder = root / country / f'{index:04d}'
                rows = batch.load_rows(folder)
                if rows and (len(rows) != 1 or rows[0]['vatId'] != identifier):
                    raise ValueError('unexpected batch observation identity')
                if not rows:
                    pending.append((index, folder))
            if not pending:
                continue
            try:
                data, states = availability()
                append(journal, {'event': 'availability', 'source': STATUS_URL, 'data': data})
                if not data['vow']['available']:
                    append(journal, {'event': 'deferred', 'reason': 'centralUnavailable'})
                    break
                if states.get(country) == 'Unavailable':
                    append(journal, {'event': 'deferred', 'country': country, 'reason': 'countryUnavailable'})
                    continue
            except (OSError, ValueError, KeyError, TypeError) as error:
                # Monitoring failure cannot establish that VAT validation is down.
                append(journal, {'event': 'availabilityUnknown', 'errorType': type(error).__name__})
            consecutive = 0
            for index, folder in pending:
                if requests >= args.max_requests:
                    break
                # Initial gap also protects against a preceding external collection run.
                time.sleep(next_delay(root, 60 if first else 0))
                first = False
                if batch.sha(args.binary) != frozen['binarySha256']:
                    raise ValueError('test binary changed during recovery; preserve the run and inspect before resuming')
                options = SimpleNamespace(country=country, queue=queue, offset=index, count=1,
                    run_dir=folder, binary=args.binary, docker_image=args.docker_image,
                    deadline_seconds=args.deadline_seconds, resume=(folder/'manifest.json').exists())
                code = batch.collect(options)
                requests += 1
                rows = batch.load_rows(folder)
                if code or len(rows) != 1:
                    append(journal, {'event': 'stopped', 'country': country, 'exitCode': code})
                    raise RuntimeError('collector stopped; inspect preserved evidence before resuming')
                if any(a.get('retryAfter') is not None for a in rows[0]['attempts']):
                    append(journal, {'event': 'stopped', 'country': country, 'reason': 'retryAfterReview'})
                    raise RuntimeError('Retry-After requires review before any further collection')
                failed = not rows[0]['attempts'][-1].get('response')
                consecutive = consecutive + 1 if failed else 0
                append(journal, {'event': 'observed', 'country': country, 'index': index,
                                 'providerError': failed})
                if consecutive >= 3:
                    append(journal, {'event': 'deferred', 'country': country,
                                     'reason': 'threeConsecutiveProviderErrors'})
                    break
            if requests >= args.max_requests:
                break
        result = summary(root, selected, getattr(args, 'binary_manifest', None))
        batch.write_json(root/'recovery-report.json', result)
        append(journal, {'event': 'passFinished', 'requests': requests, 'counts': result})
        return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--run-dir', type=Path, required=True)
    parser.add_argument('--execute', action='store_true')
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--binary-manifest', type=Path, help='current build manifest for offline verification')
    parser.add_argument('--docker-image')
    parser.add_argument('--deadline-seconds', type=int, choices=[6, 25], default=25)
    parser.add_argument('--max-requests', type=int, default=20)
    parser.add_argument('--skip', action='append', choices=sorted(batch.vat_formats()), default=[])
    args = parser.parse_args()
    try:
        print(json.dumps(run(args), indent=2))
        return 0
    except (OSError, ValueError, KeyError, TypeError, RuntimeError) as error:
        print(str(error), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
