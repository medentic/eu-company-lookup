import subprocess
import tempfile
import unittest
from pathlib import Path

from check_public_source import PUBLIC_FIXTURES, forbidden_files, history_paths


class PublicSourceTests(unittest.TestCase):
    def test_code_and_approved_synthetic_fixtures_are_allowed(self):
        self.assertEqual(forbidden_files(PUBLIC_FIXTURES | {
            'src/bg/tests.rs', 'README.md', 'tools/vies_batch.py', 'Cargo.lock',
        }), [])

    def test_data_and_renamed_json_cannot_bypass_the_boundary(self):
        for path in ('src/bg/fixtures/commercial/case.json',
                     'src/vies/fixtures/countries/bg.json',
                     'src/vies/fixtures/ids/bg.source.json',
                     'docs/observations.json', 'records.jsonl', 'target/report.txt'):
            with self.subTest(path=path):
                self.assertEqual(forbidden_files({path}), [path])

    def test_removing_a_file_does_not_remove_it_from_history(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                return subprocess.check_output(['git', *args], cwd=root, stderr=subprocess.DEVNULL)
            git('init', '-q')
            git('config', 'user.email', 'test@example.invalid')
            git('config', 'user.name', 'Fixture test')
            path = root/'records.json'
            path.write_text('{}')
            git('add', 'records.json')
            git('commit', '-qm', 'Add synthetic test record')
            git('rm', '-q', 'records.json')
            git('commit', '-qm', 'Remove record')
            self.assertEqual(forbidden_files(history_paths(root, 'HEAD')), ['records.json'])


if __name__ == '__main__':
    unittest.main()
