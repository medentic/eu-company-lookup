"""Offline checks of the differential runner; no compiler or oracle install needed."""
import unittest

from check_vat_checksums import SEEDS, candidates, compare


class ChecksumComparisonTests(unittest.TestCase):
    def test_generated_inputs_are_repeatable_and_cover_all_prefixes(self):
        ids = candidates()
        self.assertEqual(ids, candidates())
        self.assertEqual(len(ids), len(set(ids)))
        self.assertTrue(set(SEEDS) <= set(ids))
        self.assertEqual({s[:2] for s in ids}, set(
            'AT BE BG CY CZ DE DK EE EL ES FI FR HR HU IE IT LT LU LV MT NL PL PT RO SE SI SK XI'.split()))
        self.assertIn('IE1*27456B', ids)
        self.assertIn('NL123456789B14', ids)
        self.assertFalse(any(s.endswith('B00') for s in ids if s.startswith('NL')))
        self.assertTrue(all(s.endswith('01') for s in ids if s.startswith('SE')))
        self.assertTrue(all(s.endswith('001') for s in ids if s.startswith('XI') and len(s) == 14))

    def test_disagreement_or_unsupported_verdict_fails(self):
        ids = ['RO19', 'RO10']
        oracle = lambda value: value == 'RO19'
        self.assertEqual(compare(ids, {'RO19': 'pass', 'RO10': 'fail'}, oracle),
                         {'pass': 1, 'fail': 1})
        for verdict in ['pass', 'unsupported', 'unknown']:
            with self.subTest(verdict=verdict), self.assertRaisesRegex(ValueError, 'disagreements'):
                compare(ids, {'RO19': 'pass', 'RO10': verdict}, oracle)

    def test_missing_or_extra_results_fail(self):
        for results in [{}, {'RO19': 'pass', 'RO10': 'fail'}]:
            with self.subTest(results=results), self.assertRaisesRegex(ValueError, 'result set'):
                compare(['RO19'], results, lambda _: True)


if __name__ == '__main__':
    unittest.main()
