# Testing and evidence

Normal tests are offline and use synthetic HTTP fixtures. Collected responses
are retained locally for separate opt-in regressions. The complete check commands are in
[CONTRIBUTING.md](../../CONTRIBUTING.md#setup-and-checks).

## Country completion minimum

Before declaring a country's VAT and company-lookup work complete, verify at least
200 distinct VAT IDs and 200 distinct national company/tax IDs live. This is a
minimum sample per operation, not permission for unbounded collection or proof of
exhaustive national coverage. Select sourced identifiers and retain provenance.

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
