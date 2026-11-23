# Repository guidance for agents

Read [README.md](README.md) and [CONTRIBUTING.md](CONTRIBUTING.md) first.
[API stability](docs/reference/api-stability.md) defines the consumer contract;
[release preparation](docs/maintainers/releasing.md) defines the publication gate.

For maintenance without prior chat context, use the
[agent handoff guide](docs/maintainers/agent-handoff.md) and
[source domain map](src/README.md). Scoped instructions live in
[src/AGENTS.md](src/AGENTS.md) and [tools/AGENTS.md](tools/AGENTS.md).

## Registry design

For registry work, read the [shared field contract](docs/reference/company-result.md)
and [adapter implementation guide](docs/development/registry-adapters.md). Common
company concepts stay at the top level with consistent meanings; native codes,
status text and provider-specific facts stay in typed `registryDetails.data`.
Missing common facts remain null. Provider-local pure mappings own conversion;
keep the documented BG VIES orchestration in the Bulgarian company adapter.
Do not introduce universal prefix inference or undocumented address substitutions.

## Scope and invariants

- Version 5.0 source covers VIES and the national adapters in the README support
  table. Implementation, reviewed live coverage and publication are separate gates.
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
- VIES country completion requires at least 200 distinct VAT IDs with matched
  valid responses and at least one distinct VAT ID with a matched invalid response.
  National company/tax lookup separately requires 200 distinct IDs. Both require
  live observations, reviewed source-bound expectations and passing offline replay.
  Provider failures do not count toward either VIES threshold. Report negative
  observations, provider failures and ambiguities separately; request counts alone
  do not establish completion.
- Default to offline tests. Live calls require explicit identifiers or a bounded
  selection authorized by the user. Never run every ignored test as a shortcut.
- Reviewed, prepared national-registry fixtures belong in `tests/fixtures/registry/`
  so a fresh checkout can replay them. Follow its README: preserve immutable
  observations, source hashes, review revisions and explicit latest-review pointers.
  Remove unused personal/contact/financial data before acceptance; keep original
  captures and unreviewed queues local. Never commit credentials or browser sessions.
  Fixture replay and live refresh are explicit local workflows, never CI jobs.
  Live refresh stages pending observations; only reviewed acceptance updates replay.
  Track at least 200 distinct active cases separately from resolved non-active cases,
  unresolved records and access failures. Historical fixtures do not prove current status.
  All collected fixtures, provenance and local tools must stay outside the crate.
  Run `python3 tools/check_public_source.py --history HEAD` before a source push.
- Never replace expected results simply to pass tests. Preserve source observations,
  fixture provenance, review revisions and append-only journals.
- Keep current behavior in docs; put reviewed IDs and snapshot provenance only in
  the approved repository fixtures. Keep timing tables and disposable batch reports local. Do not copy chat transcripts or local paths into docs.
- Keep the [country and VIES output examples](docs/examples/README.md) current.
  Before editing an example page, check the latest provider code and shared contracts.
  When fields, mappings, validation, errors or lookup behavior change, refresh the
  affected JSON and positive, negative and edge-case notes in the same change.
  Before release, review every example against the exact candidate and regenerate
  any stale output offline from synthetic fixtures through the current serializers
  and client error paths. Update version notes, the example index and README links
  as needed; never substitute collected production records or imply live coverage.
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
