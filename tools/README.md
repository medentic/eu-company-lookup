# Repository tooling and local evidence

These Python tools support maintenance; none are part of the published Rust API.
Read [collector workflows](../docs/development/collectors.md) for exact commands
and [testing/evidence](../docs/development/testing.md) for review semantics.

| Tool | Responsibility |
| --- | --- |
| `check_docs.py` | Validate repository Markdown targets and headings without fetching external links. |
| `check_distribution.py` | Inspect Cargo's current package file list, allowed contents and consumer links. |
| `check_public_source.py` | Enforce approved fixture paths and reject other collected records/queues in source and history. |
| `check_vat_checksums.py` | Compare local Rust checksum arithmetic with pinned development-only python-stdnum using deterministic synthetic inputs; no provider calls or evidence access. |
| `vies_batch.py` | VIES collection orchestration and explicit report saving. |
| `vies_evidence.py` | Read-only manifests, journals, reviews and replay/source integrity checks; one cached input view per run evaluation. |
| `vies_reporting.py` | Pure case classification and report statistics from loaded evidence; shared Rust/Python status precedence. |
| `vies_binary.py` | Locked, hash-checked executable freezing into an explicitly supplied cache. |
| `vies_verification.py` | Read-only offline distinct 200-valid/1-invalid country gate over loaded evidence and reports. |
| `vies_recovery.py` | Bounded multi-country recovery, availability observations, deferral and resumable single-ID batches; offline report by default. |
| `registry_fixtures.py` | Local-only committed registry replay, bounded refresh, explicit acceptance and active coverage. |
| `registry_fixture_data.py` | Prepared payloads, immutable observations/reviews and integrity checks. |
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
queues/raw corpora are excluded from Git and the crate. The approved
[prepared registry fixtures](../tests/fixtures/registry/README.md) are committed but
run locally and remain outside the crate; an ignored path is not permission
to delete its contents. Classify a file before cleanup, especially under `target/`.
The command layer loads repository configuration explicitly. Evidence loading never
imports a collector; reporting evaluates loaded values without filesystem access.
Collection and the batch `report` command explicitly save `report.json`. Country
audits and recovery summaries evaluate the same report without rewriting batch
reports. Report fields, status precedence and older manifest formats remain shared.

VIES collection keeps one hash-checked frozen executable per digest under
`target/vies-probe-binaries/`, shared by new batches and retained after completion.
This avoids deleted per-batch executables remaining open inside Docker's VM.
Preserve these executables while their batches may need resuming; older manifests
continue to use their existing `probe-bin` copies. Retention is not an automatic
cleanup service: before removing an obsolete cached executable, check all referring
manifests and active processes, confirm no run needs it for resumption or verification,
and preserve required evidence. Hashes and original journals are not cache.
The public source-history scan is required before a push, even if the final tree
appears clean. See [agent handoff](../docs/maintainers/agent-handoff.md).

`registry_validation.py` classifies company validation independently of parser
agreement. Unknown/null lifecycle is unresolved, never a definitive negative.
Commercial Register reports expose separate validation counts/completion;
BULSTAT checks require a reviewed known lifecycle and bound replay. See
[registry validation outcomes](../docs/development/testing.md#registry-validation-outcomes).

Registry validation reports reject empty/duplicate selections and retain unresolved
native lifecycle details. Reviewed negatives remain scoped to the source register.
