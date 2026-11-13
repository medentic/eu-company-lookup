"""Check Cargo package contents and consumer-documentation links."""
import argparse
from pathlib import Path
import subprocess
from check_docs import ROOT, check

CONSUMER_DOCS = {'README.md', 'CHANGELOG.md', 'docs/reference/api-stability.md',
                 'docs/reference/bulgarian-registry.md', 'docs/reference/romanian-registry.md', 'docs/reference/vies.md'}
ROOT_FILES = {'Cargo.toml', 'Cargo.toml.orig', 'Cargo.lock', '.cargo_vcs_info.json',
              'LICENSE', 'THIRD-PARTY-LICENSE.txt'}
SYNTHETIC_FIXTURES = {'src/vies/fixtures/valid.xml',
                      'src/vies/fixtures/report-contract.json',
                      'src/vies/fixtures/review-policy.json'}
REQUIRED = CONSUMER_DOCS | SYNTHETIC_FIXTURES | {
    'Cargo.toml', 'Cargo.lock', 'LICENSE', 'THIRD-PARTY-LICENSE.txt', 'src/lib.rs',
    'tests/public_api.rs', 'tests/configuration.rs', 'tests/identifiers.rs',
}


def validate_files(files):
    """Allow source code and explicitly reviewed consumer/fixture files only."""
    forbidden = sorted(path for path in files if not (
        path in CONSUMER_DOCS | ROOT_FILES | SYNTHETIC_FIXTURES
        or (path.startswith(('src/', 'tests/', 'examples/')) and path.endswith('.rs'))
    ))
    missing = sorted(REQUIRED - files)
    return missing, forbidden


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--file-list', type=Path, help='Saved cargo package --list output')
    args = parser.parse_args()
    listing = args.file_list.read_text() if args.file_list else subprocess.check_output(
        ['cargo', 'package', '--locked', '--allow-dirty', '--list'], text=True, cwd=ROOT
    )
    files = set(listing.splitlines())
    missing, forbidden = validate_files(files)
    if missing or forbidden:
        raise SystemExit(f'Invalid distribution: missing={missing}, forbidden={forbidden}')
    errors = check(ROOT, files, packaged=True)
    if errors:
        raise SystemExit('\n'.join(errors))
    print(f'Distribution verified: {len(files)} files; consumer links resolve; development evidence excluded.')


if __name__ == '__main__':
    main()
