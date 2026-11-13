# Maintaining the package with an AI agent

Give a new developer or agent the repository checkout and a concrete task. No
previous chat history is required to understand the intended contracts. Read
[AGENTS.md](../../AGENTS.md), [CONTRIBUTING.md](../../CONTRIBUTING.md), the
[source architecture](../../src/README.md) and the relevant provider README first.
The [API contract](../reference/api-stability.md) governs consumer changes;
[release preparation](releasing.md) governs publication.

## Start a maintenance session

1. Inspect `git status --short` and the relevant diff. Preserve existing work and
   coordinate overlapping edits. Read scoped `AGENTS.md` files before editing.
2. Establish the working version from `Cargo.toml`, release intent from CHANGELOG,
   and actual behavior from code/tests. A version string or a local successful
   build does not prove publication, CI completion or country verification.
3. Trace the affected operation from facade to identifier, transport, decoder,
   public result and serialization. Use the module map below to find ownership.
4. Reproduce locally with a synthetic fixture or available reviewed offline replay.
   If original evidence is unavailable, report that limitation. Do not silently
   replace it with generated expectations or make unapproved live requests.
5. Make the smallest supported change, update explanatory comments and affected
   mappings, and run checks appropriate to the behavior changed.

## Find the owning domain

| Task | Start here | Important contract checks |
| --- | --- | --- |
| Enable/configure an adapter or change discovery | [Facade and catalog](../../src/README.md) | `tests/client_facade.rs`, `tests/configuration.rs` |
| Change identity, JSON, lifecycle or common result | [Model map](../../src/model/README.md) | `tests/shared_company.rs`, `tests/result_names.rs`, `tests/public_api.rs` |
| VIES validity or SOAP failure | [VIES map](../../src/vies/README.md) | `vies::tests` |
| Bulgarian source/fallback behavior | [BG field map](../../src/bg/README.md) | `bg::tests` |
| Romanian taxpayer/date behavior | [ANAF field map](../../src/ro/README.md) | `ro::tests`, `tests/romanian.rs` |
| Address extraction/presentation | [Location map](../../src/locations/README.md) | `locations::`, `vies::address::tests`, affected provider tests |
| Collection, replay or evidence | [Tools guide](../../tools/README.md) | Python collector tests and the specifically documented offline replay |

For example, `cargo test --locked --lib bg::tests` selects the BG unit suite;
`cargo test --locked --test shared_company` selects the external common-result
contract. Do not append `--ignored` broadly: ignored tests include live operations.
Use the full [contributor suite](../../CONTRIBUTING.md#setup-and-checks) for release
preparation and broad behavior changes. Markdown changes require `check_docs.py`
and `check_distribution.py`. Check disk space before/after substantial runs.

## Diagnose failures without inventing negative results

| Observation | Investigation |
| --- | --- |
| Disabled/unsupported/invalid input | Inspect facade configuration, country/scheme pairing and identifier syntax; no source lookup is implied. |
| Busy, timeout, cooldown, throttling or service failure | Inspect admission and transport classification. Preserve retry metadata; absence is unproven. |
| Identity/date mismatch | Verify echoed identifiers/date and fixture provenance; never accept the first returned company. |
| Malformed or unexpected response | Compare schema/content type with a retained authorized observation; add a synthetic failing case before changing decoding. |
| Missing or surprising address fields | Inspect original text and the selected parser; unknown components are allowed. |
| Definitive negative result | Verify the provider-specific negative evidence, including both misses for BG. Do not merge it with operational failures. |

A source change needs a reviewed explanation of what changed and why the new
interpretation follows from that source. Keep raw observations, source hashes,
review revisions and journals intact. Public synthetic regressions must not copy
collected identifiers or records. The [testing guide](../development/testing.md)
defines evidence review; the [collector guide](../development/collectors.md)
defines bounded live work. Counts alone never establish country completion.

## Extend coverage coherently

Follow [Adding a country adapter](../../CONTRIBUTING.md#adding-a-country-adapter).
Specify identifier scheme, provider, supported operations, language behavior,
dates, definitive negative evidence, lifecycle mapping and address provenance
before implementing them. Review public contract changes with the maintainer.
Update facade dispatch/configuration, catalog, exports, typed results, native-to-common
mapping, synthetic tests and documentation together. Unknown source values must
remain unknown until a mapping is supported by evidence.

Keep a new adapter's README beside its implementation. Document endpoint ownership,
source field names, identity checks, negative-result rules and the tests protecting
them. Add focused comments explaining non-obvious evidence or ordering constraints;
avoid comments that merely repeat the code. Do not duplicate country coverage counts
or provider defaults across guides; link to their maintained reference instead.

## Leave a reviewable handoff

Report changed behavior/files, checks actually run and their results, unavailable
checks, evidence limitations and remaining decisions. Distinguish pre-existing
changes from your own. Keep run-specific identifiers and paths out of maintained
public documentation. A handoff should allow the next developer to reproduce the
checks from a fresh checkout; local corpora are optional additional evidence,
not a hidden prerequisite for the normal test suite.

Do not publish or tag until the exact reviewed candidate is approved. Agent files,
module READMEs, collector tools and maintainer guides stay repository-only. Cargo's
explicit include list and the distribution checker protect that boundary.
