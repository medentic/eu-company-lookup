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
| `client.lookup_company(&identifier)` | Shared company result for an explicit `BusinessId` country/scheme pair. |
| `client.lookup_bg_company(value)` | Exact Commercial Register match, with BULSTAT fallback only after a definitive miss. |
| `validate_ro_business_id_syntax(value)` / `RoBusinessId::parse(value)` | Offline Romanian CUI syntax validation. |
| `client.lookup_ro_company(value)` | ANAF taxpayer details and dated domestic tax status. |
| `client.capabilities()` | Local catalog of implemented providers, operations, identifier schemes and language behavior. |
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

## Next country registry priorities

These priorities guide additional registry work. BULSTAT work extends the existing
fallback adapter; Romanian work extends coverage beyond ANAF. The remaining
countries represent new national registry integrations. The numbers below are planning priorities, not
implemented coverage or release dates.

| Country / registry | Priority |
| --- | --- |
| Bulgaria — BULSTAT | Not yet assigned |
| Romania — further registry coverage | 3 |
| Greece | 2 |
| Austria | 1 |
| Belgium | 1 |
| Czechia | 1 |
| Spain | 1 |

## License

MIT with BSD-3-Clause address-rule material; the package license expression is
`MIT AND BSD-3-Clause`. See [LICENSE](https://github.com/medentic/eu-company-lookup/blob/main/LICENSE) and
[THIRD-PARTY-LICENSE.txt](https://github.com/medentic/eu-company-lookup/blob/main/THIRD-PARTY-LICENSE.txt). These licenses cover the code;
provider services and data remain subject to their own terms.
