import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace

import vies_batch as batch
import vies_verification as verification
from vies_evidence import load_evidence
from vies_reporting import build_report


def write_review(root, country, status='reviewed'):
    fixtures = root/'src/vies/fixtures/countries'
    (fixtures/'reviews').mkdir(exist_ok=True)
    cases = json.loads((fixtures/(country+'.json')).read_text())
    policy = {'knownNormalizationVersions': ['synthetic-v1']}
    (fixtures.parent/'review-policy.json').write_text(json.dumps(policy))
    reviews = [{'vatId': c['vatId'], 'status': status, 'note': 'Synthetic source review',
                'reviewVersion': 1, 'reviewedAt': '2026-01-01',
                'normalizationVersion': 'synthetic-v1',
                'expectedSha256': verification.expected_hash(c['expected'])} for c in cases]
    (fixtures/'reviews'/f'{country}.json').write_text(json.dumps(reviews))


class VerificationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.folder = self.root/'run'
        self.folder.mkdir()
        self.fixtures = self.root/'src/vies/fixtures/countries'
        self.fixtures.mkdir(parents=True)
        self.case = {'vatId': 'EL123456789', 'valid': True,
                     'viesOutput': {'company': 'Synthetic', 'address': 'Raw'},
                     'expected': {'address': {'city': None}}}
        self.address = {'fullAddressByVies': 'Raw', 'city': None}
        self.row = {'vatId': self.case['vatId'], 'totalMs': 1, 'attempts': [{
            'rawBody': '<synthetic/>', 'response': {'valid': True,
            'company': 'Synthetic', 'address': self.address}}]}
        self.write(self.fixtures/'gr.json', [self.case])
        write_review(self.root, 'gr')
        self.write(self.folder/'manifest.json', {'ids': [self.case['vatId']]})
        (self.folder/'observations.jsonl').write_text(json.dumps(self.row)+'\n')
        self.write(self.folder/'offline-replay.json', [{'vatId': self.case['vatId'], 'actual': self.address}])
        (self.folder/'review-journal.jsonl').write_text(json.dumps({
            'case': self.case, 'sourceSha256': verification.sha_bytes('<synthetic/>')})+'\n')
        for name in ['Cargo.toml', 'Cargo.lock', 'src/lib.rs', 'binary', 'offline.log']:
            (self.root/name).write_text('synthetic')
        files = ['Cargo.toml', 'Cargo.lock', 'src/lib.rs']
        self.binary_manifest = self.root/'binary.json'
        self.write(self.binary_manifest, {'binary': 'binary', 'sha256': verification.sha(self.root/'binary'),
                   'sourceHashes': {p: verification.sha(self.root/p) for p in files}})
        self.current = verification.snapshot(self.root, self.binary_manifest)
        self.proof = {**self.current, 'observationSha256': verification.sha(self.folder/'observations.jsonl'),
                      'replaySha256': verification.sha(self.folder/'offline-replay.json'),
                      'log': 'offline.log', 'logSha256': verification.sha(self.root/'offline.log'),
                      'tests': ['vies::address::corpus::replay_probe_journals',
                                'vies::tests::probe::records::validate_saved_typed_journals']}
        self.write(self.folder/'verification.json', self.proof)

    def write(self, path, data):
        path.write_text(json.dumps(data))

    def report(self):
        return batch.report(self.folder, root=self.root, formats=batch.vat_formats(), current=self.current)

    def test_review_and_proof_are_required(self):
        self.assertEqual(self.report()['verifiedValid'], 1)
        reviews_path = self.fixtures/'reviews/gr.json'
        original = json.loads(reviews_path.read_text())
        for change in [{}, {'expectedSha256': 'wrong'}, {'normalizationVersion': 'unknown'},
                       {'status': 'pending'}, {'reviewedAt': 'bad'}, {'reviewVersion': 0}]:
            with self.subTest(change=change):
                self.write(reviews_path, [] if not change else [{**original[0], **change}])
                result = self.report()
                self.assertEqual(result['needsReview'], 1)
                self.assertEqual(result['verifiedValid'], 0)
        self.write(reviews_path, original)
        (self.folder/'verification.json').unlink()
        self.assertEqual(self.report()['verifiedValid'], 0)

    def test_each_proof_binding_fails_closed(self):
        for field in self.proof:
            with self.subTest(field=field):
                self.write(self.folder/'verification.json', {**self.proof, field: [] if field == 'tests' else 'stale'})
                self.assertEqual(self.report()['verifiedValid'], 0)
        self.write(self.folder/'verification.json', self.proof)
        (self.folder/'review-journal.jsonl').write_text('')
        self.assertEqual(self.report()['verifiedValid'], 0)

    def test_source_or_binary_changes_reject_snapshot(self):
        for name in ['binary', 'src/lib.rs', 'src/new.rs']:
            with self.subTest(name=name):
                path = self.root/name
                original = path.read_bytes() if path.exists() else None
                path.write_text('changed')
                with self.assertRaises(ValueError):
                    verification.snapshot(self.root, self.binary_manifest)
                if original is None:
                    path.unlink()
                else:
                    path.write_bytes(original)

    def test_baseline_binding_checks_original_bytes(self):
        line = json.dumps(self.row)
        (self.root/'original.jsonl').write_text(line+'\n')
        self.write(self.folder/'manifest.json', {'ids': [self.case['vatId']], 'kind': 'derivedReplayView',
                   'bindings': {self.case['vatId']: {'sourceObservationFile': 'original.jsonl', 'line': 1,
                    'lineSha256': verification.sha_bytes(line), 'rawBodySha256': verification.sha_bytes('<synthetic/>')}}})
        self.assertEqual(self.report()['verifiedValid'], 1)
        (self.root/'original.jsonl').write_text(line+' \n')
        self.assertEqual(self.report()['verifiedValid'], 0)

    def test_distinct_counts_and_conflicts(self):
        with patch.object(batch, 'ROOT', self.root):
            result = verification.audit(self.root, [self.folder, self.folder], 'EL', self.binary_manifest, formats=batch.vat_formats())
        self.assertEqual(result['verifiedValid'], 1)
        self.assertFalse(result['accepted'])
        cases = [{'vatId': 'EL'+str(i), 'verified': True} for i in range(201)]
        rows = [{'vatId': c['vatId'], 'attempts': [{'response': {'valid': i < 200}}]} for i, c in enumerate(cases)]
        with patch.object(verification, 'build_report', return_value={'cases': cases}), patch.object(verification, 'load_evidence', return_value=SimpleNamespace(rows=rows)):
            self.assertTrue(verification.audit(self.root, [self.folder], 'EL', self.binary_manifest, formats=batch.vat_formats())['accepted'])
        duplicate = copy.deepcopy(rows[0])
        duplicate['attempts'][0]['response']['valid'] = False
        with patch.object(verification, 'build_report', side_effect=[{'cases': cases}, {'cases': [{**cases[0], 'verified': False}]}]), \
             patch.object(verification, 'load_evidence', side_effect=[SimpleNamespace(rows=rows), SimpleNamespace(rows=[duplicate])]):
            result = verification.audit(self.root, [self.folder, self.folder], 'EL', self.binary_manifest, formats=batch.vat_formats())
        self.assertEqual(result['conflictingValidity'], 1)
        self.assertFalse(result['accepted'])

    def test_negative_provider_error_and_ambiguity_stay_separate(self):
        cases = [{'vatId': 'EL1', 'verified': True},
                 {'vatId': 'EL2', 'verified': False},
                 {'vatId': 'EL3', 'verified': False}]
        rows = [{'vatId': 'EL1', 'attempts': [{'response': {'valid': False}}]},
                {'vatId': 'EL2', 'attempts': [{'error': 'timeout'}]},
                {'vatId': 'EL3', 'attempts': [{'response': {'valid': True}}]}]
        with patch.object(verification, 'build_report', return_value={'cases': cases}), \
             patch.object(verification, 'load_evidence', return_value=SimpleNamespace(rows=rows)):
            result = verification.audit(self.root, [self.folder], 'EL', self.binary_manifest, formats=batch.vat_formats())
        self.assertEqual(result['verifiedInvalid'], 1)
        self.assertEqual(result['verifiedValid'], 0)
        self.assertEqual(result['providerFailedDistinct'], 1)
        self.assertEqual(result['awaitingReviewOrReplay'], 1)
        write_review(self.root, 'gr', 'ambiguous')
        self.assertEqual(self.report()['ambiguous'], 1)
        self.assertEqual(self.report()['verifiedValid'], 0)

    def test_unbound_report_and_legacy_manifest(self):
        with patch.object(batch, 'ROOT', self.root):
            self.assertEqual(batch.report(self.folder)['verifiedValid'], 0)
        info = json.loads(self.binary_manifest.read_text())
        for name in ['Cargo.toml', 'Cargo.lock']:
            del info['sourceHashes'][name]
        self.write(self.binary_manifest, info)
        self.assertEqual(verification.snapshot(self.root, self.binary_manifest), self.current)
        (self.root/'Cargo.lock').write_text('changed')
        current = verification.snapshot(self.root, self.binary_manifest)
        self.assertFalse(verification.proof_valid(self.folder, self.root, current))

    def test_cli_reports_incomplete_coverage_without_network(self):
        args = ['vies_verification.py', '--country', 'EL', '--run-dir', str(self.folder),
                '--binary-manifest', str(self.binary_manifest)]
        with patch('sys.argv', args), patch.object(verification, 'ROOT', self.root), patch.object(verification, 'vat_formats', return_value=batch.vat_formats()), patch('builtins.print'):
            self.assertEqual(verification.main(), 1)
        (self.root/'binary').write_text('changed')
        with patch('sys.argv', args), patch.object(verification, 'ROOT', self.root), patch.object(verification, 'vat_formats', return_value=batch.vat_formats()), patch('builtins.print'):
            self.assertEqual(verification.main(), 2)

    def test_tool_imports_do_not_read_repository_files(self):
        import subprocess
        import sys
        script = """
from pathlib import Path
from unittest.mock import patch
with patch.object(Path, 'read_text', side_effect=AssertionError('import read')), \
     patch.object(Path, 'read_bytes', side_effect=AssertionError('import read')):
    import vies_batch, vies_verification, vies_recovery
"""
        subprocess.run([sys.executable, '-B', '-c', script],
                       cwd=Path(__file__).resolve().parent, check=True)

    def test_loaded_report_is_pure_and_uses_original_snapshot(self):
        evidence = load_evidence(self.root, self.folder, batch.vat_formats(), current=self.current)
        before = copy.deepcopy(evidence)
        # Replacing disk evidence cannot alter a report of the already loaded run.
        (self.folder/'observations.jsonl').write_text('changed after load')
        with patch.object(Path, 'read_bytes', side_effect=AssertionError('report read')), \
             patch.object(Path, 'read_text', side_effect=AssertionError('report read')), \
             patch.object(Path, 'write_text', side_effect=AssertionError('report write')):
            result = build_report(evidence)
        self.assertEqual(result['verifiedValid'], 1)
        self.assertEqual(evidence, before)

    def test_evidence_reads_each_file_once_and_audit_preserves_reports(self):
        reads = []
        original = Path.read_bytes

        def read(path):
            reads.append(path)
            return original(path)

        with patch.object(Path, 'read_bytes', read):
            evidence = load_evidence(self.root, self.folder, batch.vat_formats(), current=self.current)
        self.assertEqual(build_report(evidence)['verifiedValid'], 1)
        self.assertEqual(len(reads), len(set(reads)))
        report_path = self.folder/'report.json'
        report_path.write_text('existing report')
        before = {path: path.read_bytes() for path in self.root.rglob('*') if path.is_file()}
        verification.audit(self.root, [self.folder], 'EL', self.binary_manifest, formats=batch.vat_formats())
        self.assertEqual(before, {path: path.read_bytes() for path in self.root.rglob('*') if path.is_file()})

    def test_thresholds_are_independent(self):
        for valid, invalid, accepted in [(199, 9, False), (200, 0, False), (200, 1, True)]:
            self.assertEqual(verification.coverage(valid, invalid)['coverageGateMet'], accepted)
        with self.assertRaises(ValueError):
            verification.coverage(200, 1, required_valid=199)

    def test_policy_consistency_without_private_corpus(self):
        policy = json.loads((batch.ROOT/'src/vies/fixtures/review-policy.json').read_text())
        known = policy['knownNormalizationVersions']
        self.assertIn(policy['normalizationVersion'], known)
        self.assertEqual(len(known), len(set(known)))
        changes = [c['version'] for c in policy['changes']]
        self.assertEqual(set(known), set(changes))
        self.assertEqual(len(changes), len(set(changes)))


if __name__ == '__main__':
    unittest.main()
