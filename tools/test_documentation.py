"""Regression tests for documentation and publication boundaries."""
from pathlib import Path
import tempfile
import unittest
from check_docs import check
from check_distribution import REQUIRED, validate_files


class DocumentationTests(unittest.TestCase):
    def test_local_links_and_fragments_are_checked_without_fetching_external_urls(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'README.md').write_text(
                '# Start\n[guide](guide.md#usage)\n[remote](https://example.invalid/)\n'
                '```text\n[not a link](missing.md)\n```\n')
            (root / 'guide.md').write_text('# Guide\n## Usage\n')
            files = {'README.md', 'guide.md'}
            self.assertEqual(check(root, files), [])
            (root / 'guide.md').write_text('# Guide\n## Renamed\n')
            self.assertIn('missing heading', check(root, files)[0])

    def test_package_cannot_link_to_repository_only_docs(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'README.md').write_text('[guide](CONTRIBUTING.md)\n')
            (root / 'CONTRIBUTING.md').write_text('# Contributors\n')
            self.assertEqual(check(root, {'README.md'}), [])
            self.assertIn('missing link target', check(root, {'README.md'}, packaged=True)[0])

    def test_repository_urls_resolve_locally_but_can_reference_excluded_guides(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'README.md').write_text(
                '[guide](https://github.com/medentic/eu-company-lookup/blob/main/CONTRIBUTING.md#usage)\n')
            (root / 'CONTRIBUTING.md').write_text('# Contributors\n## Usage\n')
            self.assertEqual(check(root, {'README.md'}, packaged=True), [])
            (root / 'CONTRIBUTING.md').unlink()
            self.assertIn('missing link target', check(root, {'README.md'}, packaged=True)[0])

    def test_rejects_local_paths_and_missing_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'README.md').write_text('Use /home/example/project.\n[absent](missing.md)\n')
            errors = check(root, {'README.md'})
            self.assertEqual(len(errors), 2)
            self.assertTrue(any('machine-specific' in error for error in errors))

    def test_publication_excludes_maintainer_docs_and_source_evidence(self):
        self.assertEqual(validate_files(REQUIRED | {'src/new_module.rs'}), ([], []))
        for unwanted in ['AGENTS.md', 'CONTRIBUTING.md', 'docs/README.md',
                         'docs/development/testing.md', 'docs/development/collectors.md',
                         'docs/maintainers/releasing.md',
                         'src/vies/fixtures/ids/bg.source.json', 'src/bg/fixtures/commercial/case.json',
                         'src/vies/fixtures/ids/bg.json', 'src/debug.log', 'target/run.jsonl']:
            with self.subTest(unwanted=unwanted):
                self.assertEqual(validate_files(REQUIRED | {unwanted})[1], [unwanted])
        self.assertEqual(validate_files(REQUIRED - {'LICENSE'})[0], ['LICENSE'])


if __name__ == '__main__':
    unittest.main()
