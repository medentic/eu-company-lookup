"""Offline differential check of local Rust arithmetic against python-stdnum 2.2.

Only synthetic, supported checksum variants are compared. Check-digit mutations
keep dates/body formats unchanged: stdnum also checks constraints our preflight
deliberately leaves to VIES. This tool never reads or writes collected evidence.
"""
import importlib.metadata
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
ORACLE_VERSION = '2.2'
# Synthetic seeds, including both Dutch schemes and Irish legacy variants.
SEEDS = (
    'ATU82345678', 'BE0123452789', 'BG423456789', 'CY92345678A',
    'CZ82345678', 'DE423456789', 'DK13345678', 'EE823456789',
    'EL323456789', 'ESA42345678', 'FI02345678', 'FR11123456782',
    'HR02345678901', 'HU32345678', 'IE1234667A', 'IT82345670010',
    'LT723456710', 'LU78345678', 'LV40000000000', 'MT12845678',
    'NL323456789B01', 'PL3234567890', 'PT123456789', 'RO19',
    'SE923456789001', 'SI82345678', 'SK7234567890', 'XI723456789',
    'BE1123456770', 'BG9001011232', 'BG9001011237', 'BG9001011238',
    'CZ612345670', 'CZ5401011231', 'ES14345678A', 'ESX6234567A',
    'IE1A27456A', 'IE1*27456A', 'IE1+27456A', 'IE9234567AA',
    'LT023456789010', 'NL123456789B13', 'XI723456789001',
)


def candidates():
    """Deterministic single check-character mutations, retaining original seeds."""
    result = set(SEEDS)
    for seed in SEEDS:
        prefix = seed[:2]
        positions = {
            'FR': (2, 3), 'BE': (10, 11), 'LU': (8, 9), 'MT': (8, 9),
            'NL': (10, 12, 13), 'SE': (11,), 'IE': (9,), 'XI': (9, 10),
        }.get(prefix, (len(seed) - 1,))
        for position in positions:
            alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ' if seed[position].isalpha() else '0123456789'
            for character in alphabet:
                candidate = seed[:position] + character + seed[position + 1:]
                # stdnum rejects branch 00 as a format constraint, independently
                # of the checksum. Our preflight deliberately leaves that to VIES.
                if prefix == 'NL' and candidate.endswith('B00'):
                    continue
                result.add(candidate)
    return sorted(result)


def rust_verdicts(ids):
    """Compile the actual private rules, without Cargo dependencies or API changes."""
    rules = json.dumps(str(ROOT / 'src/vies/checksum/rules.rs'))
    source = '''
#[path = RULES_PATH]
mod rules;
use std::io::{self, BufRead};
fn main() {
    for line in io::stdin().lock().lines() {
        let id = line.unwrap();
        let verdict = match rules::check(&id[..2], &id[2..]) {
            Some(true) => "pass", Some(false) => "fail", None => "unsupported",
        };
        println!("{}\\t{}", id, verdict);
    }
}
'''.replace('RULES_PATH', rules)
    with tempfile.TemporaryDirectory(prefix='vat-checksum-') as directory:
        directory = Path(directory)
        probe = directory / 'probe.rs'
        executable = directory / 'probe'
        probe.write_text(source)
        subprocess.run(['rustc', '--edition=2024', str(probe), '-o', str(executable)],
                       check=True, capture_output=True, text=True, timeout=120)
        result = subprocess.run([str(executable)], input='\n'.join(ids) + '\n',
                                check=True, capture_output=True, text=True, timeout=30)
    rows = [line.split('\t') for line in result.stdout.splitlines()]
    if [row[0] for row in rows] != ids or any(len(row) != 2 for row in rows):
        raise ValueError('Rust probe returned incomplete or unexpected identifiers')
    return {row[0]: row[1] for row in rows}


def compare(ids, actual, oracle):
    """Fail closed on missing, unsupported or disagreeing results."""
    if set(actual) != set(ids):
        raise ValueError('Rust probe result set differs from selected synthetic inputs')
    mismatches = []
    counts = {'pass': 0, 'fail': 0}
    for identifier in ids:
        expected = 'pass' if oracle(identifier) else 'fail'
        counts[expected] += 1
        if actual[identifier] != expected:
            mismatches.append(f'{identifier}: Rust={actual[identifier]}, stdnum={expected}')
    if mismatches:
        raise ValueError(f'{len(mismatches)} checksum disagreements:\n' + '\n'.join(mismatches[:10]))
    return counts


def main():
    try:
        version = importlib.metadata.version('python-stdnum')
    except importlib.metadata.PackageNotFoundError:
        raise SystemExit('Install development-only python-stdnum==2.2 in an isolated Python environment.')
    if version != ORACLE_VERSION:
        raise SystemExit(f'Expected python-stdnum {ORACLE_VERSION}, found {version}; do not silently change the oracle.')
    from stdnum.eu import vat
    if not all(vat.is_valid(seed) for seed in SEEDS):
        raise SystemExit('An oracle seed failed validation; inspect it before changing expectations.')
    ids = candidates()
    try:
        counts = compare(ids, rust_verdicts(ids), vat.is_valid)
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        detail = getattr(error, 'stderr', '') or str(error)
        raise SystemExit(detail[:4000]) from error
    print(f'python-stdnum {version}: {len(ids)} synthetic cases across '
          f'{len({seed[:2] for seed in SEEDS})} prefixes agree '
          f'({counts["pass"]} checksum passes, {counts["fail"]} failures). No live requests.')


if __name__ == '__main__':
    main()
