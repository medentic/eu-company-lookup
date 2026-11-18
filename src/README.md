# Source architecture

This is one crate with private provider adapters and an explicit public surface in
[lib.rs](lib.rs). Read the [API contract](../docs/reference/api-stability.md) before
changing exported types or serialized fields. These module guides describe the
working source; they do not establish a published version or live coverage.

## Domain ownership

| Domain | Owner | Boundary |
| --- | --- | --- |
| Client construction and dispatch | [client.rs](client.rs) | Enabled adapters, named configuration and local disabled-operation errors. |
| Support discovery | [capabilities.rs](capabilities.rs) | Compiled support; the client filters it to enabled providers without network I/O. |
| Identity and result contracts | [model](model/README.md) | National identity, common company view, source details, errors and JSON. |
| VAT verification | [vies](vies/README.md) | VAT syntax, SOAP exchange, matched validity and VAT jurisdiction. |
| Bulgarian business registration | [bg](bg/README.md) | Commercial Register first, then BULSTAT only after a definitive miss. |
| Austrian company registration | [at](at/README.md) | Anonymous Firmenbuch active-record search and exact detail identity. |
| Belgian company identity | [be](be/README.md) | Anonymous NBB Consult with CBE-derived identity; no CBE absence inference. |
| Finnish company information | [fi](fi/README.md) | Anonymous PRH YTJ exact business-ID lookup; native contact-address and tax-register roles retained. |
| Estonian public registry | [ee](ee/README.md) | Exact registry-code match in anonymous RIK portal search. |
| Norwegian entity information | [no](no/README.md) | Anonymous Enhetsregisteret entities; domestic VAT and address roles remain source-specific. |
| Polish P/S extracts | [pl](pl/README.md) | Exact current KRS business/association register identity; NIP/REGON remain distinct. |
| Slovak accounting entities | [sk](sk/README.md) | Unique IČO resolution and exact detail identity; RPO lifecycle supplement follows a matched accounting record. |
| Icelandic company pages | [is](is/README.md) | Identity-bound anonymous HTML; labelled legal address separate from postal address. |
| Irish company records | [ie](ie/README.md) | Anonymous CRO dataset with exact company number; includes dissolved records. |
| Lithuanian legal entities | [lt](lt/README.md) | Anonymous JAR dataset identity; names may be anonymized and entries may be removed. |
| Latvian registry dataset | [lv](lv/README.md) | Exact filtered official record plus liquidation, suspension and insolvency datasets; legal address text preserved. |
| French public company data | [fr](fr/README.md) | Anonymous exact SIREN search; source headquarters and administrative codes retained. |
| Greek company registration | [gr](gr/README.md) | Anonymous GEMI details; AFM remains distinct from registration identity. |
| Czech economic subjects | [cz](cz/README.md) | Anonymous ARES lookup and native source registrations. |
| Romanian fiscal registration | [ro](ro/README.md) | ANAF CUI/date matching and domestic fiscal observations. |
| Anonymous registry transport | [registry_transport](registry_transport/mod.rs) | Private AT/BE/CZ/EE/FI/FR/GR/IE/IS/LT/LV/NO/PL/RO/SK bounds, admission, cooldowns, HTTP failure classification and bounded JSON/HTML bodies; shared CKAN wire envelopes, with native decoding owned by providers. |
| Address interpretation | [locations](locations/README.md) | Offline extraction and country metadata; no registration decisions. |

## Request flow

`Client::SUPPORTED_COMPANY_COUNTRIES` is the shared ISO alpha-2 support list for
company lookup, business identity validation and national capability discovery.
Use it when listing supported company countries. Country-specific dispatch,
scheme mappings and source parsing remain explicit; adding a country requires
its adapter as well as a support-list entry.

A facade method checks whether its adapter is enabled, then the adapter validates
input, applies admission/deadline policy, exchanges bounded responses and checks
source identity before producing a result. `lookup_company` takes a national code and country string, selects
the country adapter, validates the code and converts its native result into a common view.
Provider-local `mapping.rs` modules own these pure projections. Conversion
performs no extra lookup. The facade delegates BG company lookup to its private country
orchestrator, which owns a required VIES dependency and retains supplementary
evidence in BG details. Standalone VIES enablement does not gate that dependency. The
shared model defines field contracts; typed details determine the source tag.

Country metadata, VAT jurisdiction, national identifier scheme, provider and
physical address country are different concepts. For example, EL is the VIES
prefix for GR; XI represents GB-NIR, not general GB VAT support. A country in the
capability catalog with no providers is metadata, not an implemented lookup.

## Where a change belongs

Change a source decoder for changed provider fields; change `locations` for a
shared address rule; change `model` for a consumer contract; change the facade and
catalog together for adapter configuration or operation availability. Provider
parsers and test endpoints remain private. External consumer contracts are tested
in [tests](../tests); provider unit tests remain beside the implementation.

For the maintenance workflow and verification matrix, see the
[agent handoff guide](../docs/maintainers/agent-handoff.md).

[HTTP capture](http_capture.rs) supplies request-scoped raw diagnostics to the common
[error contract](../docs/reference/errors.md). Capture defaults off and does not
replace provider admission, response interpretation or definitive-miss policy.

Private [test support](test_support.rs) checks common result invariants across
adapters. Its HTTP helpers read complete bounded requests and abort owned server
tasks on drop; provider response fixtures and assertions remain local.

National offline probes use `test_support::company_validation` to distinguish
parser agreement from resolved company validation. Reproducing an unknown/null
lifecycle never qualifies as a positive or negative validation case. Per-case
outcomes and separate summaries retain this distinction without changing source
observations; see [validation outcomes](../docs/development/testing.md#registry-validation-outcomes).

All native company clients enforce the shared lifecycle success policy inside
HTTP diagnostic capture; mappings remain provider-owned. Unresolved data does
not trigger fallback or outage cooldown.

## Registry pacing and evidence

Country READMEs describe the implemented route, not every service offered by the
registry. Anonymous access, a successful HTTP response and a resolved company
lifecycle are separate facts. Access to a public portal does not imply access to
its credentialed API, and a portal failure does not establish an API outage.

For adapters using `registry_transport`, defaults are a 30-second total lookup
deadline, 3-second connect timeout, 2-second interval, 60-second failure cooldown
and 1 MiB limit per response. These are library policies, not provider quotas.
The configurable interval must be at least one second. One lookup runs at a time
per client and its clones; a concurrent call returns `busy`. Separate instances,
processes and other applications sharing an IP do not share this admission state.

Single-request adapters measure spacing from the preceding lookup start. AT, CZ, LV, PL
and SK also wait the interval between HTTP exchanges and retain a gap after the
lookup finishes or is cancelled. Their total deadline includes pacing and all
supplemental requests; increasing the interval may require a larger total timeout.
BG has its own transport and defaults, documented in its country guide. VIES has
separate admission rules, including when used by the BG orchestrator.

HTTP 429 is `rate_limited`; HTTP 401/403 is `access_denied`. Service failures and
rate limits establish a local cooldown of at least the configured duration,
extended by a longer valid `Retry-After`. No automatic retry occurs. Malformed
responses, identity mismatches and unresolved lifecycle do not establish company
absence; unresolved lifecycle does not start an outage cooldown. Reuse clients,
coordinate callers and respect retry guidance instead of creating fresh clients
to evade the cooldown. A working pacing configuration is not a guaranteed quota.

Keep three evidence categories separate: published provider limits (link their
source), observed failures (retain endpoint/status/headers locally), and chosen
client policy (verify against code). If no numerical quota has been established,
say so rather than infer one from a successful batch. Recovery keeps original
journals and reviews; replay must account for every originally selected identity.
A complete observation batch can still contain unresolved data. Neither parser
agreement nor a synthetic negative case establishes live country coverage.
