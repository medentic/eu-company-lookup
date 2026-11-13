"""Reject collected data in the Git index and, optionally, reachable history.

This enforces a file boundary, not anonymization or a general secret scanner.
Review source literals and newly allowed fixture files before publishing them.
"""
import argparse
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
PUBLIC_FIXTURES = {
    'src/vies/fixtures/valid.xml',
    'src/vies/fixtures/report-contract.json',
    'src/vies/fixtures/review-policy.json',
}


def forbidden_files(paths):
    return sorted(path for path in paths if (
        ('fixtures/' in path and path not in PUBLIC_FIXTURES)
        or (path.endswith(('.json', '.jsonl', '.log', '.crate')) and path not in PUBLIC_FIXTURES)
        or path.startswith('target/')
    ))


def history_paths(root, revision):
    output = subprocess.check_output(
        ['git', 'rev-list', '--objects', revision, '--'], cwd=root, text=True
    )
    return {line.split(' ', 1)[1] for line in output.splitlines() if ' ' in line}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--history', help='Also check all ancestors of this Git revision')
    args = parser.parse_args()
    tracked = set(subprocess.check_output(
        ['git', 'ls-files', '-z'], cwd=ROOT, text=True
    ).rstrip('\0').split('\0'))
    forbidden = forbidden_files(tracked)
    missing = sorted(PUBLIC_FIXTURES - tracked)
    if forbidden or missing:
        raise SystemExit(f'Invalid public source: forbidden={forbidden}, missing={missing}')
    if args.history:
        # rev-list includes tree names too; only blobs have extensions in this policy.
        forbidden = forbidden_files(history_paths(ROOT, args.history))
        if forbidden:
            raise SystemExit(f'Collected data remains in reachable history: {forbidden}')
    print('Public source boundary verified: only approved synthetic fixtures are tracked'
          + ('; reachable history checked.' if args.history else '.'))


if __name__ == '__main__':
    main()
