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
| Polish business extracts | [pl](pl/README.md) | Exact current KRS business-register identity; NIP/REGON remain distinct. |
| Slovak accounting entities | [sk](sk/README.md) | Unique IČO resolution and exact detail identity; native accounting fields retained. |
| Icelandic company pages | [is](is/README.md) | Identity-bound anonymous HTML; labelled legal address separate from postal address. |
| Irish company records | [ie](ie/README.md) | Anonymous CRO dataset with exact company number; includes dissolved records. |
| Lithuanian legal entities | [lt](lt/README.md) | Anonymous JAR dataset identity; names may be anonymized and entries may be removed. |
| Latvian registry dataset | [lv](lv/README.md) | Exact filtered official open-data record; legal address text preserved. |
| French public company data | [fr](fr/README.md) | Anonymous exact SIREN search; source headquarters and administrative codes retained. |
| Greek company registration | [gr](gr/README.md) | Anonymous GEMI details; AFM remains distinct from registration identity. |
| Czech economic subjects | [cz](cz/README.md) | Anonymous ARES lookup and native source registrations. |
| Romanian fiscal registration | [ro](ro/README.md) | ANAF CUI/date matching and domestic fiscal observations. |
| Anonymous registry transport | [registry_transport](registry_transport/mod.rs) | Private AT/BE/CZ/EE/FI/FR/GR/IE/IS/LT/LV/NO/PL/SK bounds, admission, cooldowns, HTTP failure classification and bounded JSON/HTML bodies; shared CKAN wire envelopes, with native decoding owned by providers. |
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
