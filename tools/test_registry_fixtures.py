"""Synthetic workflow regressions; never load collected fixtures or call providers."""
import copy
import json
from pathlib import Path
import tempfile
import types
import unittest
from unittest.mock import patch

import registry_fixture_data as D
import registry_fixtures as F


def record(identifier='001', status='active', date='2026-11-01T00:00:00Z'):
    body = D.canonical({'organisasjonsnummer': identifier, 'navn': 'Synthetic company'})
    row, review, provenance = D.prepare('NO', {
        'id': identifier, 'checkedAt': date,
        'capture': {'httpStatus': 200, 'body': body},
    }, {'fields': {'/taxId': identifier, '/status': status}})
    return dict(observation=row, review=review, provenance=provenance,
                reviewer='Test reviewer', rationale='Synthetic source review',
                category='resolved', expectedValidation='verifiedPositive')


class RegistryFixtureTests(unittest.TestCase):
    def test_append_closure_preserves_active_replay_but_reduces_active_count(self):
        corpus = D.empty('NO')
        for n in range(200):
            D.add(corpus, record(str(n)))
        self.assertEqual(F.coverage(corpus)['activeShortfall'], 0)
        before = copy.deepcopy(corpus['records'])
        closed = record('0', 'closed', '2026-11-02T00:00:00Z')
        D.add(corpus, closed)
        D.validate(corpus)
        self.assertTrue(all(corpus['records'][k] == v for k, v in before.items()))
        self.assertEqual(len(corpus['replay']), 201)
        self.assertEqual(F.coverage(corpus)['activeShortfall'], 1)
        D.add(corpus, record('0'))  # An older imported observation cannot become latest.
        self.assertEqual(corpus['records'][corpus['current']['0']], closed)

    def test_review_revision_retains_old_review_without_duplicate_oracle(self):
        corpus = D.empty('NO')
        original = record()
        old = D.add(corpus, original)
        changed = copy.deepcopy(original)
        changed['review']['fields']['/status'] = 'closed'
        new = D.add(corpus, changed)
        D.validate(corpus)
        self.assertEqual(corpus['history']['001'], [old, new])
        self.assertEqual(corpus['replay'], [new])
        self.assertEqual(corpus['records'][old], original)
        corpus['records'][old]['rationale'] = 'tampered'
        with self.assertRaisesRegex(AssertionError, 'immutable'):
            D.validate(corpus)

    def test_failure_and_unresolved_never_count_as_active(self):
        corpus = D.empty('NO')
        for n, (category, expected) in enumerate([
            ('accessFailure', 'failed'), ('sourceFailure', 'failed'),
            ('unresolved', 'unresolved'), ('negative', 'negativeObservation'),
        ]):
            r = record(str(n))
            r.update(category=category, expectedValidation=expected)
            D.add(corpus, r)
        D.validate(corpus)
        self.assertEqual(F.coverage(corpus)['activeShortfall'], 200)

    def test_restaging_rejects_duplicate_or_unselected_observations(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp).resolve();fixtures=root/'tests/fixtures/registry'
            c=D.empty('NO');D.add(c,record());D.write(fixtures/'NO/corpus.json',c)
            directory=root/'target/run';directory.mkdir(parents=True)
            (directory/'manifest.json').write_text(json.dumps({'ids':['001']}))
            args=types.SimpleNamespace(country='NO',run_dir=str(directory),docker_image=None)
            for rows in ([{'id':'001'},{'id':'001'}],[{'id':'002'}]):
                (directory/'observations.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in rows))
                with patch.object(F,'ROOT',root),patch.object(F,'FIXTURES',fixtures),patch.object(F,'audit') as audit:
                    with self.assertRaises(ValueError):F.stage_captured(args)
                    audit.assert_not_called()

    def test_shared_history_selection_replaces_only_same_observation_review(self):
        records={'old':{'observation':'day-one','expected':'wrong'},'new':{'observation':'day-one','expected':'correct'},'later':{'observation':'day-two','expected':'new facts'}}
        c={'records':records,'history':{'001':['old','new','later']}}
        selected=D.applicable_history(c,lambda r:r['observation'])
        self.assertEqual(selected,{'001':['new','later']})
        self.assertEqual(len(c['records']),3)

    def test_partial_access_is_separate_from_unknown_lifecycle(self):
        c=D.empty('NO');r=record(status=None)
        r.update(category='accessFailure',expectedValidation='unresolved')
        D.add(c,r)
        with self.assertRaises(AssertionError):D.validate(c)
        c=D.empty('NO');r['observation']['collectionFailure']={'reason':'connection'}
        D.add(c,r);D.validate(c)
        self.assertEqual(F.coverage(c)['latestReviewed'],{'accessFailure':1})

    def test_undated_history_does_not_hide_known_freshness_range(self):
        corpus=D.empty('NO')
        dated=record('1');undated=record('2');undated['observation'].pop('checkedAt')
        D.add(corpus,dated);D.add(corpus,undated)
        report=F.coverage(corpus)
        self.assertEqual(report['undatedObservations'],1)
        self.assertEqual(report['reviewedDateRange'],['2026-11-01T00:00:00Z']*2)
        self.assertEqual(report['observationsByRecordedDate'],{'2026-11-01':1})

    def test_preparation_retains_required_fields_and_original_hash(self):
        raw = {'id': '001', 'capture': {'responses': [{
            'httpStatus': 200,
            'body': D.canonical({'id': '1', 'fnr': '001', 'legalForm': {
                'abbr': 'TEST', 'name': 'Synthetic'}, 'email': 'omit@example.invalid'}),
            'authorization': 'secret',
        }]}}
        row, _, provenance = D.prepare('AT', raw, {'fields': {}})
        self.assertEqual(provenance['originalObservationSha256'], D.digest(raw))
        self.assertNotIn('secret', D.canonical(row))
        self.assertNotIn('email', D.canonical(row))
        self.assertEqual(json.loads(row['capture']['responses'][0]['body'])['legalForm']['abbr'], 'TEST')
        self.assertEqual(D.payload('CZ', 'ros', {'icoId': '001', 'zaznamy': []})['icoId'], '001')

    def test_refresh_stages_only_then_accept_updates_replay_with_history(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            fixtures = root / 'tests/fixtures/registry'
            path = fixtures / 'NO/corpus.json'
            corpus = D.empty('NO')
            D.add(corpus, record())
            D.write(path, corpus)
            before = path.read_bytes()
            run = root / 'target/run'
            args = types.SimpleNamespace(country='NO', ids=['001'], limit=None,
                                         run_dir=str(run), docker_image=None,
                                         reviewer='Test reviewer', review_file=str(run / 'reviewed.json'))
            new = record(status='closed', date='2026-11-02T00:00:00Z')
            def collect(*a, **kw):
                self.assertTrue(kw['online'])
                (run / 'observations.jsonl').write_text(D.canonical(new['observation']) + '\n')
                return 1  # Partial/failed runs must retain their observations.
            result = {'001': {'validationStatus': 'verifiedPositive',
                              'reviewBound': True, 'differences': [{'field': '/status'}]}}
            with patch.object(F, 'ROOT', root), patch.object(F, 'FIXTURES', fixtures):
                with patch.object(F, 'cargo', side_effect=collect), patch.object(F, 'audit', return_value=result):
                    self.assertEqual(F.refresh(args), 2)
                self.assertEqual(path.read_bytes(), before)
                template = json.loads((run / 'review-template.json').read_text())
                Path(args.review_file).write_text(json.dumps(template))
                with self.assertRaisesRegex(ValueError, 'rationale'):
                    F.accept(args)
                template['001']['fields']['/status'] = 'closed'
                template['001']['rationale'] = 'Reviewed synthetic closure'
                Path(args.review_file).write_text(json.dumps(template))
                with patch.object(F, 'audit', return_value=result):
                    with self.assertRaisesRegex(ValueError, 'reviewed fields'):
                        F.accept(args)
                self.assertEqual(path.read_bytes(), before)
                result['001']['differences'] = []
                with patch.object(F, 'audit', return_value=result):
                    self.assertEqual(F.accept(args), 0)
                updated = D.read(path)
                self.assertEqual(len(updated['records']), 2)
                self.assertEqual(len(updated['replay']), 2)
                self.assertEqual(F.coverage(updated)['latestReviewed'], {'closed': 1})

    def test_ci_rejects_real_workflow(self):
        with patch.dict('os.environ', {'CI': 'true'}), patch('sys.argv', ['registry_fixtures.py', 'replay']):
            with self.assertRaises(SystemExit) as result:
                F.main()
            self.assertEqual(result.exception.code, 2)

    def test_docker_replay_has_no_network_and_exact_test(self):
        args = types.SimpleNamespace(docker_image='existing-image')
        with patch.object(F.subprocess, 'run', return_value=types.SimpleNamespace(returncode=0, stdout='')) as run:
            self.assertEqual(F.cargo(args, 'no::probe::audit_retained_registry_responses', {}), 0)
            command = run.call_args.args[0]
            self.assertIn('--network', command)
            self.assertIn('none', command)
            self.assertIn('--pull=never', command)
            self.assertIn('--exact', command)


if __name__ == '__main__':
    unittest.main()
