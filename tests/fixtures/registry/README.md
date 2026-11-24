# Reviewed registry fixtures

These committed real-response fixtures let a maintainer continue verification from
a fresh checkout. National adapters and the VIES country samples have versioned corpora containing
identifiers, prepared source responses, source hashes, reviewed expectations and
history. Unreviewed queues and original full captures remain local.

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
If preparation was interrupted or improved, re-stage the retained capture without
another live request:

```sh
python3 tools/registry_fixtures.py stage --country RO --run-dir target/ro-refresh
```

This regenerates pending review files, so preserve any manual draft before staging
again. Accepted corpus history and original capture journals are unchanged.

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

National corpora managed by `registry_fixtures.py` have content-addressed immutable
`records`, per-ID `history`, a `current` reference per ID and a `replay` selection.
Each record stores an
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


## Additional national adapters

The registry workflow supports AT, BE, CZ, EE, FI, FR, GR, IE, IS, LT, LV, NO,
PL, RO and SK. JSON preparation retains adapter inputs and original source hashes;
Polish officer lists retain only counts where that is all the lifecycle adapter
consumes. Unknown Polish lifecycle fields stop preparation for review.

Icelandic preparation retains selected company HTML and removes session/cart
values consistently. Its PDF is a clearly marked prepared excerpt containing only
the independently extracted company identifier, issue date and status. Original
captures remain local; original hashes remain in provenance. Preparing a new
Icelandic PDF requires Poppler (`pdftotext`) and Python ReportLab; set
`REGISTRY_PDF_PYTHON` to a Python executable with ReportLab when necessary.
Offline replay uses the prepared PDF and needs neither preparation dependency.

## VIES and Bulgarian Commercial Register

`VIES/<prefix>/corpus.json` and `BG/corpus.json` preserve their existing scoped
reviews and all accepted/imported observation revisions. These use separate local
entry points because their collector and replay contracts differ from the other
national adapters. VIES valid/invalid counts never count as national active cases.

Build the current library test executable locally with `cargo test --locked --lib
--no-run`, then use its reported path with an existing compatible local Docker
image (the tools never pull an image):

```sh
python3 tools/replay_fixtures.py --docker-image "$RUST_IMAGE" --binary "$TEST_BINARY"
# Or select an individual provider:
python3 tools/vies_fixtures.py replay --docker-image "$RUST_IMAGE" --binary "$TEST_BINARY"
python3 tools/vies_fixtures.py coverage
python3 tools/bg_fixtures.py replay --docker-image "$RUST_IMAGE" --binary "$TEST_BINARY"
python3 tools/bg_fixtures.py coverage
```

These commands mount temporary materialized copies over the legacy ignored fixture
paths inside the container. They never overwrite local legacy corpora, and replay
runs without container networking. Historical observations replay separately to avoid duplicate identifiers
overwriting earlier expectations. Superseded reviews remain in history; only the
latest review of the same captured observation is asserted. Normal CI does not load
these datasets.

After explicitly bounded collection using the existing collector tools, ingest
only the selected retained observation files:

```sh
python3 tools/vies_fixtures.py ingest target/selected-vies/observations.jsonl
python3 tools/bg_fixtures.py ingest target/selected-bg/observations/000000000.json
```

Ingestion makes no network requests. It carries forward expectations only when
retained source facts match the preceding reviewed source. A changed/new source is
preserved as unresolved without copying parser output into expectations; access
failures remain separate. Run replay after ingestion and review unresolved sources
before claiming completion. Undated imported historical samples do not establish
fresh live coverage. The archived VIES import command is a one-time migration and
refuses to replace an existing corpus.

The Bulgarian prepared scope excludes roles, financial data and documents, and
removes contact suffixes embedded in office text from both source and the scoped
full-address expectation. Original source/review hashes remain in provenance;
these excerpts do not represent complete official extracts. Negative Commercial
Register observations remain register-specific, not claims about BULSTAT.


BULSTAT has its own corpus at `BULSTAT/corpus.json` and local
`tools/bulstat_fixtures.py` entry point with the same `replay`, `coverage` and
`ingest` commands as the Bulgarian Commercial Register workflow. Its prepared
records retain the complete fallback chain: definitive Commercial Register miss,
BULSTAT search, then company detail. Only reviewed company rows are retained;
action/history columns and unrelated sections are removed. Its coverage is
reported separately from Commercial Register coverage.

A partially captured national lookup can replay as unresolved while the collector
recorded a transport failure fetching a later required source. Such a fixture
retains the typed collection failure and is counted as an access failure, not as
an unresolved company status. Earlier complete observations remain replayable.
