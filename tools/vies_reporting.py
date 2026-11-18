"""Report statuses shared with Rust via fixtures/report-contract.json."""
import collections
import math
import statistics
from vies_evidence import review_valid
from dataclasses import dataclass
from enum import Enum

class Status(str, Enum):
    PENDING = 'pending'
    PROVIDER_ERROR = 'providerError'
    NEEDS_REVIEW = 'needsReview'
    VALIDITY_CHANGED = 'validityChanged'
    SOURCE_CHANGED = 'sourceChanged'
    PARSER_MISMATCH = 'parserMismatch'
    AMBIGUOUS = 'ambiguous'
    PASSED = 'passed'

@dataclass(frozen=True)
class Facts:
    observed: bool = False
    response: bool = False
    expectation: bool = False
    validityChanged: bool = False
    sourceChanged: bool = False
    parserMismatch: bool = False
    ambiguous: bool = False


def classify(f: Facts) -> Status:
    if not f.observed: return Status.PENDING
    if not f.response: return Status.PROVIDER_ERROR
    if not f.expectation: return Status.NEEDS_REVIEW
    if f.validityChanged: return Status.VALIDITY_CHANGED
    if f.sourceChanged: return Status.SOURCE_CHANGED
    if f.parserMismatch: return Status.PARSER_MISMATCH
    if f.ambiguous: return Status.AMBIGUOUS
    return Status.PASSED


def build_report(evidence):
    """Evaluate an already loaded run without reading or writing files."""
    ids, rows = evidence.ids, evidence.rows
    fixtures, reviews = evidence.fixtures, evidence.reviews
    policy, replay, proof_ok = evidence.policy, evidence.replay, evidence.proof_ok
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
                observed=True, response=True, expectation=review_valid(fixture, review, policy),
                validityChanged=bool(fixture and fixture['valid'] != response['valid']),
                sourceChanged=bool(fixture and fixture['viesOutput'] != {'company': response['company'], 'address': response['address']['fullAddressByVies']}),
                parserMismatch=bool(diffs), ambiguous=review.get('status') == 'ambiguous',
            )).value
        verified = (proof_ok and status == 'passed' and row['vatId'] in replay
                    and type(response.get('valid')) is bool
                    and evidence.bindings.get(row['vatId'], False))
        cases.append({'vatId': row['vatId'], 'status': status, 'differences': diffs,
                      'verified': bool(verified),
                      'reviewNote': review.get('note'), 'reviewVersion': review.get('reviewVersion'),
                      'reviewedAt': review.get('reviewedAt'), 'normalizationVersion': review.get('normalizationVersion'),
                      'error': last.get('error'),
                      'totalMs': row['totalMs'], 'parsedFrom': 'offlineReplay' if row['vatId'] in replay else 'liveObservation'})
    counts = collections.Counter(c['status'] for c in cases)
    times = sorted(r['totalMs'] for r in rows)
    passed = counts['passed']
    result = {'selected': len(ids), 'completed': len(rows), 'pending': len(ids) - len(rows),
              'complete': len(ids) == len(rows), 'statuses': dict(counts),
              'verifiedValid': sum(c['verified'] and rows[n]['attempts'][-1]['response']['valid'] is True for n, c in enumerate(cases)),
              'verifiedInvalid': sum(c['verified'] and rows[n]['attempts'][-1]['response']['valid'] is False for n, c in enumerate(cases)),
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
    return result
