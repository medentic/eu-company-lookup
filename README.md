# eu-company-lookup

An asynchronous Rust library for **EU VAT verification through VIES** and
**Bulgarian business lookup through the Commercial Register and BULSTAT**,
and **Romanian ANAF taxpayer lookup**.
It returns typed results, source-provided company details, structured addresses
and explicit errors. No database, service account or full-registry download is required.

Version 2.0 provides a shared company lookup for Bulgaria and Romania while
keeping VAT verification and provider-specific results separate. See the
[1.x migration notes](docs/reference/api-stability.md#migrating-from-1x) before upgrading.

## What it does

| Operation | Result |
| --- | --- |
| `validate_vat_syntax(value)` / `VatId::parse(value)` | Offline VAT format validation and normalization. |
| `client.lookup_vat(value)` | VIES validity, checked identifier, company name and address when supplied. |
| `client.is_vat_valid(value)` | The same VIES lookup, returning only its validity flag. |
| `validate_bg_business_id_syntax(value)` / `BgBusinessId::parse(value)` | Offline nine- or thirteen-digit Bulgarian business-code validation. |
| `client.lookup_company(&identifier)` | Common BG/RO company view from a validated `BusinessId`; preserves typed source details. |
| `client.lookup_bg_company(value)` | Exact Commercial Register match, with BULSTAT fallback only after a definitive miss. |
| `validate_ro_business_id_syntax(value)` / `RoBusinessId::parse(value)` | Offline Romanian CUI syntax validation. |
| `client.lookup_ro_company(value)` | ANAF taxpayer details and dated domestic tax status. |
| `capabilities()` / `client.capabilities()` | Compiled support or enabled providers, with typed operation inputs and language behavior. |
| `locations::parse_address(text, country_hint)` | Offline, conservative address extraction. |

Syntax acceptance does **not** prove checksum validity or registration. A company
record does not establish VAT registration. Address extraction is heuristic;
original provider address text is retained and uncertain components remain null.

Beyond these providers, the library does not implement other national registries, D-U-N-S, checksum
validation, verification reports, bulk-download APIs, automatic retries or caching.
Bulgarian adapters use observed public portal interfaces, which can change.

## Install

Requires **Rust 1.98.1+** and **Tokio with I/O and time enabled**.
Add the following dependencies:

```toml
[dependencies]
eu-company-lookup = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

For source builds, use the Git repository and pin a reviewed commit with `rev`.
Release availability is listed on [crates.io](https://crates.io/crates/eu-company-lookup).

## Configure only the adapters you need

```no_run
use eu_company_lookup::{capabilities, ClientBuilder, ClientConfig, LookupError};

fn setup() -> Result<(), LookupError> {
    let supported = capabilities(); // No HTTP clients or network requests.
    let client = ClientBuilder::empty()
        .vies(ClientConfig::default())
        .romanian_registry(Default::default())
        .build()?;
    let enabled = client.capabilities();
    assert!(enabled.len() == supported.len()); // Same country metadata; enabled providers differ.
    Ok(())
}
```

`Client::builder()` enables the current adapters by default; named `.vies(...)`,
`.bulgarian_registry(...)` and `.romanian_registry(...)` methods configure them.
`ClientBuilder::empty()` opts into only the adapters you select. The existing
constructors remain available. BG configuration includes both Commercial Register
and its definitive-miss-only BULSTAT fallback.

The free `capabilities()` reports compiled support. `client.capabilities()` reports
adapters enabled on that instance. Each provider's `operations` contains typed
contracts with `operation`, `identifierSchemes`, `execution` and `dateInput`.
Use `provider.supports(operation, scheme)` to check a particular pairing. An
explicit ANAF fiscal date is available through `LookupRoCompanyAt`; this does not
claim historical company-record support.

A disabled adapter returns `OperationDisabled` locally, before parsing the lookup
input or sending requests. Unsupported national country/scheme pairs return
`UnsupportedIdentifierScheme`; malformed values under supported schemes return
`InvalidInput`. Neither error is a negative registration observation.

## Look up VAT details

```no_run
use eu_company_lookup::{Client, LookupError, validate_vat_syntax};

#[tokio::main]
async fn main() -> Result<(), LookupError> {
    // Synthetic example: format acceptance is not a registration claim.
    let vat = validate_vat_syntax("bg 123456789")?;
    let client = Client::new()?;
    let result = client.lookup_vat(vat.as_str()).await?;
    println!("{}: valid={}", result.vat_id, result.valid);
    println!("Company: {:?}; address: {:?}", result.company, result.address);
    Ok(())
}
```

`lookup_vat` returns a `VatLookupResult` containing validity and available company
identity/address details. `lookup_bg_company` returns a `BgCompanyLookupResult`
containing registered identity and source-specific registry details. The original
`VatCheck` and `CompanyRecord` names remain compatibility aliases to `VatLookupResult` and
`BgCompanyLookupResult`, respectively.

Reuse and clone one client across application requests. Calling both `lookup_vat`
and `is_vat_valid` performs two requests; use the returned `valid` field when you
already have the full result.

VIES prefixes: **AT BE BG CY CZ DE DK EE EL ES FI FR HR HU IE IT LT LU LV MT NL PL PT RO SE SI SK XI**.
Greece uses **EL**, mapped to ISO country **GR**. **XI** maps to **GB / GB-NIR** for
Northern Ireland; general GB coverage is not included. OSS/IOSS prefixes **EU** and
**IM** are unsupported. VIES has no language-selection parameter.

See the [VIES guide](https://github.com/medentic/eu-company-lookup/blob/main/docs/reference/vies.md) for input rules, JSON output and address semantics.

## Look up a company across supported countries

`BusinessId` combines country, national identifier
scheme and validated value. It does not derive a company ID by stripping a VAT
prefix, and a Bulgarian business code alone does not distinguish ЕИК from BULSTAT.

```no_run
use eu_company_lookup::{BusinessId, Client, IdentifierScheme, LookupError};

async fn lookup() -> Result<(), LookupError> {
    let client = Client::new()?;
    let id = BusinessId::parse("RO", IdentifierScheme::RoBusinessId, "001234")?;
    let company = client.lookup_company(&id).await?;
    println!("{}: {}", company.identifier.country(), company.company_name);
    Ok(())
}
```

`CompanyLookupResult` provides `identifier`, `source`, `companyName`, `legalForm`,
`registeredOffice`, `fiscalAddress`, `status`, `nativeRegistryStatus` and typed
`details`. Original source address text and source-specific facts remain available.
ANAF structured address components are mapped directly; missing fields stay null.
Its tax flags never substitute for a VIES check. The country-specific lookup
methods retain their source-specific JSON output, and their results convert into
the common view using `Into<CompanyLookupResult>`.

Capabilities enumerate EEA countries plus the existing Northern Ireland scope.
An empty provider list means no lookup implementation. `vat_prefix: None` means
no VIES mapping; it does not invent national VAT support. Languages expose known
`available` codes, the adapter's `default`, and `selectable`; BG/RO adapters do
not currently accept caller-selected languages. The catalog is not a health check
or a claim that every address format has been reviewed.

## Look up a Bulgarian business

```no_run
use eu_company_lookup::{Client, LookupError};

# async fn example() -> Result<(), LookupError> {
let client = Client::new()?;
// Synthetic business code, without a BG VAT prefix.
match client.lookup_bg_company("123456789").await {
    Ok(result) => println!("{}: {} ({:?})", result.tax_id, result.company_name, result.source),
    Err(LookupError::NotFound) => println!("No matching record in either register"),
    Err(error) if error.is_service_unavailable() => {
        println!("Verification unavailable; the identifier may still be correct");
    }
    Err(error) => return Err(error),
}
# Ok(())
# }
```

ЕИК and BULSTAT formats overlap; callers do not select a register by pattern.
The complete nine- or thirteen-digit code and leading zeroes are preserved.
Prefixed VAT IDs and ten-digit personal identifiers are rejected before any request.

The result includes company names, source, official record URL, legal form,
a shared typed lifecycle status, address and registry details. Status is limited to `Active`, `Closed`, or `Unknown` (null when absent);
original provider status values remain in `nativeRegistryStatus` and
source-specific fields. Commercial Register details
can include managers, sole capital owners, capital and constitutive-document links.
Unavailable details remain null. The registered foreign-script name is not a
translation, and the UIC field entry date is not necessarily incorporation date.

`BgRegistryClient::lookup` is available when only Bulgarian lookup is needed.
See the [Bulgarian registry guide](https://github.com/medentic/eu-company-lookup/blob/main/docs/reference/bulgarian-registry.md) for fallback rules,
field meanings and coverage limitations.

## Look up a Romanian taxpayer

```no_run
use eu_company_lookup::{Client, LookupError};

# async fn example() -> Result<(), LookupError> {
let client = Client::new()?;
// Synthetic CUI without the RO VAT prefix.
let record = client.lookup_ro_company("1234").await?;
println!("{}: {}", record.tax_id, record.company_name);
println!("Domestic VAT registration: {:?}", record.tax_status.vat_registered);
# Ok(())
# }
```

ANAF taxpayer details and domestic VAT status are separate from VIES validity.
The Romanian result preserves source address text and distinct registered-office
and fiscal-address fields. It is not an ONRC extract. See the
[Romanian registry guide](https://github.com/medentic/eu-company-lookup/blob/main/docs/reference/romanian-registry.md)
for dates, fields, limits and verification scope.

## Handle failures and configure limits

`valid: false` means an identifier-matched negative VIES response.
`LookupError::NotFound` means definitive misses from both Bulgarian registers.
`LookupError::RegistryNotFound` identifies a definitive miss from ANAF only.
Timeouts, throttling, access errors, changed schemas and identity mismatches remain
errors; they never establish that an identifier is invalid.

`error.is_service_unavailable()` identifies availability failures. `Busy`, rate
limits, cooldowns and unexpected responses remain separate error categories.
Error messages omit submitted identifiers, owner details and arbitrary response bodies.

| Default | VIES | Bulgarian lookup | Romanian ANAF |
| --- | --- | --- | --- |
| Total deadline, including admission waiting | 8 seconds | 30 seconds across both registers | 30 seconds |
| Connect timeout | 3 seconds | 3 seconds | 3 seconds |
| Concurrent lookups across clones | 2 globally, 1 per VAT jurisdiction | 1 | 1 |
| Request start spacing | 2 seconds globally, 5 per jurisdiction | None | 2 seconds |
| Failure cooldown | 60 seconds, extended by Retry-After | 60 seconds, extended by Retry-After | 60 seconds, extended by Retry-After |
| Response body limit | 64 KiB | 1 MiB | 1 MiB |

These are client policies. The ANAF adapter also enforces its documented minimum
one-second spacing; see the Romanian reference for provider limits. There are no
automatic retries, redirects, cookies, ambient proxies or persistent cache.
Clones share limits; separate clients and processes require application-level
coordination. Cancellation releases local capacity but cannot cancel work already
received by a provider.

```rust
use eu_company_lookup::{Client, ClientConfig, BgRegistryConfig};
use std::time::Duration;

let mut vies = ClientConfig::default();
vies.timeout = Duration::from_secs(20);
let mut registry = BgRegistryConfig::default();
registry.timeout = Duration::from_secs(45);
let client = Client::with_configs(vies, registry)?;
# Ok::<(), eu_company_lookup::LookupError>(())
```

## Address utilities and API stability

```rust
use eu_company_lookup::locations::{parse_address, Country};

let address = parse_address("ул. Примерна 1, гр. СОФИЯ 1000", "bg");
assert_eq!(address.city.as_deref(), Some("София"));
assert_eq!(Country::BG.native_name(), "България");
```

A country hint selects parsing rules; it does not establish the physical country.
Country metadata coverage is broader than registry and address-parser coverage.

Result and configuration structs and extensible enums are non-exhaustive. Construct
configurations through `Default`, read public result fields and use fallback match
arms. See the [API compatibility contract](https://github.com/medentic/eu-company-lookup/blob/main/docs/reference/api-stability.md) for JSON guarantees,
address corrections and Rust-version policy.

## Development

New maintainers and AI agents can start with the
[agent handoff guide](https://github.com/medentic/eu-company-lookup/blob/main/docs/maintainers/agent-handoff.md),
which links domain ownership, registry field mappings and verification workflows.

The [documentation index](https://github.com/medentic/eu-company-lookup/blob/main/docs/README.md)
groups consumer references, development workflows and release maintenance.

Normal tests are offline and use synthetic fixtures. Run `cargo test --locked --all-targets` and
`cargo test --locked --doc`; CI is Linux-only. The
[contributor guide](https://github.com/medentic/eu-company-lookup/blob/main/CONTRIBUTING.md)
covers the complete check suite, module layout and opt-in provider testing.

The runnable `check_vat`, `lookup_bg_company` and `lookup_ro_company` examples contact live providers
and print returned company data. Development queues, collected responses, local
reports are local-only and excluded from public Git history and the published
crate. Agent/release instructions are repository-only.

## Country coverage and verification

The target scope is all 30 European Economic Area (EEA) countries. Northern
Ireland is listed separately for the existing XI VIES scope; it is not an EEA
country. Target scope does not imply implemented national lookup support.

Counts below are distinct identifiers in the retained, reviewed live-response
corpora, not HTTP requests, retries or synthetic tests. They describe the evidence
reviewed for 2.0.1, not an exhaustive record of every live attempt. A zero means no
reviewed corpus is retained for that operation. Provider failures are not negative
registration results and are not counted as reviewed outcomes.

### National registers and taxpayer sources

VIES is listed separately below. ANAF is a taxpayer source, not an ONRC company
register extract. Priority numbers preserve the existing planning order; a dash
means no priority has been assigned. Planned integrations have no release date.

| EEA country | Implemented register / source | Distinct company / tax IDs tested | Delivery status | Next priority |
| --- | --- | ---: | --- | --- |
| Austria | Not implemented | 0 | Planned | 1 |
| Belgium | Not implemented | 0 | Planned | 1 |
| Bulgaria | Commercial Register | 278 | Released in 2.0.0 | — |
| Bulgaria | BULSTAT | 10 | Released; broader verification pending | — |
| Croatia | Not implemented | 0 | Planned | — |
| Cyprus | Not implemented | 0 | Planned | — |
| Czechia | Not implemented | 0 | Planned | 1 |
| Denmark | Not implemented | 0 | Planned | — |
| Estonia | Not implemented | 0 | Planned | — |
| Finland | Not implemented | 0 | Planned | — |
| France | Not implemented | 0 | Planned | — |
| Germany | Not implemented | 0 | Planned | — |
| Greece | Not implemented | 0 | Planned | 2 |
| Hungary | Not implemented | 0 | Planned | — |
| Iceland | Not implemented | 0 | Planned | — |
| Ireland | Not implemented | 0 | Planned | — |
| Italy | Not implemented | 0 | Planned | — |
| Latvia | Not implemented | 0 | Planned | — |
| Liechtenstein | Not implemented | 0 | Planned | — |
| Lithuania | Not implemented | 0 | Planned | — |
| Luxembourg | Not implemented | 0 | Planned | — |
| Malta | Not implemented | 0 | Planned | — |
| Netherlands | Not implemented | 0 | Planned | — |
| Norway | Not implemented | 0 | Planned | — |
| Poland | Not implemented | 0 | Planned | — |
| Portugal | Not implemented | 0 | Planned | — |
| Romania | ANAF | 200 | Released in 2.0.0; further registry coverage planned | 3 |
| Slovakia | Not implemented | 0 | Planned | — |
| Slovenia | Not implemented | 0 | Planned | — |
| Spain | Not implemented | 0 | Planned | 1 |
| Sweden | Not implemented | 0 | Planned | — |

BULSTAT evidence covers ten nine-digit institutional identifiers; natural persons
and thirteen-digit branches remain outside that verified sample. The Bulgarian
register counts are per source and should not be added as a country-wide unique
identifier total. Reviewed corpora have passing offline replay; this does not
establish exhaustive field or entity-type coverage.

### VIES VAT verification

The VIES adapter is released for the prefixes below. Implementation and live
verification are separate. All EU VIES jurisdictions have retained reviewed
observations; Northern Ireland (XI) still has no retained live corpus. France
also has an unresolved provider rate-limit observation, excluded from the
reviewed-result count. Iceland, Liechtenstein and Norway have no VIES mapping
in this library.

| Country | VIES prefix | Distinct VAT IDs tested | Valid / invalid observations | Verification status |
| --- | --- | ---: | --- | --- |
| Austria | AT | 10 | 10 / 0 | Reviewed; offline replay passed |
| Belgium | BE | 10 | 10 / 0 | Reviewed; offline replay passed |
| Bulgaria | BG | 961 | 960 / 1 | Offline replay matched; 82 address ambiguities |
| Croatia | HR | 10 | 10 / 0 | Reviewed; offline replay passed |
| Cyprus | CY | 10 | 10 / 0 | Reviewed; offline replay passed |
| Czechia | CZ | 10 | 10 / 0 | Reviewed; offline replay passed |
| Denmark | DK | 10 | 10 / 0 | Reviewed; offline replay passed |
| Estonia | EE | 10 | 10 / 0 | Reviewed; offline replay passed |
| Finland | FI | 10 | 10 / 0 | Reviewed; offline replay passed |
| France | FR | 9 | 9 / 0 | Offline replay passed; one additional ID rate-limited |
| Germany | DE | 10 | 10 / 0 | Reviewed; offline replay passed |
| Greece | EL | 10 | 10 / 0 | Reviewed; offline replay passed |
| Hungary | HU | 10 | 10 / 0 | Reviewed; offline replay passed |
| Iceland | — | — | — | Not supported by VIES |
| Ireland | IE | 10 | 10 / 0 | Reviewed; offline replay passed |
| Italy | IT | 10 | 10 / 0 | Reviewed; offline replay passed |
| Latvia | LV | 10 | 10 / 0 | Reviewed; offline replay passed |
| Liechtenstein | — | — | — | Not supported by VIES |
| Lithuania | LT | 10 | 9 / 1 | Reviewed; offline replay passed |
| Luxembourg | LU | 10 | 10 / 0 | Reviewed; offline replay passed |
| Malta | MT | 10 | 10 / 0 | Offline replay matched; one known address limitation |
| Netherlands | NL | 10 | 10 / 0 | Reviewed; offline replay passed |
| Norway | — | — | — | Not supported by VIES |
| Poland | PL | 10 | 10 / 0 | Reviewed; offline replay passed |
| Portugal | PT | 10 | 10 / 0 | Offline replay matched; one known address limitation |
| Romania | RO | 200 | 191 / 9 | Reviewed; offline replay passed |
| Slovakia | SK | 10 | 10 / 0 | Reviewed; offline replay passed |
| Slovenia | SI | 10 | 9 / 1 | Reviewed; offline replay passed |
| Spain | ES | 10 | 10 / 0 | Reviewed; offline replay passed |
| Sweden | SE | 10 | 10 / 0 | Reviewed; offline replay passed |
| Northern Ireland (outside EEA) | XI | 0 | — | Live verification pending |

A matched invalid VIES observation counts as a tested identifier. An address
ambiguity does not change VAT validity and is not a fully passed address review.
Country completion requires at least 200 distinct VAT IDs and 200 distinct
company/tax IDs, source-bound expectation review and passing offline replay;
counts alone do not establish completion. Provider failures and unresolved
ambiguities remain separate in the local evidence.

## License

MIT with BSD-3-Clause address-rule material; the package license expression is
`MIT AND BSD-3-Clause`. See [LICENSE](https://github.com/medentic/eu-company-lookup/blob/main/LICENSE) and
[THIRD-PARTY-LICENSE.txt](https://github.com/medentic/eu-company-lookup/blob/main/THIRD-PARTY-LICENSE.txt). These licenses cover the code;
provider services and data remain subject to their own terms.
