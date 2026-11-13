import json
from pathlib import Path
import tempfile
import unittest
from ro_batch import select, timestamp


class RomanianSelectionTests(unittest.TestCase):
    def test_selection_preserves_zeroes_and_requires_provenance(self):
        with tempfile.TemporaryDirectory() as directory:
            p = Path(directory) / 'queue.json'
            cases = [{'taxId': '001234', 'sourceUrl': 'https://example.org/company'}]
            p.write_text(json.dumps({'cases': cases}))
            self.assertEqual(select(p, 0, 1), cases)
            cases[0]['sourceUrl'] = ''
            p.write_text(json.dumps({'cases': cases}))
            with self.assertRaises(ValueError): select(p, 0, 1)

    def test_rejects_duplicate_invalid_and_unbounded_selections(self):
        with tempfile.TemporaryDirectory() as directory:
            p = Path(directory) / 'queue.json'
            good = {'taxId': '1234', 'sourceUrl': 'https://example.org/company'}
            p.write_text(json.dumps({'cases': [good]}))
            for offset, count in [(-1, 1), (0, 0), (0, 101), (1, 1)]:
                with self.assertRaises(ValueError): select(p, offset, count)
            for cases in [[good, good], [{**good, 'taxId': 'RO1234'}], [{**good, 'taxId': '１２３４'}]]:
                p.write_text(json.dumps({'cases': cases}))
                with self.assertRaises(ValueError): select(p, 0, 1)

class TimestampTests(unittest.TestCase):
    def test_nanosecond_timestamps_round_up_for_conservative_pacing(self):
        self.assertEqual(timestamp('2026-11-13T15:28:36.123456789Z'),
                         timestamp('2026-11-13T15:28:36.123457Z'))
        self.assertEqual(timestamp('2026-11-13T15:28:36Z'),
                         timestamp('2026-11-13T15:28:36+00:00'))
        with self.assertRaises(ValueError): timestamp('2026-11-13T15:28:36')
