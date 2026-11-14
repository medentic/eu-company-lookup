# eu-company-lookup

Async Rust lookups for **EU VAT numbers (VIES)**, **Bulgarian businesses
(Commercial Register / BULSTAT)** and **Romanian taxpayers (ANAF)**.
Returns typed company details, source-preserving addresses and explicit errors.
No account, database or full-registry download is required for the current providers.

## Why this exists

Looking up a European business means integrating multiple national registries and
VIES, each with its own identifiers, interfaces, response formats and failure
rules. That knowledge is difficult to gather and maintain in one place.

This library brings those integrations and their country-specific parsing rules
behind one Rust API. The initial use case is to look up a business by tax or VAT
ID and use the returned company name and address in business details and billing
flows, while checking VAT validity separately. Applications can reuse this work
instead of building and maintaining each registry integration themselves.

The project started with Bulgaria and is expanding country by country toward
**EU and EEA business lookup**. Bulgaria and Romania are implemented today;
most national registry integrations are still pending. It queries supported
sources on demand rather than maintaining a database of all European companies.

## Choose a lookup

| You have | Call | Returns on success |
| --- | --- | --- |
| EU VAT number | `client.lookup_vat("BG123456789")` | `VatLookupResult`: validity, company name and address |
| Bulgarian ЕИК / BULSTAT code | `client.lookup_bg_company("123456789")` | `BgCompanyLookupResult`: identity, address and registry details |
| Romanian CUI | `client.lookup_ro_company("1234")` | `RoCompanyLookupResult`: identity, addresses and domestic tax status |
| BG or RO business identifier | `client.lookup_company(&id)` | `CompanyLookupResult`: common company view with typed source details |

National business lookup currently covers **BG and RO only** (ANAF is a taxpayer
source). VAT verification and company registration are separate checks. Syntax validation does not prove
checksum validity or registration.

## Country support

“Supported” means an implemented adapter, not guaranteed provider availability
or complete live verification. “Pending” means no national lookup is implemented.
The table covers all 30 EEA countries, plus the separate Northern Ireland scope.

| Country | VIES VAT lookup | National business / taxpayer lookup |
| --- | --- | --- |
| Austria | Supported (`AT`) | Pending |
| Belgium | Supported (`BE`) | Pending |
| Bulgaria | Supported (`BG`) | Supported — Commercial Register, BULSTAT fallback |
| Croatia | Supported (`HR`) | Pending |
| Cyprus | Supported (`CY`) | Pending |
| Czechia | Supported (`CZ`) | Pending |
| Denmark | Supported (`DK`) | Pending |
| Estonia | Supported (`EE`) | Pending |
| Finland | Supported (`FI`) | Pending |
| France | Supported (`FR`) | Pending |
| Germany | Supported (`DE`) | Pending |
| Greece | Supported (`EL`) | Pending |
| Hungary | Supported (`HU`) | Pending |
| Iceland | Not supported by VIES | Pending |
| Ireland | Supported (`IE`) | Pending |
| Italy | Supported (`IT`) | Pending |
| Latvia | Supported (`LV`) | Pending |
| Liechtenstein | Not supported by VIES | Pending |
| Lithuania | Supported (`LT`) | Pending |
| Luxembourg | Supported (`LU`) | Pending |
| Malta | Supported (`MT`) | Pending |
| Netherlands | Supported (`NL`) | Pending |
| Norway | Not supported by VIES | Pending |
| Poland | Supported (`PL`) | Pending |
| Portugal | Supported (`PT`) | Pending |
| Romania | Supported (`RO`) | Supported — ANAF taxpayer lookup |
| Slovakia | Supported (`SK`) | Pending |
| Slovenia | Supported (`SI`) | Pending |
| Spain | Supported (`ES`) | Pending |
| Sweden | Supported (`SE`) | Pending |
| Northern Ireland (outside EEA) | Supported (`XI`) | Not implemented; outside EEA target |

Greece uses VAT prefix `EL`, mapped to country `GR`. `XI` covers Northern Ireland
only; general `GB` VAT IDs and OSS/IOSS prefixes `EU` / `IM` are unsupported.
Romanian ANAF results are not ONRC trade-register extracts. See
[country coverage and verification](#country-coverage-and-verification) for live
evidence, known limitations and national-registry priorities.

## Install

Requires **Rust 1.98.1+** and **Tokio with I/O and time enabled**.

```toml
[dependencies]
eu-company-lookup = "2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde_json = "1" # To print the JSON shown below.
```

[crates.io](https://crates.io/crates/eu-company-lookup) ·
[API documentation](https://docs.rs/eu-company-lookup) ·
[Migration from 1.x](docs/reference/api-stability.md#migrating-from-1x)

## Examples

Each network example is a complete Rust program: put it in `src/main.rs` with
the dependencies above and run `cargo run`. Replace the synthetic identifier
with one you intend to look up. The JSON shows illustrative successful returns,
not live observations or guaranteed results for these identifiers. All fields
are shown, including nulls; available values depend on the provider.

The `rust,no_run` code-fence annotation tells rustdoc to compile these Rust
examples without executing live requests during documentation tests.

## Look up VAT details

**Call** — create a client once, then reuse or clone it across requests:

```rust,no_run
use eu_company_lookup::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;
    let result = client.lookup_vat("BG123456789").await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
```

**Returns** `Result<VatLookupResult, LookupError>`. This is an illustrative
successful response with synthetic data, not a live result for the example ID:

```json
{
  "vatId": "BG123456789",
  "valid": true,
  "source": "Vies",
  "company": "Synthetic Company",
  "address": {
    "fullAddressByVies": "ул. ПРИМЕР 1 обл.РУСЕ, гр.РУСЕ 7000",
    "address": "ул. ПРИМЕР 1",
    "address2": null,
    "city": "Русе",
    "postcode": "7000",
    "region": "Русе",
    "country": "България",
    "countryISO2": "BG"
  }
}
```

Company details may be withheld or missing. Unknown address components remain
null; the original decoded address is retained. `valid: false` requires a matched
negative VIES response. A timeout or provider failure returns an error.

Only need a boolean? `client.is_vat_valid("BG123456789").await` returns
`Result<bool, LookupError>`. It makes its own request; use `result.valid` if you
already have the full result. [VIES input rules and field meanings](docs/reference/vies.md).

## Look up a company across supported countries

Use the common result when your application handles both Bulgaria and Romania.
Pass a national business code without its VAT prefix; leading zeroes are preserved.

**Call:**

```rust,no_run
use eu_company_lookup::{BusinessId, Client, IdentifierScheme};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;
    let id = BusinessId::parse("RO", IdentifierScheme::RoBusinessId, "001234")?;
    let company = client.lookup_company(&id).await?;
    println!("{}", serde_json::to_string_pretty(&company)?);
    Ok(())
}
```

**Returns** `Result<CompanyLookupResult, LookupError>`. Complete serialized example:

```json
{
  "identifier": {
    "country": "RO",
    "scheme": "RoBusinessId",
    "value": "001234"
  },
  "source": "RomanianAnaf",
  "companyName": "Synthetic Company SRL",
  "legalForm": null,
  "registeredOffice": {
    "fullAddressByRegistry": null,
    "address": "Strada Exemplului 1",
    "address2": null,
    "city": "București",
    "postcode": "010101",
    "region": "București",
    "municipality": null,
    "country": "ROMANIA",
    "countryISO2": "RO"
  },
  "fiscalAddress": {
    "fullAddressByRegistry": "Strada Exemplului 1, București, 010101",
    "address": "Strada Exemplului 1",
    "address2": null,
    "city": "București",
    "postcode": "010101",
    "region": "București",
    "municipality": null,
    "country": "ROMANIA",
    "countryISO2": "RO"
  },
  "status": "unknown",
  "nativeRegistryStatus": "INREGISTRAT",
  "details": {
    "provider": "RomanianAnaf",
    "data": {
      "requestDate": "2026-11-14",
      "checkedAt": "2026-11-14T10:00:00Z",
      "tradeRegisterNumber": null,
      "registrationDate": null,
      "registrationStatus": "INREGISTRAT",
      "activityCode": null,
      "registeredOffice": {
        "street": "Strada Exemplului",
        "streetNumber": "1",
        "locality": "București",
        "county": "București",
        "country": "ROMANIA",
        "postcode": "010101",
        "details": null
      },
      "fiscalAddress": {
        "street": "Strada Exemplului",
        "streetNumber": "1",
        "locality": "București",
        "county": "București",
        "country": "ROMANIA",
        "postcode": "010101",
        "details": null
      },
      "taxStatus": {
        "vatRegistered": true,
        "vatOnCollection": false,
        "inactive": false,
        "splitVat": null,
        "eInvoiceRegistered": null
      }
    }
  }
}
```

The common view keeps `registeredOffice` and `fiscalAddress` separate and retains
ANAF-specific fields under `details`. Unavailable optional fields remain null.
For Bulgaria, use country `"BG"` and
`IdentifierScheme::BgBusinessId`. [Shared result contract](docs/reference/api-stability.md).

## Look up a Bulgarian business

**Call:**

```rust,no_run
use eu_company_lookup::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;
    let company = client.lookup_bg_company("123456789").await?;
    println!("{}", serde_json::to_string_pretty(&company)?);
    Ok(())
}
```

**Returns** `Result<BgCompanyLookupResult, LookupError>`. Complete serialized example:

```json
{
  "taxId": "123456789",
  "source": "BulgarianCommercialRegister",
  "registryUrl": "https://portal.registryagency.bg/CR/Reports/ActiveConditionTabResult?uic=123456789",
  "companyName": "Пример ЕООД",
  "companyEnglishName": null,
  "legalForm": "ЕООД",
  "address": {
    "fullAddressByRegistry": "БЪЛГАРИЯ, гр. СОФИЯ 1000, ул. Примерна 1",
    "address": "ул. Примерна 1",
    "address2": null,
    "city": "София",
    "postcode": "1000",
    "region": null,
    "municipality": null,
    "country": "България",
    "countryISO2": "BG"
  },
  "status": "active",
  "nativeRegistryStatus": "1",
  "registryDetails": {
    "statusCode": 1,
    "registeredName": "Пример",
    "identifierEntryDate": "2020-01-15",
    "identifierEntryYear": 2020,
    "managers": [
      {
        "name": "Примерен Управител",
        "role": "manager",
        "country": "България",
        "countryISO2": "BG",
        "sourceText": "Примерен Управител, Държава: БЪЛГАРИЯ"
      }
    ],
    "owners": null,
    "capital": {
      "registered": {
        "amount": "100.00",
        "currency": "EUR",
        "sourceText": "100.00 EUR"
      },
      "paidIn": null
    },
    "businessActivity": null,
    "documents": null,
    "hasInstructions": false
  }
}
```

`companyEnglishName` is a separately registered foreign-script name, not a
translation. `identifierEntryDate` dates the UIC field entry, not necessarily
incorporation. Null `owners` or `documents` means unavailable, not proof of absence.
Capital values use exact decimal strings; currencies are retained from the source.

For BULSTAT, the top-level fields are the same, `source` becomes `"Bulstat"`, and
`registryDetails` contains `subjectText` and `statusText` instead of Commercial
Register fields. Only nine- or thirteen-digit business codes are accepted;
BG-prefixed VAT IDs and ten-digit personal identifiers are rejected. Fallback to
BULSTAT happens only after a definitive Commercial Register miss, never after a
provider failure.
[Bulgarian fields and coverage limitations](docs/reference/bulgarian-registry.md).

## Look up a Romanian taxpayer

**Call:**

```rust,no_run
use eu_company_lookup::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;
    let company = client.lookup_ro_company("1234").await?;
    println!("{}", serde_json::to_string_pretty(&company)?);
    Ok(())
}
```

**Returns** `Result<RoCompanyLookupResult, LookupError>`. Complete serialized example:

```json
{
  "taxId": "1234",
  "source": "RomanianAnaf",
  "requestDate": "2026-11-14",
  "checkedAt": "2026-11-14T10:00:00Z",
  "companyName": "Synthetic Company SRL",
  "tradeRegisterNumber": null,
  "status": "unknown",
  "nativeRegistryStatus": "INREGISTRAT",
  "registrationStatus": "INREGISTRAT",
  "registrationDate": null,
  "legalForm": null,
  "activityCode": null,
  "fullFiscalAddress": "Strada Exemplului 1, București, 010101",
  "registeredOffice": {
    "street": "Strada Exemplului",
    "streetNumber": "1",
    "locality": "București",
    "county": "București",
    "country": "ROMANIA",
    "postcode": "010101",
    "details": null
  },
  "fiscalAddress": {
    "street": "Strada Exemplului",
    "streetNumber": "1",
    "locality": "București",
    "county": "București",
    "country": "ROMANIA",
    "postcode": "010101",
    "details": null
  },
  "taxStatus": {
    "vatRegistered": true,
    "vatOnCollection": false,
    "inactive": false,
    "splitVat": null,
    "eInvoiceRegistered": null
  }
}
```

`lookup_ro_company` requests today's fiscal status in Europe/Bucharest;
`requestDate` and `checkedAt` above are illustrative. `lookup_ro_company_at`
accepts an explicit fiscal date. ANAF structured addresses retain street numbers
separately. A supplied registration description currently maps to `"unknown"`;
missing status remains null. Domestic tax flags do not establish company lifecycle.
ANAF domestic VAT status is separate from VIES validity. This is taxpayer
information, not an ONRC extract. [Romanian fields and dated lookups](docs/reference/romanian-registry.md).

## Handle failures and configure limits

| Outcome | Meaning |
| --- | --- |
| `Ok(result)` with VIES `valid: false` | Matched negative VAT response |
| `Err(LookupError::NotFound)` | Definitive misses from both Bulgarian registers |
| `Err(LookupError::RegistryNotFound { .. })` | Definitive ANAF miss only |
| Invalid input, timeout, rate limit, service or schema error | Lookup failed; registration status remains unproven |

There are no automatic retries or persistent caching. Clones share admission
limits and cooldowns; separate clients and processes need their own coordination.

<details>
<summary>Client defaults and configuration</summary>

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

fn main() -> Result<(), eu_company_lookup::LookupError> {
    let mut vies = ClientConfig::default();
    vies.timeout = Duration::from_secs(20);
    let mut registry = BgRegistryConfig::default();
    registry.timeout = Duration::from_secs(45);
    let _client = Client::with_configs(vies, registry)?;
    Ok(())
}
```

`Client::builder()` enables all current adapters. To select only the ones you need:

```rust
use eu_company_lookup::{ClientBuilder, LookupError};

fn main() -> Result<(), LookupError> {
    let _client = ClientBuilder::empty()
        .vies(Default::default())
        .romanian_registry(Default::default())
        .build()?;
    Ok(())
}
```

Disabled operations return `OperationDisabled` before parsing or sending a request.
`capabilities()` reports compiled support; `client.capabilities()` reports enabled
providers. Discovery is local and does not indicate provider health.

</details>

## Offline utilities

```rust
use eu_company_lookup::{validate_vat_syntax, locations::parse_address};

fn main() -> Result<(), eu_company_lookup::LookupError> {
    let vat = validate_vat_syntax("bg 123456789")?;
    assert_eq!(vat.as_str(), "BG123456789");

    let address = parse_address("ул. Примерна 1, гр. СОФИЯ 1000", "bg");
    assert_eq!(address.city.as_deref(), Some("София"));
    Ok(())
}
```

Syntax helpers also exist for BG and RO business codes. They preserve leading
zeroes and check format only. Address parsing is conservative; a country hint
selects parsing rules without proving the physical country.

## Country coverage and verification

VIES supports EU VAT jurisdictions and Northern Ireland (`XI`). Greece uses `EL`.
General GB, OSS/IOSS (`EU` / `IM`), Iceland, Liechtenstein and Norway are outside
VIES coverage. National company lookup currently supports Bulgaria and Romania;
other national registries are planned.

Implementation and live verification are separate. BULSTAT has limited reviewed
coverage; XI has no retained live corpus. Known source/address limitations remain
for BG, MT and PT, and a French provider rate-limit observation remains unresolved.

<details>
<summary>Country-by-country evidence, limitations and priorities</summary>

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

</details>

## Development

Start with the [contributor guide](https://github.com/medentic/eu-company-lookup/blob/main/CONTRIBUTING.md)
for offline tests and checks, or the [documentation index](https://github.com/medentic/eu-company-lookup/blob/main/docs/README.md)
for provider and maintainer guides. Live examples contact providers; collected
records and development evidence stay local-only and outside the published crate.

## License

MIT with BSD-3-Clause address-rule material; the package license expression is
`MIT AND BSD-3-Clause`. See [LICENSE](https://github.com/medentic/eu-company-lookup/blob/main/LICENSE) and
[THIRD-PARTY-LICENSE.txt](https://github.com/medentic/eu-company-lookup/blob/main/THIRD-PARTY-LICENSE.txt). These licenses cover the code;
provider services and data remain subject to their own terms.
