import unittest
from registry_validation import classify


class RegistryValidationTests(unittest.TestCase):
    def test_null_unknown_and_missing_status_are_neither_positive_nor_negative(self):
        for record in [None, {}, {'status': None}, {'status': 'unknown'}, {'status': ''}]:
            self.assertEqual(classify('found', record, 'passed', record), 'unresolved')

    def test_closed_company_is_a_positive_match_and_requires_status_review(self):
        for status in ['active', 'closed']:
            record = {'status': status}
            self.assertEqual(classify('found', record, 'passed', record), 'verifiedPositive')
            self.assertEqual(classify('found', record, 'passed', {'companyName': 'Example'}), 'needsReview')
            self.assertEqual(classify('found', record, 'ambiguous', record), 'needsReview')

    def test_negative_requires_provider_specific_definitive_outcome(self):
        kwargs = {'negative_outcomes': ('notFoundInCommercialRegister',)}
        self.assertEqual(classify('notFoundInCommercialRegister', None, None, None, **kwargs), 'negativeObservation')
        for outcome in ['providerError', 'parserError', 'emptyActiveSearch', 'unknown', None]:
            self.assertEqual(classify(outcome, None, None, None, **kwargs), 'failed')

    def test_reviewed_negative_requires_exact_outcome_and_passing_review(self):
        kwargs = {'negative_outcomes': ('notFoundInCommercialRegister',)}
        expected = {'outcome': 'notFoundInCommercialRegister'}
        self.assertEqual(classify('notFoundInCommercialRegister', None, 'passed', expected, **kwargs), 'verifiedNegative')
        self.assertEqual(classify('notFoundInCommercialRegister', None, 'ambiguous', expected, **kwargs), 'negativeObservation')
        self.assertEqual(classify('providerError', None, 'passed', expected, **kwargs), 'failed')

    def test_unresolved_details_preserve_source_status(self):
        from registry_validation import unresolved_detail
        self.assertEqual(unresolved_detail({})['reason'], 'missingSourceStatus')
        detail = unresolved_detail({'source': 'Example', 'nativeRegistryStatus': 'Pending'})
        self.assertEqual(detail['reason'], 'unmappedSourceStatus')
        self.assertEqual(detail['nativeRegistryStatus'], 'Pending')

    def test_reviewed_proceedings_are_positive_identity_cases(self):
        from registry_validation import RESOLVED_STATUSES
        for status in RESOLVED_STATUSES:
            record = {'status': status}
            self.assertEqual(classify('found', record, 'passed', record), 'verifiedPositive')

    def test_runtime_unresolved_error_is_not_a_provider_failure_or_negative(self):
        self.assertEqual(classify('unresolvedData', None, None, None), 'unresolved')
