import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import vies_batch as batch
from test_vies_verification import write_review


class BatchTests(unittest.TestCase):
    def test_selection_accepts_every_client_format_and_rejects_malformed_ids(self):
        import re
        examples = re.findall(r'"([A-Z]{2}[A-Z0-9]+)"',
            (batch.ROOT / 'src/vies/tests/identifiers.rs').read_text().split('];', 1)[0])
        self.assertEqual({v[:2] for v in examples}, set(batch.vat_formats()))
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'ids.json'
            for vat in examples:
                with self.subTest(country=vat[:2]):
                    path.write_text(json.dumps([vat]))
                    self.assertEqual(batch.selection(path, 0, 1, vat[:2]), [vat])
                    for invalid in [vat + '!', vat[:2] + '1', 123, {}, vat.lower()]:
                        path.write_text(json.dumps([invalid]))
                        with self.assertRaises(ValueError):
                            batch.selection(path, 0, 1, vat[:2])

    def test_extended_profile_is_explicit_and_frozen(self):
        import argparse
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            queue, binary = root/'ids.json', root/'binary'
            queue.write_text(json.dumps(['BG123456789']))
            binary.write_text('synthetic executable')
            args = argparse.Namespace(queue=queue, offset=0, count=1,
                run_dir=root/'run', binary=binary, docker_image=None,
                resume=False, deadline_seconds=25)
            with patch.object(batch, 'BINARY_CACHE', root/'cache'), patch.object(batch.subprocess, 'call', return_value=0) as call, patch('builtins.print'):
                self.assertEqual(batch.collect(args), 2)
            self.assertIn('live_extended_probe', call.call_args.args[0])
            manifest = json.loads((args.run_dir/'manifest.json').read_text())
            self.assertEqual(manifest['policy'], {**batch.POLICY, 'deadlineSeconds': 25})
            args.resume, args.deadline_seconds = True, 6
            with self.assertRaisesRegex(ValueError, 'policy changed'):
                batch.collect(args)

    def test_vat_prefix_aliases_read_iso_fixtures_without_rewriting_ids(self):
        for vat, country in [('EL123456789', 'gr'), ('XI012345678', 'gb'), ('XI012345678901', 'gb')]:
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                fixtures = root / 'src/vies/fixtures/countries'
                fixtures.mkdir(parents=True)
                (root / 'ids.json').write_text(json.dumps([vat]))
                address = {'fullAddressByVies': None, 'city': None}
                (root / 'observations.jsonl').write_text(json.dumps({
                    'vatId': vat, 'totalMs': 1, 'attempts': [{'response': {
                        'valid': False, 'company': None, 'address': address}}]}) + '\n')
                (fixtures / f'{country}.json').write_text(json.dumps([{
                    'vatId': vat, 'valid': False,
                    'viesOutput': {'company': None, 'address': None},
                    'expected': {'address': {'city': None}}}]))
                write_review(root, country)
                with patch.object(batch, 'ROOT', root):
                    result = batch.report(root)
                self.assertEqual(result['passed'], 1)
                self.assertEqual(result['invalidResponses'], 1)

                self.assertEqual(result['cases'][0]['vatId'], vat)
                self.assertEqual(result['verifiedInvalid'], 0)

    def test_public_checkout_reports_unreviewed_responses_without_local_corpus(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'ids.json').write_text(json.dumps(['BG123456789']))
            row = {'vatId': 'BG123456789', 'totalMs': 1, 'attempts': [{'response': {
                'valid': True, 'company': 'Example', 'address': {'fullAddressByVies': 'Example street'}
            }}]}
            (root/'observations.jsonl').write_text(json.dumps(row)+'\n')
            with patch.object(batch, 'ROOT', root):
                result = batch.report(root)
            self.assertTrue(result['complete'])
            self.assertEqual(result['statuses'], {'needsReview': 1})
            self.assertEqual(result['passed'], 0)

    def test_selection_and_duplicates(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d) / 'ids.json'
            p.write_text(json.dumps(['BG123456789', 'BG987654321']))
            self.assertEqual(batch.selection(p, 1, 1), ['BG987654321'])
            for offset, count in [(-1, 1), (0, 0), (0, 101), (2, 1)]:
                with self.assertRaises(ValueError): batch.selection(p, offset, count)
            p.write_text(json.dumps(['BG123456789'] * 2))
            with self.assertRaises(ValueError): batch.selection(p, 0, 1)

    def test_report_keeps_errors_pending_and_ambiguity_in_denominator(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            fixtures = root / 'src/vies/fixtures/countries'
            (fixtures / 'reviews').mkdir(parents=True)
            address = {'fullAddressByVies': 'raw', 'city': 'Place'}
            (fixtures / 'bg.json').write_text(json.dumps([{'vatId':'BG1','valid':True,'viesOutput':{'company':'Co','address':'raw'},'expected':{'address':{'city':'Place'}}}]))
            write_review(root, 'bg', 'ambiguous')
            (root / 'ids.json').write_text(json.dumps(['BG1','BG2','BG3']))
            rows = [{'vatId':'BG1','totalMs':20,'attempts':[{'response':{'valid':True,'company':'Co','address':address}}]}, {'vatId':'BG2','totalMs':6000,'attempts':[{'error':'timeout'}]}]
            (root / 'batch-00.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in rows))
            with patch.object(batch, 'ROOT', root): result = batch.report(root)
            self.assertEqual(result['statuses'], {'ambiguous':1,'providerError':1})
            self.assertEqual(result['pending'], 1)
            self.assertEqual(result['overallPassPercentage'], 0)
            self.assertFalse(result['complete'])
            with (root / 'batch-00.jsonl').open('a') as f: f.write(json.dumps(rows[0])+'\n')
            with self.assertRaises(ValueError): batch.load_rows(root)

    def test_partial_tail_is_not_silently_lost(self):
        with tempfile.TemporaryDirectory() as d:
            p = Path(d)
            (p / 'observations.jsonl').write_text('{')
            with self.assertRaises(ValueError): batch.load_rows(p)


class ResumeIntegration(unittest.TestCase):
    def test_docker_batches_reuse_and_retain_the_same_executable(self):
        import argparse
        with tempfile.TemporaryDirectory() as d:
            root = Path(d).resolve()
            (root/'src/vies/fixtures').mkdir(parents=True)
            (root/'src/vies/fixtures/review-policy.json').write_text('{}')
            for name in ['Cargo.toml', 'Cargo.lock']:
                (root/name).write_text('synthetic')
            queue, source = root/'queue.json', root/'source'
            queue.write_text(json.dumps(['BG123456789']))
            source.write_bytes(b'synthetic executable')
            args = argparse.Namespace(queue=queue, offset=0, count=1,
                run_dir=root/'first', binary=source, docker_image='synthetic-image', resume=False)
            with patch.object(batch, 'ROOT', root), patch.object(batch, 'BINARY_CACHE', root/'target/binaries'), patch.object(batch, 'report', side_effect=[{'complete': False}, {'complete': True}]*2), patch.object(batch.subprocess, 'check_output', return_value='head'), patch.object(batch.subprocess, 'call', return_value=0) as call, patch('builtins.print'):
                self.assertEqual(batch.collect(args), 0)
                args.run_dir = root/'second'
                self.assertEqual(batch.collect(args), 0)
            commands = [c.args[0] for c in call.call_args_list]
            self.assertEqual(commands[0][commands[0].index('--entrypoint')+1], commands[1][commands[1].index('--entrypoint')+1])
            manifests = [json.loads((root/name/'manifest.json').read_text()) for name in ['first', 'second']]
            self.assertEqual(manifests[0]['binaryPath'], manifests[1]['binaryPath'])
            self.assertTrue((root/manifests[0]['binaryPath']).is_file())
            self.assertFalse(list(root.glob('*/probe-bin')))

    def test_frozen_binary_is_shared_without_linking_mutable_source(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            source = root/'source'
            source.write_bytes(b'original executable')
            with patch.object(batch, 'BINARY_CACHE', root/'cache'):
                first = batch.freeze_binary(source)
                inode = first.stat().st_ino
                self.assertEqual(batch.freeze_binary(source).stat().st_ino, inode)
                source.write_bytes(b'new executable')
                second = batch.freeze_binary(source)
                self.assertNotEqual(first, second)
                self.assertEqual(first.read_bytes(), b'original executable')
                first.chmod(0o700)
                first.write_bytes(b'tampered')
                source.write_bytes(b'original executable')
                with self.assertRaisesRegex(ValueError, 'cached frozen binary changed'):
                    batch.freeze_binary(source)

    def test_resume_uses_frozen_binary_and_does_not_repeat_errors(self):
        import argparse
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            queue = root / 'queue.json'
            queue.write_text(json.dumps(['BG123456789', 'BG987654321']))
            binary = root / 'fake-probe'
            binary.write_text('''#!/usr/bin/env python3
import os,json,pathlib,sys
p=pathlib.Path(os.environ['VIES_PROBE_REPORT'])
rows=[json.loads(l) for l in p.read_text().splitlines()] if p.exists() else []
ids=os.environ['VIES_PROBE_IDS'].split(',')
with p.open('a') as f:f.write(json.dumps({'vatId':ids[len(rows)],'totalMs':1,'attempts':[{'error':'test-only'}]})+'\\n')
sys.exit(3 if not rows else 0)
''')
            binary.chmod(0o700)
            args = argparse.Namespace(queue=queue, offset=0, count=2, run_dir=root/'run', binary=binary, docker_image=None, resume=False)
            with patch.object(batch, 'BINARY_CACHE', root/'cache'):
                self.assertEqual(batch.collect(args), 3)
            manifest = json.loads((args.run_dir/'manifest.json').read_text())
            frozen = (batch.ROOT / manifest['binaryPath']).resolve()
            frozen_bytes = frozen.read_bytes()
            binary.write_text('changed source binary; frozen copy must be used')
            args.resume = True
            args.deadline_seconds = 25
            with self.assertRaisesRegex(ValueError, 'policy changed'):
                batch.collect(args)
            args.deadline_seconds = 6
            self.assertEqual(batch.collect(args), 0)
            self.assertEqual(len(batch.load_rows(args.run_dir)), 2)
            self.assertFalse((args.run_dir/'probe-bin').exists())
            self.assertEqual(frozen.read_bytes(), frozen_bytes)
            self.assertEqual(batch.collect(args), 0)  # Completed resume is a no-op.

    def test_legacy_incomplete_manifest_keeps_its_frozen_copy(self):
        import argparse
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            queue, source = root/'queue.json', root/'source'
            queue.write_text(json.dumps(['BG123456789']))
            source.write_bytes(b'legacy executable')
            args = argparse.Namespace(queue=queue, offset=0, count=1,
                run_dir=root/'run', binary=source, docker_image=None, resume=False)
            with patch.object(batch, 'BINARY_CACHE', root/'cache'), patch.object(batch.subprocess, 'call', return_value=0), patch('builtins.print'):
                self.assertEqual(batch.collect(args), 2)
                path = args.run_dir/'manifest.json'
                manifest = json.loads(path.read_text())
                del manifest['binaryPath']
                path.write_text(json.dumps(manifest))
                (args.run_dir/'probe-bin').write_bytes(source.read_bytes())
                source.write_bytes(b'changed input')
                args.resume = True
                with patch.object(batch.subprocess, 'call', return_value=0) as call:
                    self.assertEqual(batch.collect(args), 2)
                self.assertEqual(call.call_args.args[0][0], str(args.run_dir.resolve()/'probe-bin'))

class ReportContractTests(unittest.TestCase):
    def test_shared_rust_python_status_vectors(self):
        from vies_reporting import Facts, classify
        for case in json.loads((batch.ROOT/'src/vies/fixtures/report-contract.json').read_text()):
            self.assertEqual(classify(Facts(**case['facts'])).value, case['status'])

    def test_every_expectation_has_versioned_review_and_matching_hash(self):
        import hashlib
        from datetime import date
        root = batch.ROOT/'src/vies/fixtures'
        if not list((root/'countries').glob('*.json')):
            self.skipTest('local-only reviewed VIES corpus is not installed')
        policy = json.loads((root/'review-policy.json').read_text())
        for path in sorted((root/'countries').glob('*.json')):
            reviews = json.loads((root/'countries/reviews'/path.name).read_text())
            lookup = {r['vatId']:r for r in reviews}
            self.assertEqual(len(lookup), len(reviews))
            cases = json.loads(path.read_text())
            self.assertEqual(set(lookup), {c['vatId'] for c in cases})
            for case in cases:
                review = lookup[case['vatId']]
                self.assertIn(review['normalizationVersion'], policy.get('knownNormalizationVersions', [policy['normalizationVersion']]))
                self.assertGreaterEqual(review['reviewVersion'], 1)
                date.fromisoformat(review['reviewedAt'])
                self.assertTrue(review['note'])
                expected_hash = hashlib.sha256(json.dumps(case['expected'], sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()).hexdigest()
                self.assertEqual(review['expectedSha256'], expected_hash, case['vatId'])

if __name__ == '__main__': unittest.main()

class RomanianBatchTests(unittest.TestCase):
    def test_romanian_selection_is_explicit_and_length_bounded(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'queue.json'
            p.write_text(json.dumps(['RO001234']))
            self.assertEqual(batch.selection(p,0,1,'RO'), ['RO001234'])
            with self.assertRaises(ValueError): batch.selection(p,0,1)
            p.write_text(json.dumps(['RO12345678901']))
            with self.assertRaises(ValueError): batch.selection(p,0,1,'RO')

    def test_romanian_report_does_not_consume_bulgarian_expectations(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d)
            (root/'ids.json').write_text(json.dumps(['RO1234']))
            row={'vatId':'RO1234','totalMs':1,'attempts':[{'response':{'valid':False,'company':None,'address':{'fullAddressByVies':None}}}]}
            (root/'observations.jsonl').write_text(json.dumps(row)+'\n')
            with patch.object(batch,'ROOT',root): result=batch.report(root)
            self.assertEqual(result['needsReview'],1)
            self.assertEqual(result['invalidResponses'],1)
