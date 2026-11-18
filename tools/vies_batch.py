#!/usr/bin/env python3
"""Bounded, resumable VIES development collection and offline reports (stdlib only)."""
import argparse
import fcntl
import json
import os
import re
from pathlib import Path
import subprocess
import sys
from datetime import datetime, timezone
from vies_reporting import Facts, classify, build_report
from vies_evidence import load_evidence, load_rows, sha, snapshot, vat_formats as read_formats
from vies_binary import freeze_binary as store_binary

ROOT = Path(__file__).resolve().parents[1]
BINARY_CACHE = ROOT / 'target/vies-probe-binaries'
POLICY = {"deadlineSeconds": 6, "maxAttempts": 3, "retryStatus": 502,
          "retryPausesSeconds": [0, 1], "successGapSeconds": 5, "errorGapSeconds": 60}


def freeze_binary(source):
    return store_binary(source, BINARY_CACHE)


def write_json(path, value):
    temp = path.with_suffix(path.suffix + '.tmp')
    temp.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')
    temp.replace(path)


def vat_formats(root=ROOT):
    return read_formats(root)


def selection(path, offset, count, country="BG", *, formats=None):
    formats = vat_formats() if formats is None else formats
    ids = json.loads(path.read_text())
    if not isinstance(ids, list):
        raise ValueError("queue must be a JSON array")
    if offset < 0 or not 1 <= count <= 100 or offset + count > len(ids):
        raise ValueError('offset/count outside queue; count must be 1..100')
    selected = ids[offset:offset + count]
    if country not in formats:
        raise ValueError('unsupported collector country')
    if (not isinstance(ids, list) or any(not isinstance(x, str) or not x.startswith(country) or re.fullmatch(formats[country], x[2:]) is None for x in ids) or len(set(ids)) != len(ids)):
        raise ValueError('queue contains duplicate or malformed country identifiers')
    return selected


def report(folder, *, current=None, root=None, formats=None):
    """Collector/report command boundary: evaluate, then explicitly save its report."""
    root = ROOT if root is None else root
    formats = vat_formats() if formats is None else formats
    result = build_report(load_evidence(root, folder, formats, current=current))
    write_json(folder / 'report.json', result)
    return result


def collect(args):
    folder = args.run_dir.resolve()
    folder.mkdir(parents=True, exist_ok=True)
    # OS-released lock protects manifests and snapshots even if the wrapper crashes.
    with (folder / '.collector.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        manifest_path = folder / 'manifest.json'
        deadline = getattr(args, 'deadline_seconds', 6)
        if deadline not in (6, 25):
            raise ValueError('unsupported collector deadline')
        policy = {**POLICY, 'deadlineSeconds': deadline}
        selected = selection(args.queue, args.offset, args.count, getattr(args, "country", "BG"))
        if manifest_path.exists():
            if not args.resume:
                raise ValueError('run already exists; use --resume')
            manifest = json.loads(manifest_path.read_text())
            if manifest['policy'] != policy:
                raise ValueError('resume collection policy changed')
            if manifest['ids'] != selected or manifest['queueSha256'] != sha(args.queue):
                raise ValueError('resume queue/selection changed')
        else:
            if list(folder.glob('*.jsonl')):
                raise ValueError('unmanaged journals exist; choose a fresh run directory')
            binary = freeze_binary(args.binary.resolve())
            source = {str(p.relative_to(ROOT)): sha(p) for p in sorted((ROOT / 'src').rglob('*.rs'))}
            source.update({p: sha(ROOT / p) for p in ['Cargo.toml', 'Cargo.lock']})
            manifest = {'ids': selected, 'offset': args.offset, 'queueSha256': sha(args.queue),
                        'binarySha256': sha(binary), 'sourceHashes': source,
                        'binaryPath': os.path.relpath(binary, ROOT),
                        'policy': policy, 'journalSchemaVersion': 2,
                        'reviewPolicy': json.loads((ROOT / 'src/vies/fixtures/review-policy.json').read_text()),
                        'createdAt': datetime.now(timezone.utc).isoformat(),
                        'gitHead': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
                        'dockerImage': args.docker_image}
            write_json(manifest_path, manifest)
        current = report(folder)
        if current['complete']:
            print(json.dumps({k: v for k, v in current.items() if k != 'cases'}, indent=2))
            return 0
        binary = (ROOT / manifest['binaryPath']).resolve() if 'binaryPath' in manifest else folder / 'probe-bin'
        if sha(binary) != manifest['binarySha256']:
            raise ValueError('frozen binary changed')
        if args.docker_image != manifest['dockerImage']:
            raise ValueError('resume Docker image changed')
        journal = folder / 'observations.jsonl'
        env = {'VIES_PROBE_IDS': ','.join(selected), 'VIES_PROBE_RESUME': '1'}
        test_args = ['live_six_second_probe' if deadline == 6 else 'live_extended_probe', '--ignored', '--nocapture']
        if args.docker_image:
            relative = folder.relative_to(ROOT)
            env['VIES_PROBE_REPORT'] = '/work/' + str(relative / journal.name)
            cmd = ['docker', 'run', '--rm', '--pull=never', '--entrypoint', '/work/' + str(binary.relative_to(ROOT)),
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
        if current['complete'] and 'binaryPath' not in manifest:
            binary.unlink()  # Required only while a resumable run remains incomplete.
        return code if code else (0 if current['complete'] else 2)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='action', required=True)
    p = sub.add_parser('collect')
    p.add_argument('--country', choices=sorted(vat_formats()), default='BG')
    p.add_argument('--deadline-seconds', type=int, choices=[6, 25], default=6,
                   help='6-second experiment (default) or explicit 25-second recovery profile')
    p.add_argument('--queue', type=Path, default=ROOT / 'src/vies/fixtures/ids/bg.json')
    p.add_argument('--offset', type=int, required=True, help='zero-based; 100 starts at ID 101')
    p.add_argument('--count', type=int, default=100)
    p.add_argument('--run-dir', type=Path, required=True)
    p.add_argument('--binary', type=Path, required=True, help='compiled Rust library test executable')
    p.add_argument('--docker-image', help='existing local Linux builder image; no pull')
    p.add_argument('--resume', action='store_true')
    p = sub.add_parser('report')
    p.add_argument('run_dir', type=Path)
    p.add_argument('--binary-manifest', type=Path, help='current build manifest for verified counts')
    args = parser.parse_args()
    try:
        if args.action == 'collect':
            return collect(args)
        current = snapshot(ROOT, args.binary_manifest) if args.binary_manifest else None
        print(json.dumps({k: v for k, v in report(args.run_dir, current=current).items() if k != 'cases'}, indent=2))
        return 0
    except (ValueError, OSError, KeyError) as error:
        print(str(error), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
