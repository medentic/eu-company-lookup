# Repository tooling and local evidence

These Python tools support maintenance; none are part of the published Rust API.
Read [collector workflows](../docs/development/collectors.md) for exact commands
and [testing/evidence](../docs/development/testing.md) for review semantics.

| Tool | Responsibility |
| --- | --- |
| `check_docs.py` | Validate repository Markdown targets and headings without fetching external links. |
| `check_distribution.py` | Inspect Cargo's current package file list, allowed contents and consumer links. |
| `check_public_source.py` | Detect collected records/queues/provenance in source and Git history. |
| `vies_batch.py`, `vies_reporting.py` | VIES collection orchestration and reports. |
| `registry_batch.py` | Commercial Register collection/replay workflow. |
| `bulstat_batch.py` | BULSTAT-specific evidence workflow. |
| `ro_batch.py` | ANAF collection/replay workflow. |
| `import_bg_vat_ids.py` | Local queue import; does not establish registration or verification. |
| `test_*.py` | Offline tool regression tests. |

Use `--help` and the workflow guide before running a collector. A collector may
launch Rust probes and contact real services; its presence is not authorization
to run it. Python unit tests run with
`python3 -m unittest discover -s tools -p 'test_*.py'`.

Keep source observations separate from reviewed expectations and replay outputs.
Preserve append-only audit journals, provenance and review revisions. Local-only
queues/corpora are excluded from Git and the crate; an ignored path is not permission
to delete its contents. Classify a file before cleanup, especially under `target/`.
The public source-history scan is required before a push, even if the final tree
appears clean. See [agent handoff](../docs/maintainers/agent-handoff.md).
