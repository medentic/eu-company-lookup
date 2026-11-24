"""Synthetic orchestration checks; never load real corpora or execute containers."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import replay_fixtures as R


class AllReplayTests(unittest.TestCase):
    def test_runs_all_provider_workflows(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp).resolve();binary=root/'test-binary';binary.touch()
            with patch.object(R,'ROOT',root),patch.dict('os.environ',{'CI':'false'}),patch('sys.argv',['replay_fixtures.py','--docker-image','local-image','--binary',str(binary)]),patch.object(R.subprocess,'run') as run:
                R.main()
            self.assertEqual([Path(c.args[0][1]).name for c in run.call_args_list],['registry_fixtures.py','vies_fixtures.py','bg_fixtures.py','bulstat_fixtures.py'])
            self.assertTrue(all(c.kwargs['check'] for c in run.call_args_list))

    def test_refuses_ci_before_loading_data(self):
        with patch.dict('os.environ',{'CI':'true'}),patch('sys.argv',['replay_fixtures.py','--docker-image','local-image','--binary','missing']),patch.object(R.subprocess,'run') as run:
            with self.assertRaises(SystemExit):R.main()
            run.assert_not_called()


if __name__=='__main__':unittest.main()
