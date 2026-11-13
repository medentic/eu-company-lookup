# Contributing

This single Rust crate supports VIES VAT verification and Bulgarian business
lookup and Romanian ANAF taxpayer lookup. Start with [README.md](README.md) for supported behavior and
[API stability](docs/reference/api-stability.md) before changing consumer contracts.

For a new developer or AI agent, the [handoff guide](docs/maintainers/agent-handoff.md)
provides a maintenance workflow and check matrix. The [source domain map](src/README.md)
links module ownership and provider field mappings.

## Repository map

| Location | Purpose |
| --- | --- |
| `src/lib.rs` | Explicit public API exports and crate documentation. |
| `src/client.rs` | Provider-independent facade and named adapter configuration. |
| `src/model/` | Shared identities/results, capability types and typed failures. |
| `src/capabilities.rs` | Provider-independent jurisdiction catalog. |
| `src/vies/`, `src/bg/`, `src/ro/` | Private provider clients, decoders and unit tests. |
| `src/locations/` | Public address utilities; private extraction and normalization rules. |
| `tests/` | External-consumer API, identifier and configuration contracts. |
| `examples/` | Runnable consumers; executing them contacts live providers. |
| `tools/` | Repository-only collectors, reports and validation tools. |
| `docs/` | [Documentation index](docs/README.md), grouped by audience. |
| `src/**/fixtures/` | Approved synthetic fixtures; collected corpora and queues are local-only and ignored by Git. |
| `target/` | Ignored build output and local run evidence; not all contents are disposable. |

Private parser/transport tests stay beside their modules so testing does not require
public endpoint overrides. Local real-response corpora supplement synthetic tests; they
are excluded from both public Git history and the crate archive. Do not split modules or introduce a workspace
unless a concrete ownership or consumer need justifies it.

## Setup and checks

Install Rust with rustup; `rust-toolchain.toml` pins Rust, rustfmt and Clippy.
Python 3.9+ is needed for repository checks. The collectors use POSIX file locking;
Commercial Register live collection also needs Docker and an existing Linux image.
CI is Linux-only, testing the declared minimum and stable Rust.

From a full checkout:

```sh
cargo fmt --all --check
cargo test --locked --all-targets
cargo test --locked --doc
cargo clippy --locked --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-deps
python3 -m unittest discover -s tools -p 'test_*.py'
python3 tools/check_docs.py
python3 tools/check_public_source.py --history HEAD
python3 tools/check_distribution.py
cargo package --locked
```

Cargo package verification builds the archive; it does not replace tests from the
extracted crate. The [release checklist](docs/maintainers/releasing.md) gives those commands.
A local preparation pass can use `--allow-dirty`; the approved release must come
from a clean reviewed commit. Keep `Cargo.lock` and review dependency changes.

Normal tests use synthetic HTTP fixtures and never contact providers. The
[local corpus regressions](docs/development/testing.md#local-collected-data-checks)
run separately when the original observations are available. Ignored tests
include those regressions, live operations and offline replay; invoke only the specific test
needed. Do not run all ignored tests together. Small code changes need targeted
regressions plus the affected suite; final release checks cover the full package.

## Provider development

- [Testing and evidence](docs/development/testing.md): fixture ownership, scoped
  reviews, report semantics and focused offline checks.
- [Live collectors](docs/development/collectors.md): queue imports, bounded live
  work, retry experiments and offline replay.

Live calls require an explicitly authorized selection and bounded pacing. Keep
provider observations separate from reviewed expectations. The development guides
are repository-only; consumers use [reference documentation](docs/README.md#using-the-library).

## Adding a country adapter

Keep identifiers, provider decoding, transport and provider-specific tests in the
country module. Extend the typed `Provider`/`IdentifierScheme` enums and the
validated `BusinessId` mapping; do not accept arbitrary country strings as proof
that an adapter exists. Add named facade configuration and dispatch for the new
adapter, and map its native result into the shared company result without guessing
missing fields or conflating fiscal status with legal existence.

Register operation contracts in the capability catalog. Accepted schemes, date
parameters and local/remote execution must agree with the actual methods. Keep
source language behavior factual. Test both compiled discovery and filtering for
disabled adapters; no discovery call should initialize transport or contact sources.

Reuse the shared facade consumer tests for typed failures, disabled adapters and
capability contracts. Add provider-local synthetic tests for matched identity,
positive and definitive negative observations, service/access/schema failures,
source preservation and common-result conversion. Real observations and reviewed
expectations stay local-only, with ambiguity and provider errors reported separately.
Do not add a public plugin trait or another crate solely to register a new country.

## Documentation and evidence

Keep README focused on installation, behavior, limitations and navigation. Consumer
guides describe current contracts. Contributor/agent docs describe repeatable work;
CHANGELOG summarizes user-facing changes by release. Avoid chat references, dated
batch diaries, changing fixture counts, one-off latency claims, local machine paths
and hard-coded executable hashes in maintained guides.

Provenance dates, source hashes and versioned reviews belong with the evidence and
must remain intact. Keep raw observations and required audit journals separate from
regenerable replay/report output. Do not truncate append-only evidence or delete
unknown files under `target/`. Completed frozen executable copies can be removed.

Check free space and affected cache sizes before/after substantial runs. Keep at
least 50 GiB free and reuse the current Cargo cache. Coordinate before touching
another task's outputs or services. Remove disposable task artifacts after verification;
retain requested deliverables and required release/failure evidence.
