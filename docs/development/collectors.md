# Live development collectors

Collected observations, input queues and source metadata are local-only. Their
fixture paths are ignored by Git; preserve them locally and never upload them in
source commits, pull requests, CI logs or artifacts. A fresh clone contains the
tooling and synthetic tests, not real lookup queues or reviewed response corpora.


Collectors produce source observations for development; they are not public library
APIs. Run them only for an explicitly authorized, bounded selection. Ordinary CI
and parser work stay offline. See [testing and evidence](testing.md) for review rules.

## Setup and shared rules

Use a full checkout and Python 3.9+ on a POSIX host. Build the library test executable
immediately before collection and use the path printed by Cargo:

```sh
cargo test --locked --lib --no-run
TEST_BINARY=/path/to/compiled-library-test-executable
```

Both collectors freeze the binary, source hashes, queue checksum and selection.
Offsets are zero-based; specify both offset and count, with 1–100 IDs per run.
Use a fresh run directory. Resume with the same arguments plus `--resume`.
Existing observations, including failures, are skipped; a recheck requires a
separately authorized run that preserves the original history.

Docker execution requires an image already available locally and a run directory
inside the repository. Images are never pulled by the collectors. Locks do not
coordinate unrelated applications or other checkouts. Inspect process ownership
before recovering a stale lock; never delete state to bypass a provider cooldown.
Keep original observations and journals intact. Completed runs remove their frozen
executable copy. No collection is scheduled by these instructions.

## VIES

### Import a queue

Given a downloaded NRA bulletin and `bsdtar`:

```sh
python3 tools/import_bg_vat_ids.py /path/to/VATCompanies_YYYYMMDD.7z
```

The importer writes `src/vies/fixtures/ids/bg.json` and adjacent `bg.source.json`
with the archive name, timestamp, checksums and selection details. Review both
outputs together. It selects nine-digit VAT IDs with a registration date, no
deregistration date and a known region, using stable ranking across 28 regional
groups. Importing neither queries VIES nor creates address expectations.

### Collect and replay

```sh
python3 tools/vies_batch.py collect --offset 0 --count 10 \
  --run-dir target/vies-review --binary "$TEST_BINARY"
VIES_REPLAY_DIR=target/vies-review \
  cargo test --locked replay_probe_journals -- --ignored --nocapture
python3 tools/vies_batch.py report target/vies-review
VIES_REPLAY_DIR=target/vies-review \
  cargo test --locked validate_saved_typed_journals -- --ignored --nocapture
```

Add `--docker-image <existing-local-image>` for a Linux executable on a non-Linux
host. Resume checks the queue, selection, frozen binary and Docker image. Replay
writes a separate derived snapshot and never changes observations. Partial JSONL
tails are recovery errors, not permission to truncate journals. Growth is bounded
near 20 MiB per run.

### Test-only retry policy

The collector uses a six-second total lookup budget. Only HTTP 502 permits retries:
an immediate second attempt, then a one-second wait before a third if budget remains.
Each attempt receives only the remaining budget. Retry-After stops immediate retries;
other statuses, SOAP faults, transport errors and expired deadlines are not retried.
Distinct lookups are spaced five seconds after a response and sixty after failure,
outside the lookup budget. Retry-After stops collection for review.

This policy is an experiment, not production behavior or a latency promise.
Production remains single-attempt. Shared transport handles request construction,
bounded bodies and SOAP parsing; HTTP 408/429/502/504 bodies are skipped. Timing
reports include failed lookups and retry waits, excluding inter-lookup pacing.
Successful samples establish neither a quota nor permission for bulk reuse.

For a small standalone experiment, use approved identifiers and a fresh journal:

```sh
VIES_PROBE_IDS='<authorized-VAT-ID>' VIES_PROBE_REPORT=target/vies-probe.jsonl \
  cargo test --locked live_six_second_probe -- --ignored --nocapture
```

### Refresh a saved fixture

The separate `live_corpus` test uses the single-attempt client with a 25-second
timeout and additional sequential pacing. It rechecks saved fixtures, not the ID
queue, and has no automatic resume. Use a fresh progress path and a small limit:

```sh
VIES_COUNTRY=BG VIES_LIMIT=1 VIES_REPORT=target/vies-refresh.json \
  VIES_PROGRESS=target/vies-refresh.jsonl \
  cargo test --locked live_corpus -- --ignored --nocapture
```

It reports source/validity changes and never rewrites expectations.

## Commercial Register

This collector shares the library's Commercial Register transport and decoder but
never queries BULSTAT. It requires Docker and a compiled Linux test executable.

### Import and collect

```sh
python3 tools/registry_batch.py import /path/to/business-codes.txt src/bg/fixtures/ids/bg.json
BUILDER_IMAGE=your-existing-local-rust-image
python3 tools/registry_batch.py collect \
  --queue src/bg/fixtures/ids/bg.json --offset 0 --count 10 --interval 30 \
  --run-dir target/registry-review --binary "$TEST_BINARY" \
  --docker-image "$BUILDER_IMAGE"
```

Import preserves unique supported nine-/thirteen-digit codes, order and leading
zeroes, recording source checksums, duplicates and rejected line numbers. The
interval is 10–300 seconds, default 30, frozen with the image and policy. It is a
collection setting, not a known provider quota; throttling can still occur.

Observations are atomic. Provider failures stop the run and persist cooldown;
attempted IDs are not retried automatically. Checkout-wide scheduling uses
`target/registry-provider.lock` and `target/registry-provider.json`, preserving
next-start/cooldown and an interrupted-request reservation. Incomplete legacy runs
without coordinated scheduling need a new run, preserving their original observations.

### Replay and report

```sh
REGISTRY_RUN_DIR=target/registry-review \
  cargo test --locked replay_commercial_batch -- --ignored --nocapture
python3 tools/registry_batch.py report target/registry-review
```

Replay and report output stay separate from original observations. Interpret review
scope, provenance and timing using [testing and evidence](testing.md#reading-reports).
Neither a received response nor a completed run automatically approves a record.

### Consolidated Commercial Register corpus

Keep reviewed Commercial Register cases in one local-only JSON array,
`src/bg/fixtures/commercial/bg.json`. Each case retains its complete raw response
and review object, including source SHA-256, scope, expectations and revision history.
The ignored corpus regression loads this file and rejects duplicate identities.
Live run observations remain separate, immutable evidence; failed requests do not
become company fixtures.

After source review and offline replay, merge passing cases with:

```sh
python3 tools/registry_batch.py corpus --run-dir target/registry-review \
  --output src/bg/fixtures/commercial/bg.json
```

The merge checks replay/source bindings, accepts only passing scoped reviews,
preserves existing cases and refuses conflicting replacements. Updating an existing
expectation requires explicit re-review and preserved revision history.

For a legacy directory of individual case files, run
`registry_batch.py consolidate SOURCE_DIRECTORY OUTPUT_JSON`. This copies complete
objects into one array and verifies the read-back. It leaves originals intact for comparison; remove duplicates only
after lossless verification and the saved-corpus regression pass. Corpora and
migration evidence remain excluded from public Git and the crate archive.

## BULSTAT fallback checks

The separate BULSTAT runner exercises `BgRegistryClient::lookup`, including the
Commercial Register miss and both BULSTAT reports. It does not bypass fallback or
change the production client policy. Response capture exists only in test builds
and is enabled only for the selected observation.

Prepare a local JSON queue under `target/` with a `cases` array containing one to
ten objects. Each object needs a string `taxId`, a `name`, and a public HTTPS
`sourceUrl` documenting that identifier. Preserve leading zeroes and full branch
codes. Source documentation supplies candidates; only a successful exact live
match establishes coverage. Do not put the queue or responses in Git.

After building the library test executable as above:

```sh
python3 tools/bulstat_batch.py collect \
  --queue target/bulstat-selection.json --run-dir target/bulstat-review \
  --binary "$TEST_BINARY" --docker-image "$BUILDER_IMAGE"
BULSTAT_RUN_DIR=target/bulstat-review \
  cargo test --locked bg::bulstat_probe::replay_bulstat_batch -- --exact --ignored
python3 tools/bulstat_batch.py report target/bulstat-review
```

Omit `--docker-image` for a native executable. For Linux execution, use an existing
local image; no image is pulled. Live runs share the Commercial Register collector
lock and persisted scheduling state. Each lookup uses the default 30-second total
deadline and no artificial spacing within a lookup, with an additional 30-second
pause after completion. Errors stop collection and persist a cooldown, including
Retry-After. There are no automatic retries. `--resume` continues only unattempted
identifiers with the original frozen binary, selection and policy.

The manifest retains identifier provenance, source hashes and the executable hash.
Observations retain successful response bytes, response content types, per-request
transport metadata and typed outcomes. Error response bodies are not collected.
Completed runs remove their frozen executable copy. Replay is offline and checks
that a successful result has an empty Commercial Register response followed by
exact BULSTAT search/detail matches; its output binds each original observation
by SHA-256. Replay equality is a regression check, not independent field review. A preserved,
source-bound review revision can approve an intentional result change; replay
then checks that revision while still reporting differences from the live result.
Keep separately reviewed expectations and original observations intact.

Optional `review.json` is an object keyed by selected identifier. Each review has
`schemaVersion: 1`, `observationSha256` (the original observation file hash), a
nonempty `scope` list, and an `expected` object mapping dotted result-field paths
to reviewed values. Retain review revisions separately instead of silently
replacing expectations. The report validates observation hashes, compares scoped
expectations and reports replay equality separately; mismatches fail the command.

A completed batch is not necessarily successful BULSTAT coverage: Commercial
Register hits, definitive misses and operational errors remain distinct outcomes.
The command exits unsuccessfully unless every selected identifier returns BULSTAT.
Natural-person and thirteen-digit branch coverage require separately sourced cases;
ordinary institutional records do not establish that coverage.

## Romanian ANAF

`tools/ro_batch.py` freezes an explicitly authorized selection of 1–100 CUIs per
run. Supply a local-only queue with a `cases` array; each object contains `taxId`
and a public HTTPS `sourceUrl`. Keep source snapshots, checksums and selection
method alongside the queue under ignored `target/` evidence paths.

```sh
python3 tools/ro_batch.py --queue target/ro-selection.json \
  --offset 0 --count 100 --run-dir target/ro-review \
  --binary "$TEST_BINARY" --docker-image "$BUILDER_IMAGE"
RO_RUN_DIR=target/ro-review \
  cargo test --locked ro::probe::replay_anaf_batch -- --exact --ignored
```

The probe exercises the production ANAF lookup and captures successful HTTP body
text before decoding. The manifest binds the executable, Rust sources, selection,
source provenance and requested fiscal date. Original observations are append-only
and contain body hashes, timing, transport metadata and typed outcomes. Offline
replay writes a separate derived file. A parser failure remains in the original
journal after it is fixed; replay does not rewrite history.

Calls are sequential with five seconds between completed lookups, no automatic
retries, and stop on provider or parser errors. Review failures before using
`--resume`; attempted identifiers are never automatically repeated. The wrapper
shares an ANAF process lock and persisted pacing/cooldown across runs, including
Retry-After instructions. An interrupted process requires ownership inspection
before recovery. Keep captured records and reviews local; do not publish them.
