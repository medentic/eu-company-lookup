# Public API compatibility

The 4.x contract covers exported Rust types and methods and documented JSON field
names, value types and null semantics. Provider data remains an observation that
can change between requests.

The [shared field reference](company-result.md) defines every common company
field, address component and provider extension. This document defines stability
and migration rules.

## Rust API

`VatLookupResult` contains VIES validity and available identity/address details.
`CompanyLookupResult` is the country-independent result of
`Client::lookup_company(value, country)`. Lower-level registry clients return
country-specific results such as `BgCompanyLookupResult` and `RoCompanyLookupResult`.
The crate root exports each implemented adapter's native types. `VatCheck` and
`CompanyRecord` remain aliases to `VatLookupResult` and `BgCompanyLookupResult`, respectively.

`BusinessId` validates country, scheme and value together, including Serde input.
It accepts the national schemes for `Client::SUPPORTED_COMPANY_COUNTRIES` and preserves leading zeroes. VAT
identifiers remain separate. The shared company view distinguishes registered
and fiscal addresses, retains native status and exposes typed provider details.
Converting a native result consumes it without fetching or merging other sources.

- Provider modules are private. The crate root explicitly exports identifiers,
  clients, configs, results, capabilities and errors; `locations` is public.
- Result/capability structs, configuration structs and extensible enums are
  `#[non_exhaustive]`. Read fields, destructure with `..` and include a fallback
  match arm. Minor releases may add fields or variants.
- Construct configs using `Default` and assign public fields. Construction validates
  bounds. Endpoint overrides and parser internals are not public APIs.
- `VatId`, `BgBusinessId`, `RoBusinessId`, `NoBusinessId`, `IeBusinessId` keep their contents private. Parsing,
  `TryFrom` and Serde input validate syntax. `AsRef<str>` borrows the normalized
  identifier; conversion to `String` consumes it. `BusinessId` additionally validates
  the country/scheme pair. Results support serialization, not general deserialization.
- Clients are cloneable, `Send + Sync`; lookup futures are `Send`. Clones share
  admission state. Debug output exposes configuration, not requests or provider bodies.
- `RetryAfter` is exhaustive: seconds or an absolute date. Error and provider enums
  are extensible. Public Chrono types are part of the Rust compatibility contract.

The minimum Rust version is 1.98.1. Increases require a documented minor release,
never a patch release. Network lookups require Tokio with I/O and time enabled.
CI covers Linux; no broader platform verification is claimed. See
[Cargo's SemVer guidance](https://doc.rust-lang.org/cargo/reference/semver.html).

## Migrating from 3.x

Version 4.0 replaces the public `LookupError` enum and removes `RegistryError`.
Every public operation returns the common `LookupError` struct, with stable
`ErrorId`, English title/description, an availability flag and structured details.
See the [complete error catalog, raw diagnostics and migration](errors.md).
Do not match removed enum variants or parse human-readable error messages.

## Migrating from 2.x

Version 3.0 introduces a breaking Rust API and shared JSON contract change.
Replace `lookup_bg_company(value)` with `lookup_company(value, "BG")` and
`lookup_ro_company(value)` with `lookup_company(value, "RO")`. Replace
`lookup_company(&id)` with `lookup_company(id.as_str(), id.country())`.
All return `CompanyLookupResult`; migrate native field access to `country`, `tax_id`,
`registered_office`, `fiscal_address` and typed `registry_details`.
Replace the shared result's nested `identifier` with top-level `country` and
`taxId` (`tax_id` in Rust). Identifiers remain strings to preserve leading zeroes.
Move shared `nativeRegistryStatus` into `registryDetails.data.nativeRegistryStatus`;
normalized `status` remains top-level. `CompanyDetails` variants now use named
`native_registry_status` and `details` fields; match with `..` for extensibility.
The shared result now exposes nullable `registryUrl` and `foreignName` at the
top level. Rename Rust `details` to `registry_details` and JSON `details` to
`registryDetails`. Replace `CompanyDetails::BulgarianRegistry` with direct
`BulgarianCommercialRegister` or `Bulstat` variants. Their `data` contains only
registry-specific fields; `provider` agrees with top-level `source`. RO retains
its `RomanianAnaf` variant and returns null for both new common fields.

The country argument accepts entries in `Client::SUPPORTED_COMPANY_COUNTRIES`
case-insensitively after trimming surrounding whitespace. Unsupported countries return `UnsupportedIdentifierScheme` locally.
For a supported country, disabled adapters return `OperationDisabled` before
identifier validation; malformed codes return `InvalidInput` when enabled.
VAT-prefixed IDs remain invalid for company lookup.

The country-specific facade methods, including `lookup_ro_company_at`, are
removed. Explicit fiscal dates remain available through
`RoRegistryClient::lookup_at`; native results can be converted with `Into`.
Provider capabilities still describe the lower-level native and dated operations
as well as the shared facade operation.

## Migrating from 1.x

Existing VAT methods and constructors remain available. For the 3.0
company facade migration, see the migration from 2.x above.
Existing `VatCheck` and `CompanyRecord` annotations continue to compile. New code
can use the preferred result names above. The shared company result is a new
contract; opt into it with `lookup_company` or by converting a native result.

The following changes require consumer review before upgrading to 2.0:

| Surface | 1.x | 2.x and migration |
| --- | --- | --- |
| `CompanyStatus` | `Other(String)` could carry source text. | Match `Unknown` and read `native_registry_status` for source text/codes. |
| Unknown Commercial Register status | JSON null. | JSON `unknown`; `nativeRegistryStatus` and `registryDetails.statusCode` retain the code. |
| BULSTAT active status | Literal `развиващ дейност`. | JSON `active`; native text remains in `nativeRegistryStatus` and `registryDetails.statusText`. |
| `JurisdictionCapabilities.vat_prefix` | Always a string. | `Option<&str>`; JSON `vat_prefix` may be null. Inspect enabled providers before offering a lookup. |
| `ProviderCapabilities.operations` and `identifier_scheme` | Flat operation enum list and one provider scheme. | Typed operation contracts with `identifierSchemes`, `execution` and `dateInput`; use `provider.supports(operation, scheme)`. |
| `LanguageSupport` | Enum. | Struct with `available`, `default`, `selectable`; migrate enum matches and capability JSON consumers. |

Capabilities include EEA metadata entries with empty provider lists. An entry does
not establish an implemented registry, current availability, or address coverage.
VAT jurisdiction mappings remain EL → GR and XI → GB / GB-NIR. No general GB
VAT coverage is added. New optional result fields must be tolerated by consumers.

## Facade and operation contracts in 2.0

`Client` is independent of the VIES adapter. Existing constructors retain their
behavior; `Client::builder()` adds named configuration with all current adapters
enabled by default, while `ClientBuilder::empty()` starts without HTTP clients.
Only configured adapters are constructed. Clones share adapter admission state.

The free `capabilities()` returns compiled support without client construction.
`client.capabilities()` filters providers to those enabled on that instance.
Provider `operations` changes from enum values to `OperationCapabilities` objects:
`operation`, `identifierSchemes`, `execution` (`Local`/`Remote`) and `dateInput`
(`None`/`FiscalDate`). The provider-level `identifier_scheme` field is replaced by
per-operation scheme lists. Migrate checks to `provider.supports(operation, scheme)`.
Existing source labels and address output are unchanged by this refactor.

A disabled adapter reports `OperationDisabled { operation, identifier_scheme }`
before validating input or contacting a source. This differs from a provider outage
or a definitive miss. `BusinessId` reports `UnsupportedIdentifierScheme` for an
unsupported country/scheme pair; supported pairs with malformed values retain
`InvalidInput`. Both errors remain distinct from registration results.

## Company status and source values

Every company result uses `status: Option<CompanyStatus>` with `Active`, `Closed`,
or `Unknown` (JSON `active`, `closed`, `unknown`). Missing provider status remains
null; present but unmapped values become `Unknown`. Adapters must establish
source-backed mappings rather than infer lifecycle from tax flags.

In the shared result, `registryDetails.data.nativeRegistryStatus` is a nullable
string retaining the source status even when
normalization succeeds. BULSTAT and ANAF retain decoded text; the Commercial
Register supplies numeric `deedStatus`, represented as a decimal string without
an invented label. The native results also retain BULSTAT
`registryDetails.statusText`, Commercial Register `registryDetails.statusCode`,
and ANAF `registrationStatus`. In the shared result, source-specific fields live
under typed `registryDetails`.

## JSON and business meaning

The top-level shared company object contains only provider-independent concepts
with consistent names, types and meanings. Provider-specific status text, codes,
special tax regimes and source structures belong in `registryDetails`. Provider details
may extend the observation but cannot override common fields. A shared field can
be null when unavailable; availability alone does not make it provider-specific.

Within 2.x, documented field names, types, null behavior and provider spellings
are stable. Minor releases can add fields and extensible enum values; consumers
should tolerate unknown fields and values. Missing information stays null rather
than becoming an invented fact or an assertion of absence.

VAT and company results are separate contracts. `valid: false` is a matched
negative VIES observation. `not_found` with reason `both_bulgarian_registers_missed` requires definitive misses from both
Bulgarian registers. `not_found` with provider `RomanianAnaf` is an
ANAF-specific miss. Operational failures never become negative observations.

For native Bulgarian results, `taxId` is a business code, `companyEnglishName` is
the registered foreign-script name, and `identifierEntryDate` is the UIC field
entry date. These do not imply VAT registration, translation or incorporation.
`source` discriminates the native untagged `registryDetails` payload.

The shared view uses top-level `country` and string `taxId`, and `registryDetails` has
an explicit `provider` tag and `data` payload. Registered and fiscal addresses
remain separate. ANAF domestic VAT flags are nullable source observations, not
VIES validity. Its requested fiscal date and UTC observation time remain distinct.
The lower-level registry clients preserve their separate field layouts, with the
status changes and added fields described above.

## Shared VAT fields

The shared result exposes nullable `vatRegistered` and `vatId` (`Option<bool>`
and `Option<VatId>` in Rust). The boolean describes registration supported by provider-specific evidence,
not all special VAT schemes. RO uses ANAF; BG may use supplementary VIES evidence. Null
means unavailable, not false. `vatId` is an identified VAT number, not proof of a
successful VIES check; null does not prove no VAT number exists.

RO maps the matched ANAF article-316 flag at its requested fiscal date. Only true
permits deriving RO plus the numeric matched fiscal identifier, with input padding
removed and VAT syntax checked. `taxId` itself remains unchanged. False or unknown
flags leave `vatId` null. The original ANAF flag remains in source details.
BG company lookup now delegates to the Bulgarian adapter, which automatically
checks VIES after every successful nine-digit registry lookup. Only a valid, name-matched VIES
response fills the common VAT fields. Negative validity, missing names and name
mismatches leave them null. Both BG shared details variants add nullable
`vatVerification` with the full VIES observation and its own provider source.
Thirteen-digit branch IDs are skipped. Native BG clients/conversions remain
registry-only. VIES failures propagate from the combined lookup; total latency can
include both providers' deadlines. `without_vies()` disables standalone VAT
operations only; the BG adapter retains its required VIES dependency. There is no
shared BG lookup enrichment opt-out. When standalone VIES is enabled, both paths
share its client/admission limits; otherwise BG uses default VIES settings.
This changes network behavior and error outcomes from the 2.x facade;
see [the BG contract](bulgarian-registry.md#shared-vat-fields). New providers must
supply their own evidence-backed mapping, not a universal prefix rule.

## Address policy

Address parsing is conservative and independent of validity. Parser corrections
may change structured components in patch releases while retaining original
provider address text. VIES preserves decoded XML text; registry adapters preserve
decoded visible HTML text, not the original HTML markup.

Shared Bulgarian presentation rules operate on a working copy: normalize recognized
marker spacing, remove double quotation marks, use `жк.`, uppercase single entrance
letters, and title-case all-uppercase place names while retaining abbreviations
and Roman numerals. Apostrophes and meaningful mixed-case names are preserved.
Recognized empty or asterisk-only delivery components and explicit post-box
components are omitted. Short-number `П.К` delivery components are treated as
post boxes; four-digit postcode-like values are retained. Unlabelled numbers
are not guessed into telephone, entrance, floor or apartment fields.

A `locations` country hint chooses extraction rules; it is not evidence of physical
country. VIES country metadata follows VAT jurisdiction. Registry country metadata
requires explicit recognized source country text. BG uses the shared native name;
ANAF retains the supplied name's language with normalized display casing.
Unsupported formats retain unknown components. Provider envelopes preserve source
text so applications can inspect what could not be structured.

All structured address country names use title case, retaining language and
diacritics. `countryISO2` uses uppercase ISO codes, or null when unknown. This
also applies to native ANAF addresses and their copies in `registryDetails`;
original full address strings remain unchanged.
