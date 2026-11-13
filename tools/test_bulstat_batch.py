import json
from pathlib import Path
import tempfile
import unittest
from bulstat_batch import selection, report


class BulstatBatchTests(unittest.TestCase):
    def test_selection_preserves_full_ids_and_requires_provenance(self):
        case = {'taxId': '0000000010001', 'name': 'Synthetic', 'sourceUrl': 'https://example.test/source'}
        self.assertEqual(selection({'cases': [case]}), ['0000000010001'])
        for cases in [[], [case] * 11, [case, case], [{**case, 'sourceUrl': ''}],
                      [{**case, 'taxId': '1234567890'}], [{**case, 'taxId': 'BG123456789'}]]:
            with self.assertRaises(ValueError):
                selection({'cases': cases})

    def test_complete_is_not_bulstat_success(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'observations').mkdir()
            (root / 'manifest.json').write_text(json.dumps({'ids': ['000000001']}))
            self.assertFalse(report(root)['complete'])
            row = {'schemaVersion': 1, 'taxId': '000000001', 'outcome': 'commercialFound', 'elapsedMs': 1, 'error': None}
            path = root / 'observations/000000001.json'
            path.write_text(json.dumps(row))
            self.assertTrue(report(root)['complete'])
            self.assertFalse(report(root)['allBulstatFound'])
            row['outcome'] = 'bulstatFound'
            path.write_text(json.dumps(row))
            self.assertTrue(report(root)['allBulstatFound'])
            row['taxId'] = '000000002'
            path.write_text(json.dumps(row))
            with self.assertRaises(ValueError):
                report(root)

    def test_reviews_and_replay_are_bound_to_original_observations(self):
        from registry_batch import sha
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'observations').mkdir()
            (root / 'manifest.json').write_text(json.dumps({'ids': ['000000001']}))
            row = {'schemaVersion': 1, 'taxId': '000000001', 'outcome': 'bulstatFound',
                   'elapsedMs': 1, 'error': None, 'record': {'companyName': 'Synthetic'}}
            path = root / 'observations/000000001.json'
            path.write_text(json.dumps(row))
            review = {'schemaVersion': 1, 'observationSha256': sha(path),
                      'scope': ['companyName'], 'expected': {'companyName': 'Different'}}
            (root / 'review.json').write_text(json.dumps({'000000001': review}))
            self.assertEqual(report(root)['cases'][0]['reviewStatus'], 'mismatch')
            replay = {'taxId': '000000001', 'observationSha256': sha(path),
                      'record': row['record'], 'error': None}
            (root / 'replay.json').write_text(json.dumps({'rows': [replay]}))
            self.assertTrue(report(root)['cases'][0]['replayMatchesLive'])
            row['record']['companyName'] = 'Changed'
            path.write_text(json.dumps(row))
            with self.assertRaisesRegex(ValueError, 'hash mismatch'):
                report(root)

    def test_explicit_review_can_approve_changed_replay_but_not_replay_failure(self):
        from registry_batch import sha
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'observations').mkdir()
            (root / 'manifest.json').write_text(json.dumps({'ids': ['000000001']}))
            observation = root / 'observations/000000001.json'
            observation.write_text(json.dumps({'schemaVersion': 1, 'taxId': '000000001',
                'outcome': 'bulstatFound', 'elapsedMs': 1, 'error': None,
                'record': {'status': 'развиващ дейност'}}))
            replay = {'taxId': '000000001', 'observationSha256': sha(observation),
                      'record': {'status': 'active'}, 'error': None}
            (root / 'replay.json').write_text(json.dumps({'rows': [replay]}))
            self.assertFalse(report(root)['checksPassed'])
            (root / 'review.json').write_text(json.dumps({'000000001': {
                'schemaVersion': 1, 'observationSha256': sha(observation),
                'scope': ['status'], 'expected': {'status': 'active'}}}))
            self.assertTrue(report(root)['checksPassed'])
            replay['error'] = 'parser failure'
            (root / 'replay.json').write_text(json.dumps({'rows': [replay]}))
            self.assertFalse(report(root)['checksPassed'])
