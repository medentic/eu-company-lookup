# Implementing and reviewing registry adapters

This is the implementation guide for contributors and AI agents. Read the
[shared object and field meanings](../reference/company-result.md),
[compatibility policy](../reference/api-stability.md), and the affected adapter's
README before editing. The working shared interface is unreleased; existing
native APIs and migration requirements are documented separately.

## Architectural goal

An application calls `lookup_company(tax_id, country)` and reads a common company
object without country-specific branches for identity, addresses, lifecycle or
standard VAT registration. Only registry-specific interpretation requires matching
`registryDetails`. A source's missing facts remain unknown rather than being guessed.

Each provider owns identifier interpretation, transport, source decoding and its
pure projection into the shared result. The model owns field meanings and typed
contracts. The facade owns configuration and dispatch. Address utilities own
shared formatting, not legal or fiscal inference. Keep provider modules private;
no plugin trait, public endpoint override or new crate is needed to add a country.

Pure projections live in each implemented provider's `mapping.rs` module.
They consume matched native results, preserve source evidence and make no network
calls. `CompanyDetails` determines the common result's provider tag. Do not copy a
redundant mutable source label in a way that can disagree with the typed payload.
Normal decoder output and library conversion must agree; this does not make
publicly mutable result structs tamper-proof or general validation boundaries.

## Reuse mechanics without merging provider meanings

`registry_transport` owns admission, deadlines, cooldowns, operational HTTP errors
and bounded response bodies. Adapters explicitly choose accepted HTTP statuses
and media types. In particular, only ARES accepts a 404 body for its own typed
miss validation; the shared helper never decides that a company is absent.
Multi-request adapters retain completion-based spacing.

The private CKAN helper shares exact-filter URL encoding and the single-record
response envelope used by Ireland. Each adapter supplies its resource
and identifier field, then verifies the native identifier and required name and
maps fields locally. Sharing a wire protocol does not establish shared legal,
address, lifecycle or VAT meanings.

The facade keeps country routing explicit. Its common enabled-adapter check
preserves disabled-operation errors before identifier parsing. Test clients use
the empty builder so adding an adapter does not require repeating disabled fields
in every provider's fixture constructor.

## Decide where a field belongs

| Question | Decision |
| --- | --- |
| Does this represent a shared company concept with the same meaning and type across countries? | Use an existing common field. Propose a new common field only when its meaning, null behavior and compatibility impact are explicit. |
| Is the same concept unavailable from this source? | Keep the common key and use null. Do not rename it or move it into provider details. |
| Is it a native code, special tax regime, source structure, or date with provider-specific scope? | Keep it in typed `registryDetails.data`. |
| Can a source field safely map to a common fact? | Map it using an evidenced rule and retain useful native evidence in details. Do not let native fields override common fields during serialization. |
| Does the proposed field combine different observations or require another provider? | Treat that as an explicit new operation/design decision, not a hidden conversion or fallback. |

Availability is not the classification rule. A registered foreign-script name
is a common concept even when only BG currently supplies it. An ANAF registration
status string is provider-specific even though other sources also have status
strings. Shared `status` expresses the supported normalized lifecycle.

## Specify the adapter before implementation

Record the following in the owning provider README. Use official source
specifications and reviewed observations; distinguish implemented behavior from
planned support. Do not store actual collected records in the README.

| Decision | Required information |
| --- | --- |
| Identity | Accepted scheme, normalization, branch/personal-ID exclusions, leading-zero handling, exact response identity matching. Business code and VAT code must remain distinct. |
| Source | Register versus taxpayer service, official endpoint ownership, access requirements, language behavior, supported entity types and limitations. |
| Dates | Default date/timezone, explicit date operations, echoed-date matching, timestamp meaning; do not imply a historical company extract. |
| Positive result | Required response structure and minimum identity/name fields. Missing optional facts are allowed. |
| Negative result | Exact source evidence for a definitive miss. List operational failures separately and specify allowed fallback ordering. |
| Common mapping | For every common field: source field, transformation, null rule, source citation/evidence and regression test. Explicitly mark unsupported fields. |
| Lifecycle | Source-backed mappings to active/closed/unknown; source values retained. Tax inactivity is not closure. |
| VAT | Exact domestic registration scope and date; nullable boolean; whether and how an actual VAT identifier is supported. Document exceptions before deriving anything. |
| Addresses | Office versus fiscal roles; original full text, structured components, country evidence and shared formatting. |
| Links | Official company-specific destination and a verified construction rule; no generic homepage, API URL or credential/session-dependent URL. Document access limitations. Null remains valid. |
| Transport | Deadlines, connect timeout, response-size limits, pacing, concurrency, cooldown, redirects and error classification. Use current provider policies without silently adding retries. |

A reusable field-mapping row is:

| Shared field | Source field | Transformation | Unknown/negative behavior | Evidence | Regression |
| --- | --- | --- | --- | --- | --- |
| `status` | Provider status field | Map only reviewed lifecycle values | Missing -> null; present/unmapped -> unknown | Official specification and reviewed synthetic case | Known, missing and unmapped cases |

Do not infer VAT IDs by concatenating ISO country and national ID. VAT prefixes
can differ from ISO codes, fiscal registrations can have different identifiers,
and branches can use longer business codes. The RO derivation is deliberately
limited to the matched ANAF article-316 fiscal registration. Keep its original
requested tax ID unchanged. BG registries alone do not establish VAT registration;
the Bulgarian company adapter checks nine-digit candidates through its required
VIES dependency and retains that evidence in details. The facade only dispatches;
standalone VIES operation enablement must not gate BG verification. Follow the [BG enrichment contract](../reference/bulgarian-registry.md#shared-vat-fields)
for name matching, skipped branches, unknown results and propagated failures.

## Implement as one coherent change

1. Add the private provider's identifier, transport, decoder, typed native facts
   and mapping. Reuse shared components only where their semantics actually match.
2. Extend provider/scheme enums, named configuration, enabled-client construction,
   facade dispatch, and capability contracts together. Use
   `Client::SUPPORTED_COMPANY_COUNTRIES` as the common support list; an entry alone
   never implements an adapter. EEA metadata does not mean lookup support.
3. Map each shared field explicitly. Keep registered and fiscal addresses separate;
   never substitute one for the other. Apply shared country display formatting and
   retain original decoded full address text.
4. Add a typed details variant. Its serialized provider must equal top-level source;
   source-specific fields must remain under data. Do not flatten provider keys into
   the common object or replace details with an untyped map.
5. Update exports, provider mappings, consumer field documentation and migration
   guidance. Review any change to names, types, null semantics or existing behavior
   against the compatibility policy. Keep source defaults and coverage claims in
   their maintained references instead of copying them into every guide.

## Verification and acceptance

Default to offline tests. Provider-local tests protect decoding and projection;
external tests in `tests/` protect the facade, public types and capability contract.
Existing provider mapping tests assert the exact current common JSON keys and
source-detail shapes, so a new common field requires deliberate contract updates.

Cover matched positives; missing optional data; exact negatives; wrong identity or
date; malformed payloads; timeouts/service failures; unknown lifecycle values;
leading zeroes and branch suffixes; original address preservation; country casing;
VAT true/false/unknown and derivation exclusions; provider-tag agreement; and
allowed versus forbidden fallback. Include null keys explicitly. Test pure
conversion without live HTTP, and ensure disabled operations fail locally.

Run the relevant [contributor checks](../../CONTRIBUTING.md#setup-and-checks).
New consumer documentation must be explicitly included in Cargo and the
allowlist in `tools/check_distribution.py`. Agent/development guides stay outside
the archive. Run documentation and distribution checks after Markdown/package
changes; do not weaken their allowlist to make packaging pass.

Live checks require an authorized bounded selection and retained local provenance.
Small samples can confirm specific cases, not a universal derivation rule or a
completed country. Apply the existing 200 distinct VAT IDs and 200 distinct
company/tax IDs rule, reviewed source-bound expectations and offline replay for
country completion. Report negatives, provider failures and ambiguities separately.
See [testing and evidence](testing.md); preserve raw observations and review history.

Do not publish or tag a release without approval of the exact reviewed candidate.
Report implementation, local verification, live evidence, CI and publication as
separate states.

## Keyless delivery requirement

The default country lookup must work without registry API keys or user accounts.
Implement public registry pages or anonymous service endpoints first. Do not add a
credential requirement to complete a country's default adapter. If no suitable
keyless route is available, report the country-specific access blocker and continue
with other countries without claiming completion.

After the planned keyless country coverage is released, credential-based sources
may be considered as optional enhancements. Callers must explicitly supply any
credentials; the existing keyless lookup remains available. Such optional sources
require their own evidence-backed contracts and are outside the current expansion.
