# Reviewed registry fixtures

These committed real-response fixtures let a maintainer continue verification from
a fresh checkout. AT, CZ, GR, NO, RO and SK each have a versioned corpus containing
identifiers, prepared source responses, source hashes, reviewed expectations and
history. Other country collections have not been migrated by this workflow.

Both offline replay and live refresh are **local-only**, explicitly invoked.
Neither runs in Linux CI or ordinary Cargo tests. The command rejects a true
`CI` environment variable. Fixtures, Python maintenance tools and this guide are
excluded from the published crate by Cargo's explicit include list.

## Replay and coverage

Install the pinned Rust toolchain and Python 3.9+ and provision Cargo dependencies
using the normal contributor setup. These commands never fetch dependencies or
contact providers:

```sh
python3 tools/registry_fixtures.py replay
python3 tools/registry_fixtures.py replay --country RO
python3 tools/registry_fixtures.py coverage
python3 tools/registry_fixtures.py coverage --country RO --require-active
```

An existing local Docker image with the pinned Rust toolchain may be selected with
`--docker-image IMAGE` **before** the subcommand. It is never pulled. Docker replay
runs with networking disabled and uses the repository's existing
`target/cargo-home` and `target/build` caches.

Replay exercises the current private Rust decoders and common-result projections,
checking reviewed fields, identity, source binding and expected classification.
It covers historical observations as well as current accepted observations.
Superseded reviews of the same observation are retained, but only their accepted
replacement is an assertion. Archived pre-migration reviews remain in provenance.

Coverage counts the latest accepted observation **once per identifier**. Active,
closed, inactive and liquidation cases are distinct; unresolved records, access
failures and other source failures have separate categories. The active target is
200 per country. A successful historical replay does not establish current live
status or satisfy an active shortfall. Dates and shortfalls are shown explicitly.
Slovakia remains incomplete; do not turn its access blocker into a successful gate.

## Bounded live refresh and acceptance

Live refresh requires explicit authorization and a bounded selection. Use existing
IDs or a reviewed selection of new IDs. A batch accepts at most 100 IDs. Split a
full sample into deliberate batches; never run all ignored tests together.
The provider probes retain their existing pacing and stop-on-failure behaviour.
Do not retry blocked providers automatically.

```sh
python3 tools/registry_fixtures.py refresh --country RO --limit 10 --run-dir target/ro-refresh
```

Alternatively supply `--ids ID1 ID2` instead of `--limit`. The run directory must
be new, under `target/`. Collection saves its original append-only journal there.
A partial failed batch remains available for investigation and returns exit code 2.

Refresh writes prepared `staged.json`, `changes.json` and `review-template.json`
into that run directory. **It does not change committed expectations.** Inspect
source responses, identity/date matching and lifecycle evidence. Copy the template
to a review file, select the observed records to accept, review the expected fields
and categories, and replace every PENDING rationale with a source-bound explanation.
The tool's actual result is evidence for investigation, not an automatic oracle.

```sh
python3 tools/registry_fixtures.py accept --run-dir target/ro-refresh --review-file target/ro-refresh/reviewed.json --reviewer "Maintainer name"
python3 tools/registry_fixtures.py replay --country RO
python3 tools/registry_fixtures.py coverage --country RO --require-active
```

Acceptance first replays the proposed review. Only matching reviewed records are
appended to the corpus. Previous response and review objects remain immutable.
Thus an accepted live refresh updates offline replay too, without losing history.

If an active company closes, the refresh reports the changed expectation. After
source review, accept its closed status and keep its earlier active observation.
Its latest observation stops counting as active. Add and verify more distinct
active companies to restore 200; do not delete closed cases or alter expectations
merely to make a test pass. An access failure or unresolved lifecycle never counts
as active. Retain unresolved TODOs and access blockers separately.

Review the Git diff and commit accepted changes with a meaningful explanation.
No command pushes, publishes or schedules future runs.

## Format and provenance

Each corpus has content-addressed immutable `records`, per-ID `history`, a
`current` reference per ID and a `replay` selection. Each record stores an
observation, source-bound review, reviewer, rationale, classification and provenance.
Changing a review creates another record rather than editing its predecessor.

Prepared bodies retain adapter-consumed values and omit unused contact, role and
other fields. Their hashes are recomputed and reviews bound to those prepared
bodies. Original observation/body hashes and prior reviews preserve the migration
trail. Original raw journals remain local and unchanged; replay from Git does not
depend on them. Do not commit unreviewed queues, raw bulk exports, credentials,
cookies or workstation paths. Any new country/path requires explicit boundary
review in `tools/check_public_source.py`.

Provenance identifies the official source: Austrian JustizOnline, Czech ARES,
Greek GEMI, Norwegian Brønnøysundregistrene, Romanian ANAF and Slovak RÚZ
(with RPO supplemental lifecycle evidence where captured). Provider data remains
subject to its source terms; the crate's software licence is not a blanket licence
for third-party registry data.

Run the [contributor checks](../../../CONTRIBUTING.md#setup-and-checks), including
source-history and distribution checks, before committing/pushing changes.
