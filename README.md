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
| EU VAT number | `client.lookup_vat("BG131129282")` | `VatLookupResult`: validity, company name and address |
| BG or RO business identifier | `client.lookup_company("131129282", "BG")` | `CompanyLookupResult`: common company view with typed source details |

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

Version **3** exposes the shared BG/RO company contract and automatic BG VIES
verification. See [migration from 2.x](docs/reference/api-stability.md#migrating-from-2x)
when upgrading an existing integration.

```toml
[dependencies]
eu-company-lookup = "3"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde_json = "1" # To print the JSON shown below.
```

[crates.io](https://crates.io/crates/eu-company-lookup) ·
[API documentation](https://docs.rs/eu-company-lookup) ·
[Migration from 1.x](docs/reference/api-stability.md#migrating-from-1x)

## Examples

Each network example is a complete Rust program: put it in `src/main.rs` with
the dependencies above and run `cargo run`. These programs contact live providers.
The VAT example shows an observed VIES result for Kaufland Bulgaria. The company
examples show Kaufland Bulgaria and Automobile Dacia, including every common field and all
`registryDetails` fields exactly as returned. Null values are included. Registry
data and availability can change between requests.

The `rust,no_run` code-fence annotation tells rustdoc to compile these Rust
examples without executing live requests during documentation tests.

## Look up VAT details

**Call** — create a client once, then reuse or clone it across requests:

```rust,no_run
use eu_company_lookup::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;
    let result = client.lookup_vat("BG131129282").await?;
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
```

**Returns** `Result<VatLookupResult, LookupError>`. Complete serialized result
from a live VIES lookup of Kaufland Bulgaria (`BG131129282`):

```json
{
  "vatId": "BG131129282",
  "valid": true,
  "source": "Vies",
  "company": "КАУФЛАНД БЪЛГАРИЯ ЕООД ЕНД КО - КД",
  "address": {
    "fullAddressByVies": " жк ж.к. Банишора ул. Скопие  №1А обл.СОФИЯ, гр.СОФИЯ 1233",
    "address": "жк. ж.к. Банишора, ул. Скопие №1А",
    "address2": null,
    "city": "София",
    "postcode": "1233",
    "region": "София",
    "country": "България",
    "countryISO2": "BG"
  }
}
```

Company details may be withheld or missing. Unknown address components remain
null; the original decoded address is retained. `valid: false` requires a matched
negative VIES response. A timeout or provider failure returns an error.

Only need a boolean? `client.is_vat_valid("BG131129282").await` returns
`Result<bool, LookupError>`. It makes its own request; use `result.valid` if you
already have the full result. [VIES input rules and field meanings](docs/reference/vies.md).

### Bulgarian VAT verification and future NRA integration

BG `lookup_company` always performs its VIES check after a successful nine-digit
Commercial Register or BULSTAT lookup. The Bulgarian company adapter owns this
step and calls the VIES domain internally; no separate enablement is required. It checks `BG` plus the full matched identifier; thirteen-digit branch
codes are never truncated or queried. A candidate alone does not establish VAT
identity. Only a valid VIES response with the same company name after case,
punctuation and whitespace normalization fills `vatRegistered: true` and `vatId`.
No fuzzy matching, translation or legal-form removal is performed.

The full VIES result is retained at `registryDetails.data.vatVerification`, including
negative responses and name mismatches. These leave the common VAT fields null;
VIES invalidity alone does not establish absence of domestic VAT registration.
`source` and `registryUrl` still identify the primary company registry; the nested
verification identifies `Vies` as its source. No VIES address replaces a registry
address. RO remains based on ANAF and makes no additional VIES request.

A VIES failure returns a lookup error, even if the preceding registry request
succeeded. The calls are sequential and each uses its own configured deadline and
admission limits (up to 30 seconds for BG, then up to 8 seconds for VIES by default).
The standalone VAT API is separate: `without_vies()` disables `lookup_vat` and
`is_vat_valid`, but does not disable the BG company adapter’s internal check. When
the standalone VIES API is configured, both paths share its client and rate limits;
otherwise BG uses default VIES bounds. There is no enrichment opt-out on the
shared BG company lookup. Use the direct VAT call when the VAT number is already
known; do not repeat it just to read an enrichment result.

A future improvement is integration with the official
[NRA public VAT register](https://portal.nra.bg/details/report-cash-terminal)
([service description](https://iisda.government.bg/adm_services/services/service_provision/83394)).
The register can supply the VAT number, name, correspondence address, registration
date and basis, and deregistration date. Its observed search form asks for a VAT
number; reliable EIK-to-VAT discovery and automated access still need investigation.

The page header also displays **ЕИК по БУЛСТАТ 131063188**. This identifies the
**National Revenue Agency itself**, not the company being queried. Never use that
header identifier to match a company or populate its `taxId` or `vatId`.

## Look up a company across supported countries

Use the common result when your application handles both Bulgaria and Romania.
Pass a national business code without its VAT prefix; leading zeroes are preserved.

Both calls return `Result<CompanyLookupResult, LookupError>` with the same
top-level fields. Only the typed `registryDetails` payload varies by source.

```rust,no_run
use eu_company_lookup::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;
    let kaufland = client.lookup_company("131129282", "BG").await?;
    let dacia = client.lookup_company("160796", "RO").await?;
    println!("{}", serde_json::to_string_pretty(&kaufland)?);
    println!("{}", serde_json::to_string_pretty(&dacia)?);
    Ok(())
}
```

### Bulgaria: Kaufland

Kaufland Bulgaria publishes business identifier `131129282` in its
[official company details](https://www.kaufland.bg/moyat-kaufland/igri/tiktok-usloviya.html).
The complete serialized company result, including automatic VIES enrichment:

```json
{
  "country": "BG",
  "taxId": "131129282",
  "vatRegistered": true,
  "vatId": "BG131129282",
  "source": "BulgarianCommercialRegister",
  "registryUrl": "https://portal.registryagency.bg/CR/Reports/ActiveConditionTabResult?uic=131129282",
  "foreignName": "KAUFLAND BULGARIA EOOD & CO",
  "companyName": "\"КАУФЛАНД БЪЛГАРИЯ ЕООД ЕНД КО\" КД",
  "legalForm": "Командитно дружество",
  "registeredOffice": {
    "fullAddressByRegistry": "Държава: БЪЛГАРИЯ\nОбласт: София (столица), Община: Столична\nНаселено място: гр. София, п.к. 1233\nр-н Сердика\nж.к. ж.к. Банишора, бул./ул. ул. Скопие № 1А",
    "address": "жк. ж.к. Банишора, ул. Скопие № 1А",
    "address2": null,
    "city": "София",
    "postcode": "1233",
    "region": "София (столица)",
    "municipality": "Столична",
    "country": "България",
    "countryISO2": "BG"
  },
  "fiscalAddress": null,
  "status": "active",
  "registryDetails": {
    "provider": "BulgarianCommercialRegister",
    "data": {
      "nativeRegistryStatus": "2",
      "vatVerification": {
        "vatId": "BG131129282",
        "valid": true,
        "source": "Vies",
        "company": "КАУФЛАНД БЪЛГАРИЯ ЕООД ЕНД КО - КД",
        "address": {
          "fullAddressByVies": " жк ж.к. Банишора ул. Скопие  №1А обл.СОФИЯ, гр.СОФИЯ 1233",
          "address": "жк. ж.к. Банишора, ул. Скопие №1А",
          "address2": null,
          "city": "София",
          "postcode": "1233",
          "region": "София",
          "country": "България",
          "countryISO2": "BG"
        }
      },
      "statusCode": 2,
      "registeredName": "КАУФЛАНД БЪЛГАРИЯ ЕООД ЕНД КО",
      "identifierEntryDate": "2008-05-30",
      "identifierEntryYear": 2008,
      "managers": null,
      "owners": null,
      "capital": {
        "registered": null,
        "paidIn": null
      },
      "businessActivity": "Търговия на дребно със стоки за широко потребление, които са допуснати до обществения пазар, между които промишлени и хранителни стоки в т.ч. пиротехнически средства, управление и участие в други предприятия, които осъществяват дейност в горепосочените отрасли и свързаните с тях сделки, придобиване, разпореждане, вземане и отдаване под наем, управление и експлоатация на недвижими имоти, както и на други вещни права върху недвижими имоти, както и всяка друга дейност, незабранена от българското законодателство.",
      "documents": [
        {
          "type": "constitutiveDocument",
          "title": "Актуален дружествен договор/учредителен акт/устав",
          "announcedAtLocal": "2023-09-14T16:58:59",
          "url": "https://portal.registryagency.bg/CR/DocumentAccess/MTMxMTI5MjgyJmUmY2I0ZTMwNGE5YTIyNGY4ZmEyMGE5YTRmMmMwZTkwNTCt2hlTAQvk8DHWIlGoXXyTS0lXdu08qI9iQVYT_Dtvtw"
        }
      ],
      "hasInstructions": true
    }
  }
}
```

The common VAT fields are populated by the successful, name-matched VIES check
recorded in `registryDetails.data.vatVerification`. The Commercial Register itself
does not report VAT registration. `foreignName` is the registered foreign-script
name. `identifierEntryDate` dates the UIC field entry, not incorporation. The
address preserves source wording, including repeated locality labels.

### Romania: Automobile Dacia

Automobile Dacia publishes fiscal identifier `160796` in its
[official company report](https://cdn.group.renault.com/dac/ro/despre-marca/Declaratianefinanciara2024.pdf).
The complete serialized ANAF lookup result:

```json
{
  "country": "RO",
  "taxId": "160796",
  "vatRegistered": true,
  "vatId": "RO160796",
  "source": "RomanianAnaf",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "AUTOMOBILE-DACIA SA",
  "legalForm": "SOCIETATE COMERCIALĂ PE ACŢIUNI",
  "registeredOffice": {
    "fullAddressByRegistry": null,
    "address": "Str. UZINEI 1",
    "address2": null,
    "city": "Loc. Mioveni Orş. Mioveni",
    "postcode": "115400",
    "region": "ARGEŞ",
    "municipality": null,
    "country": "Romania",
    "countryISO2": "RO"
  },
  "fiscalAddress": {
    "fullAddressByRegistry": "JUD. ARGEŞ, LOC. MIOVENI ORŞ. MIOVENI, STR. UZINEI, NR.1",
    "address": "Str. UZINEI 1",
    "address2": null,
    "city": "Loc. Mioveni Orş. Mioveni",
    "postcode": "115400",
    "region": "ARGEŞ",
    "municipality": null,
    "country": "Romania",
    "countryISO2": "RO"
  },
  "status": "unknown",
  "registryDetails": {
    "provider": "RomanianAnaf",
    "data": {
      "nativeRegistryStatus": "INREGISTRAT din data 30.11.1992",
      "requestDate": "2026-11-14",
      "checkedAt": "2026-11-14T16:13:58.179494339Z",
      "tradeRegisterNumber": "J1991000081036",
      "registrationDate": "1992-11-30",
      "registrationStatus": "INREGISTRAT din data 30.11.1992",
      "activityCode": "2910",
      "registeredOffice": {
        "street": "Str. UZINEI",
        "streetNumber": "1",
        "locality": "Loc. Mioveni Orş. Mioveni",
        "county": "ARGEŞ",
        "country": "Romania",
        "countryISO2": "RO",
        "postcode": "115400",
        "details": null
      },
      "fiscalAddress": {
        "street": "Str. UZINEI",
        "streetNumber": "1",
        "locality": "Loc. Mioveni Orş. Mioveni",
        "county": "ARGEŞ",
        "country": "Romania",
        "countryISO2": "RO",
        "postcode": "115400",
        "details": null
      },
      "taxStatus": {
        "vatRegistered": true,
        "vatOnCollection": false,
        "inactive": false,
        "splitVat": false,
        "eInvoiceRegistered": false
      }
    }
  }
}
```

ANAF supplied no address country in this response. The adapter defaults missing
address countries to `"Romania"` / `"RO"` based on the ANAF jurisdiction; these are
library defaults, not country values explicitly returned by ANAF. Explicit source
countries take precedence, with title-case names and uppercase recognized ISO codes.
The current adapter supplies no
verified company-record URL, so `registryUrl` remains null. `status: "unknown"`
reflects the conservative lifecycle mapping; ANAF's original status is retained
in the details.

Missing address countries default to `България` / `BG` for Bulgarian registries
and `Romania` / `RO` for ANAF. Explicit source countries take precedence. Country
names use title case and recognized ISO codes use uppercase throughout structured
addresses and registry details; original source text is preserved.

The top-level object is the shared company model: fields have the same meaning
and shape across providers. Provider-specific codes, native status text, special tax
regimes and source structures belong in `registryDetails` and cannot override
common fields. `status` is the normalized lifecycle; the original status lives
in `registryDetails.data.nativeRegistryStatus`.

`vatRegistered` reports VAT registration supported by the provider-specific
evidence, otherwise null. RO uses ANAF article 316 at the requested
fiscal date; this does not cover every special VAT registration. A positive
matched ANAF result supplies the RO `vatId`, using `RO` plus the numeric fiscal ID.
For RO, input padding remains in `taxId` but not the derived VAT ID. BG preserves
all nine digits in its VIES candidate. With BG automatic enrichment, valid and name-matched
results fill both VAT fields; other successful responses leave them null. There
is no generic country-prefix derivation across providers.
[VAT field scope and derivation](docs/reference/romanian-registry.md#shared-vat-fields).

The common view keeps `registeredOffice` and `fiscalAddress` separate and retains
ANAF-specific fields under `registryDetails`. `registryUrl` and `foreignName` are
common nullable fields: BG supplies a record link and, when present, a registered
foreign-script name; the current ANAF adapter supplies neither. The name is not a
translation. Unavailable optional fields remain null.
Country codes are case-insensitive and surrounding whitespace is ignored.
Unsupported countries return `UnsupportedIdentifierScheme`.
[Complete shared field reference](docs/reference/company-result.md) ·
[Compatibility and migration](docs/reference/api-stability.md).

Bulgarian codes must contain nine or thirteen digits. BULSTAT fallback follows
only a definitive Commercial Register miss, never a provider failure.
[Bulgarian fields and limitations](docs/reference/bulgarian-registry.md).

Romanian lookups use today's fiscal date in Europe/Bucharest. Domestic tax flags
are separate from VIES validity and company lifecycle. For an explicit fiscal
date, use the lower-level `RoRegistryClient::lookup_at`.
[Romanian fields and dates](docs/reference/romanian-registry.md).

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
