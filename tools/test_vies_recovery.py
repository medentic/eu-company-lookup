import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import vies_recovery as recovery


class RecoveryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.plan = self.base/'plan.json'
        self.binary = self.base/'test-bin'
        self.binary.write_text('synthetic test executable')
        self.queue('BG', ['BG123456789', 'BG123456788', 'BG123456787', 'BG123456786'])
        self.queue('CZ', ['CZ12345678'])
        self.plan.write_text(json.dumps({'countries': [
            {'country': c, 'queue': str(self.base/(c+'.json')), 'count': n}
            for c, n in [('BG', 4), ('CZ', 1)]]}))
        self.args = SimpleNamespace(plan=self.plan, run_dir=self.base/'run', execute=True,
            max_requests=20, binary=self.binary, docker_image=None, deadline_seconds=25, skip=[])
        self.calls = []
        self.failed = set()
        self.retry = False

    def queue(self, country, ids):
        (self.base/(country+'.json')).write_text(json.dumps(ids))

    def collect(self, args):
        self.calls.append((args.country, args.offset))
        identifier = json.loads(args.queue.read_text())[args.offset]
        args.run_dir.mkdir(parents=True, exist_ok=True)
        (args.run_dir/'manifest.json').write_text(json.dumps({'ids': [identifier]}))
        outcome = {'error': {'code': 'memberStateRateLimit'}} if args.country in self.failed else {'response': {'valid': True}}
        row = {'vatId': identifier, 'checkedAt': recovery.now(),
               'attempts': [{'retryAfter': '120' if self.retry else None, **outcome}]}
        (args.run_dir/'observations.jsonl').write_text(json.dumps(row)+'\n')
        return 0

    def report(self, folder):
        replay = (folder/'offline-replay.json').exists()
        return {'cases': [{'status': 'passed', 'parsedFrom': 'offlineReplay' if replay else 'liveObservation'}]}

    def run_mocked(self, states=None):
        states = states or {'BG': 'Available', 'CZ': 'Available'}
        with patch.object(recovery.batch, 'ROOT', self.base), \
             patch.object(recovery.batch, 'collect', side_effect=self.collect), \
             patch.object(recovery.batch, 'report', side_effect=self.report), \
             patch.object(recovery, 'availability', return_value=({'vow': {'available': True}}, states)), \
             patch.object(recovery.time, 'sleep'):
            return recovery.run(self.args)

    def test_bounded_resume_and_observed_is_not_verified(self):
        self.args.max_requests = 2
        first = self.run_mocked()
        self.assertEqual(first['BG']['attempted'], 2)
        self.assertEqual(first['BG']['validObserved'], 2)
        self.assertEqual(first['BG']['verifiedValid'], 0)
        self.run_mocked()
        self.assertEqual(self.calls, [('BG', 0), ('BG', 1), ('BG', 2), ('BG', 3)])
        (self.args.run_dir/'BG/0000/offline-replay.json').write_text('[]')
        self.args.execute = False
        report = self.run_mocked()
        self.assertEqual(report['BG']['verifiedValid'], 1)

    def test_three_failures_defer_country_and_continue_next(self):
        self.failed.add('BG')
        result = self.run_mocked()
        self.assertEqual(self.calls, [('BG', 0), ('BG', 1), ('BG', 2), ('CZ', 0)])
        self.assertEqual(result['BG']['providerErrors'], 3)
        self.assertEqual(result['BG']['pending'], 1)
        self.assertEqual(result['BG']['verifiedValid'], 0)

    def test_unavailable_and_explicit_skip_do_not_collect(self):
        self.run_mocked({'BG': 'Unavailable', 'CZ': 'Available'})
        self.assertEqual(self.calls, [('CZ', 0)])
        self.args.skip = ['BG']
        self.run_mocked()
        self.assertEqual(self.calls, [('CZ', 0)])

    def test_changed_queue_rejected_before_new_requests(self):
        self.args.max_requests = 1
        self.run_mocked()
        self.queue('BG', ['BG223456789', 'BG123456788', 'BG123456787', 'BG123456786'])
        with self.assertRaisesRegex(ValueError, 'selection, binary or policy changed'):
            self.run_mocked()
        self.assertEqual(len(self.calls), 1)

    def test_binary_replaced_during_pass_stops_before_next_lookup(self):
        original_collect = self.collect

        def replace_after_first(args):
            code = original_collect(args)
            self.binary.write_text('different executable')
            return code

        self.collect = replace_after_first
        with self.assertRaisesRegex(ValueError, 'binary changed during recovery'):
            self.run_mocked()
        self.assertEqual(len(self.calls), 1)

    def test_retry_after_stops_and_remains_a_restart_boundary(self):
        self.retry = True
        with self.assertRaisesRegex(RuntimeError, 'Retry-After'):
            self.run_mocked()
        with self.assertRaisesRegex(ValueError, 'Retry-After'):
            self.run_mocked()
        self.assertEqual(len(self.calls), 1)

    def test_default_offline_report_does_not_contact_sources_or_create_run(self):
        self.args.execute = False
        with patch.object(recovery, 'availability', side_effect=AssertionError('network')), \
             patch.object(recovery.batch, 'collect', side_effect=AssertionError('collection')):
            result = recovery.run(self.args)
        self.assertEqual(result['BG']['pending'], 4)
        self.assertFalse(self.args.run_dir.exists())

    def test_restart_keeps_failure_cooldown(self):
        self.failed.add('BG')
        self.args.max_requests = 1
        self.run_mocked()
        delay = recovery.next_delay(self.args.run_dir)
        self.assertGreater(delay, 58)
        self.assertLessEqual(delay, 60)

    def test_nanosecond_journal_timestamp_keeps_cooldown_without_rewriting_source(self):
        self.failed.add('BG')
        self.args.max_requests = 1
        self.run_mocked()
        path = self.args.run_dir/'BG/0000/observations.jsonl'
        row = json.loads(path.read_text())
        row['checkedAt'] = '2026-11-16T19:16:11.644030878Z'
        original = json.dumps(row)+'\n'
        path.write_text(original)
        with patch.object(recovery.time, 'time', return_value=1794856571.644030):
            self.assertAlmostEqual(recovery.next_delay(self.args.run_dir), 60, places=4)
        self.assertEqual(path.read_text(), original)


if __name__ == '__main__':
    unittest.main()
