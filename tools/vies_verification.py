"""Offline VIES review, replay proof and country coverage checks (stdlib only)."""
import argparse
import json
from pathlib import Path
import sys
from vies_evidence import (expected_hash, fingerprint, proof_valid, review_valid, sha,
                           sha_bytes, snapshot, source_bound, load_evidence, vat_formats)
from vies_reporting import build_report

ROOT = Path(__file__).resolve().parents[1]


def coverage(valid, invalid, required_valid=200, required_invalid=1):
    if min(valid, invalid) < 0 or required_valid < 200 or required_invalid < 1:
        raise ValueError('coverage requires at least 200 valid and 1 invalid')
    valid_gap, invalid_gap = max(0, required_valid-valid), max(0, required_invalid-invalid)
    return {'requiredValid': required_valid, 'requiredInvalid': required_invalid,
            'validAdditionalNeeded': valid_gap, 'invalidAdditionalNeeded': invalid_gap,
            'additionalNeeded': valid_gap+invalid_gap,
            'coverageGateMet': valid_gap == 0 and invalid_gap == 0}


def audit(root, folders, country, binary_manifest, *, formats=None):
    formats = vat_formats(root) if formats is None else formats
    current = snapshot(root, binary_manifest)
    positive, negative, failed, unverified = set(), set(), set(), set()
    observed_positive, observed_negative = set(), set()
    pending = 0
    for folder in folders:
        evidence = load_evidence(root, folder, formats, current=current)
        report = build_report(evidence)
        pending += report.get('pending', 0)
        rows = {r['vatId']: r for r in evidence.rows}
        for case in report['cases']:
            identifier = case['vatId']
            if identifier[:2] != country:
                raise ValueError('observation outside selected country')
            response = rows[identifier]['attempts'][-1].get('response')
            if not response:
                failed.add(identifier)
                continue
            if type(response.get('valid')) is bool:
                (observed_positive if response['valid'] else observed_negative).add(identifier)
            if not case['verified']:
                unverified.add(identifier)
            else:
                (positive if response['valid'] is True else negative).add(identifier)
    if snapshot(root, binary_manifest) != current:
        raise ValueError('source/binary changed during audit')
    conflicts = observed_positive & observed_negative
    valid, invalid = len(positive-conflicts), len(negative-conflicts)
    result = {'country': country, 'verifiedValid': valid, 'verifiedInvalid': invalid,
              'conflictingValidity': len(conflicts), 'providerFailedDistinct': len(failed),
              'awaitingReviewOrReplay': len(unverified), 'pending': pending, **coverage(valid, invalid)}
    result['accepted'] = result['coverageGateMet'] and not conflicts and not result['awaitingReviewOrReplay'] and not pending
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--country', required=True, choices=sorted(vat_formats(ROOT)))
    parser.add_argument('--run-dir', type=Path, action='append', required=True)
    parser.add_argument('--binary-manifest', type=Path, required=True)
    args = parser.parse_args()
    try:
        result = audit(ROOT, args.run_dir, args.country, args.binary_manifest)
        print(json.dumps(result, indent=2))
        return 0 if result['accepted'] else 1
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(str(error), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
