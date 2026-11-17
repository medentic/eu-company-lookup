# Identity and result contracts

[mod.rs](mod.rs) gathers types; [../lib.rs](../lib.rs) controls their public exports.
[API stability](../../docs/reference/api-stability.md) owns compatibility policy.

| File | Responsibility |
| --- | --- |
| [business.rs](business.rs) | Validated country/scheme/value identity, common company result and typed provider detail contracts. |
| [vat.rs](vat.rs) | VIES observation and original-address envelope; independent of company lookup. |
| [company.rs](company.rs) | BG result, original registry address, Commercial Register/BULSTAT details. |
| [company_types.rs](company_types.rs) | Shared lifecycle and BG legal forms, registry roles and document categories. |
| [capabilities.rs](capabilities.rs) | Provider, identifier and operation contracts; catalog values live outside the model. |
| [error.rs](error.rs), [registry_error.rs](registry_error.rs) | Caller-visible failure classification and source-specific failures. |

Provider projections live in [BG mapping](../bg/mapping.rs) and
[ANAF mapping](../ro/mapping.rs), alongside the source rules they interpret.
The consumer [field reference](../../docs/reference/company-result.md) defines
the object; the [adapter guide](../../docs/development/registry-adapters.md)
defines how future registries must map into it.

## Shared-model boundary

Top-level fields describe provider-independent company concepts with common
names, types and meanings. A value may be unavailable for a provider, but its
meaning must not change by provider. Native codes, source status text, provider-specific fiscal
flags and provider-specific structures belong in `registryDetails`; they never
override common fields. Keep normalized `status` at the top level and original
`nativeRegistryStatus` inside `registryDetails.data`. Apply this boundary to
future adapters and fields.

## Mapping into the common company result

`BusinessId` accepts the implemented national country/scheme pairs advertised
by the capability catalog. Deserialization uses the same validation. Never derive it by stripping a VAT prefix. Leading zeroes
and BG branch suffixes remain in the public identity.

| Common field | Bulgarian source | Romanian source |
| --- | --- | --- |
| `country` / `taxId` / `source` | BG / requested matched business code / actual successful register | RO / requested CUI spelling / ANAF |
| `vatRegistered` / `vatId` | True / checked VAT ID only after valid, name-matched VIES enrichment; otherwise null | ANAF article-316 flag / RO plus matched numeric fiscal ID only when true |
| `companyName` / `legalForm` | Native name / recognized abbreviation or source fallback | ANAF name / supplied legal-form text |
| `registeredOffice` | BG registry address envelope | `adresa_sediu_social` components |
| `fiscalAddress` | Null; no separate fiscal address is supplied | `adresa_domiciliu_fiscal` components and original `date_generale.adresa` |
| `status` | Normalized lifecycle | Unknown for present unmapped status, null if absent |
| `registryUrl` / `foreignName` | Record link / separately registered foreign-script name | Null / null |
| `registryDetails` | Native status plus typed Commercial Register or BULSTAT facts | Native status, fiscal date, observation time, source addresses and tax flags |

The RO conversion joins supplied street and number for the common delivery line;
the separate components remain in `AnafDetails`. It does not parse raw fiscal text
to fill missing structured values or substitute it for the registered office.

`None` means no supplied value; `CompanyStatus::Unknown` means a source status
exists but has no verified mapping. Domestic tax inactivity is not company closure.
BG `RegistryDetails` serializes untagged and relies on the result's `source`;
common `CompanyDetails` has its own `provider`/`data` tag. Preserve both contracts.

Review [shared company tests](../../tests/shared_company.rs),
[result names](../../tests/result_names.rs) and [public API tests](../../tests/public_api.rs)
when changing these mappings. Optional fields, Serde names and null behavior are
consumer API, even when a change looks like internal cleanup.

Address country names use shared title-case display normalization, preserving
language and diacritics. `countryISO2` is uppercase when recognized; missing or
unrecognized countries retain a null code. BG and RO registry address adapters
apply their documented jurisdiction fallback when the country is missing. Original full address text is unchanged.

`vatRegistered` is nullable and does not cover every special tax regime. Its
evidence is provider-specific: ANAF for RO, supplementary VIES for BG. `vatId` is not a generic prefix plus business code:
RO derives it only from a matched positive ANAF article-316 registration, without
caller-supplied numeric padding. BG checks nine-digit candidates but populates fields only from valid, name-matched
VIES responses retained in details; branch IDs are skipped.
Keep the original source VAT flag in ANAF details for inspection.
