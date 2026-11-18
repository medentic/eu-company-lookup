# Testing and evidence

Normal tests are offline and use synthetic HTTP fixtures. Collected responses
are retained locally for separate opt-in regressions. The complete check commands are in
[CONTRIBUTING.md](../../CONTRIBUTING.md#setup-and-checks).

## Country completion minimum

Before declaring a country's VIES work complete, verify at least 200 distinct VAT
IDs with matched valid responses and at least one distinct VAT ID with a matched
invalid response. These are separate thresholds: invalid results cannot substitute
for the 200 valid results, and provider failures count toward neither threshold.
Syntax rejection alone is not a matched invalid VIES response.

National company/tax lookup separately requires live verification of at least 200
distinct IDs. These are minimum samples per operation, not permission for unbounded
collection or proof of exhaustive national coverage. Select sourced identifiers
and retain provenance.

Report matched positives, matched negatives, provider failures, parser failures
and unresolved ambiguities separately. Repeated attempts do not increase distinct
coverage. Request completion alone is insufficient: retain original observations,
review source-bound expectations and pass offline replay against the final parser.
A decoder fix must preserve its original failed observation and be verified using
that source, rather than replacing expected values to make a test pass.

Ordinary CI remains offline. These are review gates; no background collector or
automatic live country-completion check is scheduled by this guidance.

## Test responsibilities

| Location | What it verifies |
| --- | --- |
| `tests/` | External-consumer identifiers, configuration, exported models and capabilities. |
| `src/vies/tests/` | VAT syntax, SOAP identity/fault handling and bounded HTTP behavior. |
| `src/vies/address/` | Address extraction and reviewed VIES regressions. |
| `src/bg/tests.rs` | Registry identity, fallback, field decoding, failures and reviewed Commercial Register regressions. |
| `src/ro/tests.rs` | ANAF identity/date matching, source fields, failures and bounded transport. |
| `src/locations/` | Shared normalization and delivery-line behavior. |
| `tools/test_*.py` | Collector/report behavior and documentation/distribution boundaries. |

Private transports and decoders remain private. Test-only endpoint overrides and
probes must not become public API merely to simplify tests. Synthetic cases ship
in the crate. Collected corpora, identifier queues and their provenance are excluded
from both public Git history and the crate. Public CI checks this file boundary
and runs the synthetic suite; it does not claim to validate the local corpora.

Focused checks:

```sh
cargo test --locked bg::tests
cargo test --locked vies::tests::probe
```

The last command tests the retry experiment offline. Ignored tests include both
live collection and offline replay: invoke specific tests, never all ignored tests.
See [live collectors](collectors.md) when new source observations are needed.

## Independent checksum comparison

For changes to VAT checksum arithmetic, run the repository-only differential
check with the pinned Rust toolchain and Python 3.9+. Install the reference library
in an isolated development environment (installation needs package access):

```sh
python3 -m venv target/checksum-oracle-venv
target/checksum-oracle-venv/bin/python -m pip install python-stdnum==2.2
target/checksum-oracle-venv/bin/python tools/check_vat_checksums.py
```

The check itself is offline. It compiles the actual private Rust rules with
`rustc` in a temporary directory, compares synthetic seeds and deterministic
check-character mutations across all 28 prefixes, and removes its temporary
files. Disagreements, missing results, unsupported local verdicts or an unexpected
oracle version fail the command. Neither Cargo dependencies nor the public API
change; the Python dependency is used only by this explicit development check.

Inputs cover supported checksum variants rather than all national validation
rules. In particular, Dutch branch `00` is excluded because python-stdnum rejects
its format while our preflight defers that constraint to VIES. Birth dates and
other body constraints are preserved when generating mutations. Deliberate
unsupported-variant fallbacks remain covered by Rust unit tests. This comparison
does not contact VIES, read or modify collected evidence, or count toward country
completion. Normal Python tool tests do not require python-stdnum or a compiler.

## Local collected-data checks

Original observations may contain personal names, contact details and record links.
Keep them in the ignored fixture paths below; preserve their bytes, review history
and hashes. A fresh public clone does not include them. Do not download them from
providers as an implicit setup step or upload them as CI artifacts.

When authorized local corpora are already installed, run only these offline tests:

```sh
VIES_COUNTRY=BG VIES_REPORT=target/vies-offline-report.json \
  cargo test --locked vies::address::corpus::saved_corpus -- --ignored --exact
cargo test --locked bg::tests::reviewed_commercial_corpus -- --ignored --exact
VIES_COUNTRY=RO VIES_REPORT=target/vies-ro-offline-report.json \
  cargo test --locked vies::address::corpus::saved_corpus -- --ignored --exact
cargo test --locked ro::probe::reviewed_anaf_corpus -- --ignored --exact
python3 -m unittest discover -s tools -p 'test_vies_batch.py'
```

The Rust corpus tests fail if explicitly requested without their local data. The
Python review-integrity test reports a skip when the local VIES corpus is absent.
For new parser regressions, add a small synthetic case that preserves the relevant
structure without copying identities, contact details or document identifiers.

`python3 tools/check_public_source.py --history HEAD` checks both tracked paths
and reachable history. Deleting a collected file in a later commit is insufficient;
an unpublished candidate must be rebuilt from a clean ancestor before pushing.
The path check does not replace human review of source literals or new fixtures.

## Adapter boundaries

VIES transport, SOAP decoding and address extraction are separate private modules.
Production and probes share transport primitives, while admission, retries and
fault policy belong to their callers.

The Bulgarian client owns fallback and shared admission. `commercial.rs` selects
matching main-deed sections and checks UIC/field identity before extraction;
`bulstat.rs` checks exact search/detail identities and constructs fixed report URLs.
`html.rs` extracts visible text; `address.rs` and `legal_form.rs` handle registry
labels before shared normalization. Probe code is test-only.

Commercial Register requests use `/CR/api/Deeds/{uic}`, the current Europe/Sofia
end-of-day `entryDate`, and `loadFieldsFromAllLegalForms=false`. Managers come from
field 7 and sole capital owners from field 23. BULSTAT uses the BIRT `preview`
renderer with `rpt1.rptdesign` for search and `v_rpt1.rptdesign` for details.
Test these observed boundaries without exposing endpoint overrides publicly.

## Fixture ownership

| Data | Location and purpose |
| --- | --- |
| Local VIES response expectations | `src/vies/fixtures/countries/`: one JSON array per country. |
| Local VIES reviews | `src/vies/fixtures/countries/reviews/`: ambiguity, revisions, dates and expectation hashes. |
| Normalization policy | `src/vies/fixtures/review-policy.json`: current and accepted review versions. |
| Shared reporting contract | `src/vies/fixtures/report-contract.json`: Rust/Python status precedence cases. |
| Local Commercial Register cases | `src/bg/fixtures/commercial/bg.json`: one JSON array with raw responses, source hashes, scoped expected fields and review history. |
| Local ANAF cases | `src/ro/fixtures/anaf/ro.json`: raw responses and source-bound reviewed expectations. |
| Local input queues | `src/vies/fixtures/ids/` and `src/bg/fixtures/ids/`: identifiers and provenance, not registration assertions. |

Read current counts from the data. Provenance dates describe snapshots and reviews,
not current registration. Preserve source text, hashes, review revisions and audit
journals when reorganizing code or docs.

## Reviewing parser changes

Inspect raw source, add a focused synthetic regression, then review affected
expectations independently of parser output. Never copy new output into expected
values just to get a passing test. Keep ambiguities explicit even when conservative
expected fields match.

For VIES changes, record the reason, review date/revision and canonical expectation
hash. Deliberate normalization changes require a version decision in the review
policy; structural changes that preserve output do not. Tests check review coverage
and hashes. Commercial Register reviews bind source hashes to explicit field paths;
missing fields differ from null. Retain previous reviews when expectations change.

A review covers only its stated fields. Address reviews do not certify role holders,
capital, documents or every source format. Live-coverage limitations belong in the
[consumer reference](../reference/bulgarian-registry.md#coverage-limitations), not in
success-count claims.

## Reading reports

Collection, parsing and review are separate stages. `complete` means every selected
ID has an observation, including failures. A passing regression means agreement
with reviewed expectations, not universal address correctness or legal verification.

VIES reports distinguish pending, provider error, missing review, validity change,
source change, parser mismatch, ambiguity and pass. The overall pass percentage
uses all selected IDs; a negative VAT response is a response, not a provider error.
Python reports require dated, versioned reviews with matching expectation hashes
before reporting a pass. `verifiedValid` and `verifiedInvalid` additionally require
an explicit current binary manifest, a matching offline replay proof and a binding
to the original source or source-review journal. Without the manifest, verified
counts are zero. A parser regression pass alone is not country acceptance.

The checked-in offline country gate accepts one or more local batch directories:

```sh
python3 tools/vies_verification.py --country EL \
  --binary-manifest "$BINARY_MANIFEST" --run-dir "$BATCH_DIR"
```

Repeat `--run-dir` for additional batches. The command makes no network calls and
exits 0 only with at least 200 distinct verified valid IDs, one distinct verified
invalid ID, and no unresolved review/replay or conflicting-validity observations.
It exits 1 when the acceptance gate is unmet and 2 for invalid input. Provider failures are
reported separately and never count as negatives. Proofs bind source and executable
hashes, exact observation/replay bytes and the offline test log; changed inputs
require another offline verification. Paths in existing manifests and proofs are
relative to the repository root unless absolute. Original evidence is preserved. Country audits and recovery summaries are read-only
with respect to batch files, including existing `report.json`. Use the explicit
`vies_batch.py report` command to save a refreshed batch report. Evidence loading,
pure report evaluation and command output have separate owners in the
[tooling guide](../../tools/README.md).

Reports identify original versus replayed parsing. The reader accepts supported
older journals without rewriting them and rejects conflicting or mismatched records.

Commercial Register reports retain `initialOutcomes` separately from replayed
`outcomes`. Review status distinguishes missing review, pass, ambiguity, parser
mismatch, source change and operational/parse failure. A Commercial Register miss
alone is not absence from BULSTAT. Schema-2 replay binds parser/binary hashes,
crate version and per-case source/observation hashes; older supported replay arrays
remain explicitly unbound.

Keep scheduler wait, client admission wait, HTTP/body time and total duration
separate. Never infer missing timings/status codes or backfill older observations.
Raw evidence and versioned reviews are retained; derived reports and completed
executable copies are disposable when no longer needed.

## Registry validation outcomes

Parser agreement and successful company validation are separate checks. A replay
may correctly reproduce a missing or unknown lifecycle, but that case is
`unresolved`: it counts as neither a verified positive nor a verified negative.
A matched company with a resolved lifecycle (including closed or documented
proceedings) is a positive identity match, provided its
lifecycle is explicitly reviewed against source evidence. Missing optional facts
(such as a separately registered foreign name) do not establish a negative case.

National company replays write `validationStatus` per case and a separate
`validation-summary.json`. `validationComplete` means every selected case has a
resolved, reviewed outcome, not that a country's 200-ID coverage gate is met.
Summaries compare the original manifest with replay identities. Missing, duplicate
or unexpected IDs prevent batch and validation completion; partial journals remain
retained recovery evidence, never completed batches. Duplicate identities cannot
increase qualifying coverage. A reviewed null status
can still pass parser regression while leaving validation incomplete.

Definitive negatives require the provider's actual absence contract. Czech replay
accepts a hash-bound `outcome: "notFound"` review only with captured HTTP 404 and
the documented `NENALEZENO` response envelope. Empty Austrian active-only search
remains unresolved. ANAF replay distinguishes decoded `notFound` observations
from errors and unresolved records; unreviewed observations are not verified cases.

Commercial Register reports expose `validation` counts and `validationComplete`
separately from request completion and parser review. A definitive Commercial
Register miss is a `negativeObservation` within that register's scope, not proof
that both Bulgarian registers missed. BULSTAT `checksPassed` requires explicit
lifecycle review and successful bound replay; unreviewed or unresolved records
cannot pass it. Raw observations and review history remain unchanged.

Unresolved cases include the source, preserved native lifecycle text and a reason:
`missingSourceStatus` or `unmappedSourceStatus`. These are review leads, not guessed
translations. Romania accepts optional `expectations.json` keyed by requested ID,
with `bodySha256` and either reviewed JSON-pointer `fields` or
`outcome: "notFound"`; the decoder must independently confirm the exact miss.
Commercial Register negative reviews use the existing source-hash-bound review
contract with `expected: {"outcome": "notFoundInCommercialRegister"}`.
Successful bound replay is still required for validation completion. This verifies
only the Commercial Register miss, never absence from both Bulgarian registers.


## Read-only retained-response audit

For diagnosis without changing existing expectations, set `REGISTRY_AUDIT_INPUT`
to a local JSON array of `{ "country": "PL", "path": "/absolute/observations.jsonl" }`
entries and `REGISTRY_AUDIT_OUTPUT` to a separate derived-output folder. Run only
`cargo test --locked --lib audit_retained_registry_responses -- --ignored`.
The audit verifies captured body hashes and identity, reports every observation,
and records old-review differences and unresolved-data errors. It makes no network
requests and never rewrites observations or expectations. Reports are local-only.
This diagnostic is not a completion certificate: original selection manifests,
missing observations, duplicate IDs and source-bound review coverage must still
be checked by the batch validation workflow. A passing audit test means the audit
finished, not that every reported case resolved or passed its existing review.

When a mapping change makes an expectation stale, review the captured provider
fields against the provider's classification before creating a review revision.
Do not copy the audit's actual output into expectations. Preserve the previous
review and its source binding; replay the revised review against the original
selection. Supplemental lifecycle responses must be captured and hash-bound too:
changing a base-record expectation cannot replace missing lifecycle evidence.

Report stale reviews, unresolved source facts, operational failures and sample-size
shortfalls separately. A newer passing review can supersede an older review for
the same identifier without deleting the older observation. A matched unresolved
record is still useful evidence: bounded collection may continue past it, while
provider or parser failures require recovery before collection resumes. Neither
case contributes to resolved coverage until source review and replay pass.
