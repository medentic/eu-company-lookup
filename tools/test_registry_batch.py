import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from registry_batch import import_queue, report, write


class RegistryBatchTests(unittest.TestCase):
    def test_import_preserves_leading_zeroes_deduplicates_and_reports_bad_lines(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            source = root / 'input.txt'
            source.write_text(' 000123456\n000123456\n1234567890\n１２３４５６７８９\n1234567890123\n')
            result = import_queue(source, root / 'queue.json')
            self.assertEqual(result, {'queued': 2, 'duplicatesSkipped': 1, 'rejected': 2})
            self.assertEqual(json.loads((root / 'queue.json').read_text())['ids'], ['000123456', '1234567890123'])

    def test_observation_is_not_a_review_and_replay_cannot_change_source(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            write(root / 'manifest.json', {'ids': ['123456789', '987654321']})
            row = {'schemaVersion': 1, 'taxId': '123456789', 'outcome': 'found', 'record': {'companyName': 'Example'}, 'rawResponse': '{}', 'elapsedMs': 10}
            write(root / 'observations/123456789.json', row)
            self.assertEqual(report(root)['reviews'], {'needsReview': 1})
            self.assertEqual(report(root)['remaining'], 1)
            write(root / 'review.json', {'123456789': {'schemaVersion': 1, 'sourceSha256': hashlib.sha256(b'{}').hexdigest(), 'status': 'passed', 'expected': {'companyName': 'Example'}}})
            self.assertEqual(report(root)['reviews'], {'passed': 1})
            row['rawResponse'] = '{"changed":true}'
            write(root / 'replay.json', [row])
            with self.assertRaisesRegex(ValueError, 'replay source differs'):
                report(root)

    def test_provider_failure_never_counts_as_reviewed_success(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            write(root / 'manifest.json', {'ids': ['123456789']})
            write(root / 'observations/123456789.json', {'schemaVersion': 1, 'taxId': '123456789', 'outcome': 'providerError', 'record': None, 'elapsedMs': 10})
            result = report(root)
            self.assertTrue(result['complete'])
            self.assertEqual(result['reviews'], {'providerError': 1})

    def test_missing_is_not_null_and_report_exposes_field_differences(self):
        from registry_batch import differences
        self.assertEqual(differences({'x': None}, {'x': None}), [])
        self.assertEqual(differences({'x': None}, {}), [{'field': 'x', 'expected': None, 'actualPresent': False, 'actual': None}])
        self.assertTrue(differences({'x': True}, {'x': 1}))
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            write(root / 'manifest.json', {'ids': ['123456789']})
            write(root / 'observations/123456789.json', {'schemaVersion': 2, 'taxId': '123456789', 'outcome': 'found', 'record': {}, 'rawResponse': '{}', 'elapsedMs': 1})
            write(root / 'review.json', {'123456789': {'schemaVersion': 1, 'sourceSha256': hashlib.sha256(b'{}').hexdigest(), 'status': 'passed', 'expected': {'missing': None}}})
            result = report(root)
            self.assertEqual(result['reviews'], {'parserMismatch': 1})
            self.assertFalse(result['cases'][0]['differences'][0]['actualPresent'])

    def test_provider_lock_blocks_another_process_and_cooldown_survives_exit(self):
        import subprocess
        import sys
        from datetime import datetime, timedelta, timezone
        from registry_batch import provider_guard
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            with provider_guard(root) as (_, state):
                code = 'from pathlib import Path; from registry_batch import provider_guard; import sys\nwith provider_guard(Path(sys.argv[1])): pass'
                child = subprocess.run([sys.executable, '-c', code, str(root)],
                                       cwd=Path(__file__).resolve().parent,
                                       capture_output=True, text=True)
                self.assertNotEqual(child.returncode, 0)
                self.assertIn('already active', child.stderr)
                until = (datetime.now(timezone.utc) + timedelta(minutes=1)).isoformat()
                write(state, {'nextStartAt': None, 'cooldownUntil': until})
            with self.assertRaisesRegex(ValueError, 'shared provider cooldown'):
                with provider_guard(root):
                    self.fail('cooldown bypassed')

    def test_bound_replay_rejects_modified_observation(self):
        from registry_batch import sha
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            write(root / 'manifest.json', {'ids': ['123456789']})
            path = root / 'observations/123456789.json'
            row = {'schemaVersion': 2, 'taxId': '123456789', 'outcome': 'found', 'record': {}, 'rawResponse': '{}', 'elapsedMs': 1}
            write(path, row)
            replay = {**row, 'error': None, 'observationSha256': sha(path), 'sourceSha256': hashlib.sha256(b'{}').hexdigest()}
            write(root / 'replay.json', {'schemaVersion': 2, 'parserSha256': 'a'*64, 'binarySha256': 'b'*64, 'observations': [replay]})
            self.assertTrue(report(root)['replayProvenance']['bound'])
            row['elapsedMs'] = 2
            write(path, row)
            with self.assertRaisesRegex(ValueError, 'hash mismatch'):
                report(root)

    def test_consolidation_is_lossless_and_rejects_duplicate_or_changed_sources(self):
        from registry_batch import consolidate_corpus
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            case = {'taxId': '000123456', 'rawResponse': '{}', 'review': {
                'schemaVersion': 1, 'status': 'passed', 'expected': {'x': None},
                'sourceSha256': hashlib.sha256(b'{}').hexdigest(),
                'revision': 2, 'history': [{'revision': 1, 'expected': {'x': 'old'}}]}}
            write(root / 'a.json', case)
            output = root / 'bg.json'
            self.assertEqual(consolidate_corpus(root, output)['cases'], 1)
            self.assertEqual(json.loads(output.read_text()), [case])
            self.assertEqual(json.loads((root / 'a.json').read_text()), case)
            write(root / 'b.json', case)
            with self.assertRaisesRegex(ValueError, 'duplicate'):
                consolidate_corpus(root, output)
            (root / 'b.json').unlink()
            case['rawResponse'] = '{"changed":true}'
            write(root / 'a.json', case)
            with self.assertRaisesRegex(ValueError, 'source hash'):
                consolidate_corpus(root, output)
            self.assertEqual(json.loads(output.read_text())[0]['rawResponse'], '{}')

    def test_corpus_merge_accepts_only_reviewed_cases_and_never_overwrites(self):
        from registry_batch import merge_reviewed_run
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            write(root / 'manifest.json', {'ids': ['123456789', '987654321']})
            for tax_id in ['123456789', '987654321']:
                write(root / 'observations' / (tax_id + '.json'), {
                    'schemaVersion': 2, 'taxId': tax_id, 'outcome': 'found',
                    'record': {'companyName': 'Example'}, 'rawResponse': '{}', 'elapsedMs': 1})
            review = {'schemaVersion': 1, 'status': 'passed',
                      'expected': {'companyName': 'Example'},
                      'sourceSha256': hashlib.sha256(b'{}').hexdigest()}
            write(root / 'review.json', {'123456789': review})
            output = root / 'corpus.json'
            self.assertEqual(merge_reviewed_run(root, output)['added'], 1)
            self.assertEqual(merge_reviewed_run(root, output)['added'], 0)
            self.assertEqual([x['taxId'] for x in json.loads(output.read_text())], ['123456789'])
            original = output.read_bytes()
            review['revision'] = 2
            write(root / 'review.json', {'123456789': review})
            with self.assertRaisesRegex(ValueError, 'explicit re-review'):
                merge_reviewed_run(root, output)
            self.assertEqual(output.read_bytes(), original)
