import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import vies_batch as batch


class BatchTests(unittest.TestCase):
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
            (fixtures / 'reviews/bg.json').write_text(json.dumps([{'vatId':'BG1','status':'ambiguous','note':'uncertain'}]))
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
            self.assertEqual(batch.collect(args), 3)
            binary.write_text('changed source binary; frozen copy must be used')
            args.resume = True
            self.assertEqual(batch.collect(args), 0)
            self.assertEqual(len(batch.load_rows(args.run_dir)), 2)
            self.assertFalse((args.run_dir/'probe-bin').exists())
            self.assertEqual(batch.collect(args), 0)  # Completed resume is a no-op.

class ReportContractTests(unittest.TestCase):
    def test_shared_rust_python_status_vectors(self):
        from vies_reporting import Facts, classify
        for case in json.loads((batch.ROOT/'src/vies/fixtures/report-contract.json').read_text()):
            self.assertEqual(classify(Facts(**case['facts'])).value, case['status'])

    def test_every_expectation_has_versioned_review_and_matching_hash(self):
        import hashlib
        from datetime import date
        root = batch.ROOT/'src/vies/fixtures'
        if not (root/'countries/bg.json').exists():
            self.skipTest('local-only reviewed VIES corpus is not installed')
        policy = json.loads((root/'review-policy.json').read_text())
        reviews = json.loads((root/'countries/reviews/bg.json').read_text())
        lookup = {r['vatId']:r for r in reviews}
        self.assertEqual(len(lookup), len(reviews))
        cases = json.loads((root/'countries/bg.json').read_text())
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
