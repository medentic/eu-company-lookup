# Identity and result contracts

[mod.rs](mod.rs) gathers types; [../lib.rs](../lib.rs) controls their public exports.
[API stability](../../docs/reference/api-stability.md) owns compatibility policy.

| File | Responsibility |
| --- | --- |
| [business.rs](business.rs) | Validated country/scheme/value identity, common company result, native-to-common conversions. |
| [vat.rs](vat.rs) | VIES observation and original-address envelope; independent of company lookup. |
| [company.rs](company.rs) | BG result, original registry address, Commercial Register/BULSTAT details. |
| [company_types.rs](company_types.rs) | Shared lifecycle and BG legal forms, registry roles and document categories. |
| [capabilities.rs](capabilities.rs) | Provider, identifier and operation contracts; catalog values live outside the model. |
| [error.rs](error.rs), [registry_error.rs](registry_error.rs) | Caller-visible failure classification and source-specific failures. |

## Mapping into the common company result

`BusinessId` accepts BG + `BgBusinessId` and RO + `RoBusinessId`. Deserialization
uses the same validation. Never derive it by stripping a VAT prefix. Leading zeroes
and BG branch suffixes remain in the public identity.

| Common field | Bulgarian source | Romanian source |
| --- | --- | --- |
| `identifier` / `source` | Requested matched business code / actual successful register | Requested CUI spelling / ANAF |
| `companyName` / `legalForm` | Native name / recognized abbreviation or source fallback | ANAF name / supplied legal-form text |
| `registeredOffice` | BG registry address envelope | `adresa_sediu_social` components |
| `fiscalAddress` | Null; no separate fiscal address is supplied | `adresa_domiciliu_fiscal` components and original `date_generale.adresa` |
| `status` / `nativeRegistryStatus` | Normalized lifecycle / original status text or decimal code | Unknown for present unmapped status, null if absent / original registration status |
| `details` | Record URL, foreign-script name and typed registry details | Fiscal date, observation time, source addresses and tax flags |

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
