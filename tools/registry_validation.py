"""Company validation classification, separate from parser/review agreement."""


RESOLVED_STATUSES = ('active', 'closed', 'liquidation', 'bankruptcy', 'receivership', 'liquidationAndReceivership', 'restructuring', 'pendingClosure', 'liquidationStayed', 'administration', 'inactive')

def classify(outcome, record, review_status, expected, *, found_outcomes=('found',),
             negative_outcomes=()):
    """Only a reviewed, known lifecycle can qualify a matched company case.

    Native closed records are positive identity matches, not registry misses.
    Negative observations require a provider-specific definitive-miss outcome;
    they are reported separately and are not automatically reviewed negatives.
    """
    if outcome == 'unresolvedData':
        return 'unresolved'
    if outcome in negative_outcomes:
        return ('verifiedNegative' if review_status == 'passed' and isinstance(expected, dict)
                and expected.get('outcome') == outcome else 'negativeObservation')
    if outcome not in found_outcomes:
        return 'failed'
    if not isinstance(record, dict) or record.get('status') not in RESOLVED_STATUSES:
        return 'unresolved'
    if review_status != 'passed' or not isinstance(expected, dict):
        return 'needsReview'
    if expected.get('status') != record['status']:
        return 'needsReview'
    return 'verifiedPositive'


def unresolved_detail(record):
    record = record or {}
    native = record.get('nativeRegistryStatus')
    return {'source': record.get('source'), 'nativeRegistryStatus': native,
            'reason': 'missingSourceStatus' if record.get('status') != 'unknown' and (native is None or str(native).strip() == '')
                      else 'unmappedSourceStatus'}
