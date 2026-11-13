# Repository guidance for agents

Read [README.md](README.md) and [CONTRIBUTING.md](CONTRIBUTING.md) first.
[API stability](docs/reference/api-stability.md) defines the consumer contract;
[release preparation](docs/maintainers/releasing.md) defines the publication gate.

For maintenance without prior chat context, use the
[agent handoff guide](docs/maintainers/agent-handoff.md) and
[source domain map](src/README.md). Scoped instructions live in
[src/AGENTS.md](src/AGENTS.md) and [tools/AGENTS.md](tools/AGENTS.md).

## Scope and invariants

- Version 2.0 covers VIES, Bulgarian Commercial Register/BULSTAT lookup and
  Romanian ANAF taxpayer lookup. The README priority table describes further work.
- Preserve full identifiers and leading zeroes. Syntax is not checksum or registration.
- `valid: false` requires a matched negative VIES response. `NotFound` requires both
  Bulgarian registers to miss definitively. `RegistryNotFound` identifies an ANAF
  miss only. Operational errors never prove absence.
- BULSTAT fallback follows a definitive Commercial Register miss, never a failure.
- Keep original decoded address text. Normalize only a working copy; do not invent
  missing data. Review JSON/API changes against the compatibility contract.
- Provider modules stay private; consumer tests belong in `tests/`, private parser
  and transport tests beside their modules. Avoid unrelated refactors.

## Work and verification

- Inspect the current diff before editing. Preserve uncommitted work and coordinate
  overlapping changes with other active tasks under the user's standing authorization.
- Use the contributor check commands. CI remains Linux-only; do not add other OS jobs
  without an explicit change to that preference.
- Country completion requires live verification of at least 200 distinct VAT IDs
  and 200 distinct company/tax IDs, with reviewed source-bound expectations and
  passing offline replay. Report negative observations, provider failures and
  ambiguities separately; request counts alone do not establish completion.
- Default to offline tests. Live calls require explicit identifiers or a bounded
  selection authorized by the user. Never run every ignored test as a shortcut.
- Collected records, identifier queues and their provenance stay local-only. Do not
  add them to Git or push a commit that contains them in its ancestry. Run
  `python3 tools/check_public_source.py --history HEAD` before a source push.
- Never replace expected results simply to pass tests. Preserve source observations,
  fixture provenance, review revisions and append-only journals.
- Keep current behavior in docs; keep run-specific IDs, timing tables and batch
  summaries in local evidence. Do not copy chat transcripts or local paths into docs.
- Run documentation and distribution checks after changing Markdown or package scope.
- Before/after builds, inspect free space and affected caches. Keep at least 50 GiB
  free, reuse one development cache, and retain evidence separately from disposable
  output. Unknown files are not cache; never delete active services or another
  task's storage merely to meet a size target.

## Release boundary

Local builds, packaging and publish dry runs are preparation. Do not publish,
create a release tag or announce a release until the user approves the exact
reviewed candidate. Do not claim approval, CI completion or live coverage from
local test results. Agent instructions and release checklists are repository-only
and must stay outside the crate archive.
