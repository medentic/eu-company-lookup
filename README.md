# eu-company-lookup

[crates.io](https://crates.io/crates/eu-company-lookup) ·
[API documentation](https://docs.rs/eu-company-lookup) ·
[Error reference](docs/reference/errors.md)

[Install](#install) · [Examples](#examples) · [Country support](#country-support) ·
[Errors and configuration](#handle-failures-and-configure-limits)

Async Rust lookups for **EU VAT numbers (VIES)** and national company or taxpayer
information. Returns typed company details, source-preserving addresses and a
shared error object with stable IDs.

Version **5.0** provides national lookup for **Austria, Belgium, Bulgaria,
Czechia, Estonia, Finland, France, Greece, Iceland, Ireland, Latvia, Lithuania,
Norway, Poland, Romania and Slovakia**, plus all 28 supported VIES VAT prefixes.
Implementation and reviewed live coverage are separate; see the
[country table](#country-support) and [verification requirements](#country-coverage-and-verification).

No account, database or full-registry download is required for the current providers.

## Why this exists

Business lookup requires country-specific identifiers, registry interfaces and
field mappings. This crate brings those integrations behind one async Rust API
for billing, onboarding and company-data forms. It queries sources on demand
and keeps VAT verification separate from national company or taxpayer records.

## Install

Requires **Rust 1.98.1+** and **Tokio with I/O and time enabled**.

Version **4** adds Irish and Norwegian national lookup, one common `LookupError`
object and opt-in raw HTTP diagnostics. The shared company contract and automatic
BG VIES verification remain available. See [migration from 3.x](docs/reference/errors.md#migrating-from-3x)
or [migration from 2.x](docs/reference/api-stability.md#migrating-from-2x)
when upgrading an existing integration. Version 5.0 additionally enforces resolved
company lifecycle and supported VAT checksums; see
[migration from 4.0](docs/reference/api-stability.md#migrating-from-40).
Version 5.0 replaces Chrono with Jiff datetime types; see the
[datetime migration](docs/reference/api-stability.md#datetime-types-in-50).

```toml
[dependencies]
eu-company-lookup = "5.0"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "net", "time"] }
serde_json = "1" # To print the JSON shown below.
```

For older integrations, see [migration from 1.x](docs/reference/api-stability.md#migrating-from-1x).

## Choose a lookup

| You have | Call | Returns on success |
| --- | --- | --- |
| EU VAT number | `client.lookup_vat("BG131129282")` | `VatLookupResult`: validity, company name and address |
| National business identifier | `client.lookup_company("131129282", "BG")` | `CompanyLookupResult`: common company view with typed source details |

Use the [country-support table](#country-support) or `Client::SUPPORTED_COMPANY_COUNTRIES`
for national lookup coverage. ANAF is a taxpayer source. VAT verification and
company registration are separate checks; syntax validation does not prove
checksum validity or registration.

On development main, national lookups require a resolved lifecycle. Missing or
ambiguous lifecycle returns [`unresolved_data`](docs/reference/errors.md#unresolved-company-lifecycle),
not a successful partial company or a negative registration result. Documented
proceedings have explicit status values; other optional fields remain nullable.

## Examples

The following lookup examples use the 5.0 API. Put either Rust
program in `src/main.rs` with the dependencies above. Running it contacts live
providers; documentation tests compile `no_run` examples without making requests.
Explore [complete output examples](docs/examples/README.md) for synthetic positive,
negative and edge cases, including each implemented country and VIES.

### Look up VAT details

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

Returns `Result<VatLookupResult, LookupError>`.

[VIES output examples: valid, invalid, withheld details and failures](docs/examples/vies.md).

Company details may be withheld or missing. Unknown address components remain
null; the original decoded address is retained. `valid: false` requires a matched
negative VIES response. A timeout or provider failure returns an error.

Only need a boolean? `client.is_vat_valid("BG131129282").await` returns
`Result<bool, LookupError>`. It makes its own request; use `result.valid` if you
already have the full result. [VIES input rules and field meanings](docs/reference/vies.md).

### Look up a company across supported countries

Use the common result when your application handles multiple supported countries.
Pass a national business code without its VAT prefix; leading zeroes are preserved.

These calls return `Result<CompanyLookupResult, LookupError>` with the same
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

The same method accepts an Irish CRO company number with `"IE"` or a Norwegian
organisation number with `"NO"`. These are national business identifiers, not VAT
numbers. Norway is outside VIES coverage.

Explore the [country output gallery](docs/examples/README.md), including
[Bulgaria and BULSTAT fallback](docs/examples/bg.md), [Romania](docs/examples/ro.md),
[Ireland](docs/examples/ie.md) and [Norway](docs/examples/no.md). Each page shows
complete JSON, the negative/failure boundary and country-specific edge cases.

## Understand the results

Company lookups share one top-level model. Common fields keep the same meaning
across providers; native codes, tax regimes, status text and source-specific
addresses remain in typed `registryDetails.data`.

- Identifiers are strings. Preserve leading zeroes; syntax validation does not
  establish a checksum, registration or VAT validity.
- Missing optional facts remain null. A null `vatRegistered` is unknown, not false.
  VAT IDs are populated only by the provider-specific mapping, never by a universal
  country-prefix rule.
- `registeredOffice` and `fiscalAddress` are distinct roles. The adapter does not
  substitute a postal or business address for either role without source support.
- Original decoded address text is retained where supplied. Normalization and
  component parsing use a working copy; uncertain components remain null.
- `status` is the normalized company lifecycle. Native status stays in the provider
  details; tax activity does not by itself establish legal lifecycle.

Bulgarian registry addresses default a missing country to `България` / `BG`;
ANAF addresses default to `Romania` / `RO`. Explicit source countries take
precedence. VIES address country codes identify the VAT jurisdiction and do not
prove the physical location of the address.

See the [complete shared field contract](docs/reference/company-result.md),
[VIES result semantics](docs/reference/vies.md#result-and-address-semantics) and
[API compatibility rules](docs/reference/api-stability.md).

### Provider-specific behavior

The [country example pages](docs/examples/README.md) cover accepted identifiers,
source scope, address roles, VAT fields and lifecycle edge cases for every adapter.
A missing or ambiguous record is not automatically `not_found`.

### Bulgarian VAT verification

Nine-digit BG company results require an internal VIES check; thirteen-digit
branches are never truncated into VAT candidates. Only a valid, name-matched
observation fills common VAT fields. A negative or mismatched observation leaves
them null; a VIES failure returns an error. See the [BG output examples](docs/examples/bg.md)
and [enrichment contract](docs/reference/bulgarian-registry.md#shared-vat-fields)
for fallback, field preservation and client configuration.

## Country support

The table describes the 5.0 source distribution. “Implemented” does not claim
complete live verification. “Pending” means no national lookup is implemented. VIES support
is separate and does not imply a national registry adapter.
The table covers all 30 EEA countries, plus Northern Ireland. A dash in the VIES
column means the jurisdiction is not supported by VIES.

| Country | VIES prefix | National source | Availability |
| --- | --- | --- | --- |
| [Austria](docs/examples/at.md) | `AT` | Firmenbuch active-record lookup | Implemented |
| [Belgium](docs/examples/be.md) | `BE` | NBB Consult company data | Implemented |
| [Bulgaria](docs/examples/bg.md) | `BG` | Commercial Register, BULSTAT fallback | Implemented |
| Croatia | `HR` | — | Pending |
| Cyprus | `CY` | — | Pending |
| [Czechia](docs/examples/cz.md) | `CZ` | ARES economic subjects | Implemented |
| Denmark | `DK` | — | Pending |
| [Estonia](docs/examples/ee.md) | `EE` | RIK public search | Implemented |
| [Finland](docs/examples/fi.md) | `FI` | PRH YTJ company data | Implemented |
| [France](docs/examples/fr.md) | `FR` | API Recherche Entreprises (SIREN) | Implemented |
| Germany | `DE` | — | Pending |
| [Greece](docs/examples/gr.md) | `EL` | GEMI by registration number | Implemented |
| Hungary | `HU` | — | Pending |
| [Iceland](docs/examples/is.md) | — | Skatturinn company pages | Implemented |
| [Ireland](docs/examples/ie.md) | `IE` | CRO dataset lookup | Implemented |
| Italy | `IT` | — | Pending |
| [Latvia](docs/examples/lv.md) | `LV` | Registry open data | Implemented |
| Liechtenstein | — | — | Pending |
| [Lithuania](docs/examples/lt.md) | `LT` | JAR dataset lookup | Implemented |
| Luxembourg | `LU` | — | Pending |
| Malta | `MT` | — | Pending |
| Netherlands | `NL` | — | Pending |
| [Norway](docs/examples/no.md) | — | Enhetsregisteret entity lookup | Implemented |
| [Poland](docs/examples/pl.md) | `PL` | KRS business and association extracts | Implemented |
| Portugal | `PT` | — | Pending |
| [Romania](docs/examples/ro.md) | `RO` | ANAF taxpayer lookup | Implemented |
| [Slovakia](docs/examples/sk.md) | `SK` | RÚZ public accounting entities | Implemented |
| Slovenia | `SI` | — | Pending |
| Spain | `ES` | — | Pending |
| Sweden | `SE` | — | Pending |
| Northern Ireland (outside EEA) | `XI` | — | Outside national lookup scope |

Greece uses VAT prefix `EL`, mapped to country `GR`. `XI` covers Northern Ireland
only; general `GB` VAT IDs and OSS/IOSS prefixes `EU` / `IM` are unsupported.
Romanian ANAF results are not ONRC trade-register extracts. See
[country coverage and verification](#country-coverage-and-verification) for verification policy and remaining work.

## Handle failures and configure limits

Public lookup, validation and client-construction methods return
`Result<T, LookupError>`. In 4.0, errors share one object with `id`, `title`, `description`, `isServiceUnavailable` and `details`.
Match the stable `ErrorId` in Rust (snake-case `id` in JSON); do not parse the
English title or description. Details identify the provider, country, reason,
upstream HTTP status and retry guidance when available.

| Error IDs | What to do |
| --- | --- |
| `invalid_input`, `unsupported_identifier` | Check identifier syntax, country and scheme. |
| `operation_disabled`, `invalid_configuration`, `initialization_failed` | Check client configuration and initialization. |
| `busy`, `rate_limited`, `cooling_down` | Respect admission limits and available retry instructions. |
| `service_unavailable` | Verification could not complete; registration remains unknown. |
| `access_denied`, `provider_rejected_input` | Inspect provider details before retrying or changing the request. |
| `unexpected_response`, `response_mismatch`, `response_too_large` | Inspect the response failure; do not treat it as a negative registration result. |
| `not_found` | The identified registry sources definitively missed the company. |
| `unresolved_data` (main) | A record matched, but lifecycle could not be resolved; registration is not disproved. |
| `unknown_error` | Generic failure; no validity conclusion follows. |

`service_unavailable` sets `isServiceUnavailable: true`; rate limiting and
malformed responses retain separate classifications. A matched negative VIES
response is an `Ok` result, not a `LookupError`:

| Outcome | Meaning |
| --- | --- |
| `Ok(result)` with VIES `valid: false` | Matched negative VAT response |
| `not_found`, reason `both_bulgarian_registers_missed` | Definitive misses from both Bulgarian registers |
| `not_found`, provider `RomanianAnaf` | Definitive ANAF miss only |
| `not_found`, provider `CzechAres` (main) | Documented ARES negative response only |
| Invalid input, timeout, rate limit, service or schema error | Lookup failed; registration status remains unproven |

See the [full error catalog and 3.x migration guide](docs/reference/errors.md)
for handling examples. Each adapter configuration offers `raw_debug: true` for
opt-in request/response diagnostics and cURL reproduction in error details.
Raw diagnostics are disabled by default and can include sensitive data; keep
explicitly serialized captures in an administrator/debug channel.

Create a client once and reuse or clone it. There are no automatic retries or
persistent caching. Clones share admission limits and cooldowns; separate clients
and processes need their own coordination. VIES capacity varies by country; see
[service availability](docs/reference/vies.md#national-services-and-availability).

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

These are client policies. Irish and Norwegian registry clients use a 30-second
total deadline, a 3-second connect timeout, one concurrent lookup, 2-second start
spacing, a 60-second cooldown and a 1 MiB body limit by default.

The ANAF adapter also enforces its documented minimum
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

`Client::builder()` enables every adapter compiled into your version. To select
only the ones you need:

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

Disabled operations return error ID `operation_disabled` before parsing or sending a request.
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

Each implemented national adapter also exports a typed business identifier and
syntax validator. Available validators depend on the version installed; consult
the [API documentation](https://docs.rs/eu-company-lookup). They preserve leading
zeroes and check format only. Address parsing is conservative; a country hint
selects parsing rules without proving the physical country.

## Country coverage and verification

An implemented adapter, a published adapter and complete country verification
are different milestones. The [country table](#country-support) describes
availability; it does not promise provider uptime or exhaustive entity coverage.

VIES verification requires at least **200 distinct valid VAT IDs and one matched
invalid VAT ID per supported prefix**, with reviewed source-bound expectations
and passing offline replay against the current code. Provider errors count toward
neither threshold. National lookup has a separate gate of at least 200 distinct
company/tax IDs where that operation applies. Request counts alone do not
establish completion of either operation.
Matched negatives, provider failures and address ambiguities are recorded
separately. Collected records, identifier queues and changing batch counts stay
in local evidence rather than this README.

### VIES verification status

All 28 VIES prefixes are implemented. Retained evidence meets the VIES gate for
26 prefixes; **France (`FR`) and Northern Ireland (`XI`) remain incomplete**.
Germany (`DE`) now meets the gate. France still needs additional valid results
after provider rate limits interrupted collection. Northern Ireland needs more
explicitly published XI identifiers: GB identifiers are not converted, and branch
identifiers are never truncated to create candidates.

Accepted coverage for Bulgaria, Malta and Portugal uses reviewed cases with
explicit exclusions. Ambiguous observations and legacy records without matching
journals remain preserved outside the accepted sample. Passing the sample gate
does not mean every retained record is unambiguous.

These are verification milestones for the working source, not a guarantee of
current provider availability or a new release. Offline regression tests cover
all supported prefixes; further live evidence is still required for FR and XI.

BULSTAT verification remains limited in entity scope; natural persons and
thirteen-digit branches need broader coverage. VIES identity and address data
may be withheld or only partly parseable. A successful VAT check does not mean
that every address component can be extracted.

### Polish national verification status

Polish KRS P/S lookup meets the reviewed 200-distinct-ID national verification
gate, including active and restricted lifecycle states and confirmed removals.
All accepted cases have live source observations, reviewed expectations and
passing offline replay. Historical failures remain in local evidence; no access
or lifecycle blocker remains in the accepted Polish sample. This is a verification
milestone for KRS P/S, not a CEIDG coverage claim.
See the [Polish output examples](docs/examples/pl.md).

### Remaining work

The target is national lookup across all 30 EEA countries using public sources
without requiring callers to supply registry accounts or API keys. Additional
adapters on `main` still need verification and release review. Pending countries
are listed in the support table; implementation and publication dates are not promised.

| Priority | Work |
| --- | --- |
| 1 | Verify Austria, Belgium and Czechia; investigate national lookup for Spain. |
| 2 | Verify Greek national lookup alongside VIES behavior. |
| 3 | Extend Romanian coverage beyond the current ANAF taxpayer source. |

Polish registry extensions (TODO; not yet implemented):

- [ ] Add **CEIDG** lookup for sole traders, including identifier validation,
  lifecycle mappings and reviewed positive/negative replay cases.
- [ ] Investigate **KRZ and legacy RDN** debtor/insolvency coverage. Confirm official
  access, authentication and rate limits; keep insolvency facts distinct from
  company closure. KRZ replaced RDN for modern insolvency registration.

Other countries have no assigned priority. Optional credential-based sources may
later add coverage or richer data, but no such integration is implemented today
and credentials are not required for the default lookup path.

## Development

Start with the [contributor guide](https://github.com/medentic/eu-company-lookup/blob/main/CONTRIBUTING.md)
for offline tests and checks, or the [documentation index](https://github.com/medentic/eu-company-lookup/blob/main/docs/README.md)
for provider and maintainer guides. Live examples contact providers; collected
records and development evidence stay local-only and outside the published crate.

## Further reading

- [Company fields and provider mappings](docs/reference/company-result.md), including
  the [Czech ARES adapter](docs/reference/company-result.md#czech-ares-extension).
- [VIES identifiers and address behavior](docs/reference/vies.md).
- [Error catalog, raw diagnostics and 3.x migration](docs/reference/errors.md).
- [Compatibility and migration from earlier releases](docs/reference/api-stability.md).

## Credits

The local VAT checksum checks build on the country-specific validation work in
[eu-vat](https://github.com/cipansor/crateworks/tree/main/crates/eu-vat) by
Yayasan Pesantren Cipansor and contributors. Our checksum arithmetic is adapted
from version 0.1.11, with Apache-2.0 attribution retained in
[THIRD-PARTY-LICENSE.txt](THIRD-PARTY-LICENSE.txt); eu-vat is not a runtime dependency.

Thanks also to Arthur de Jong and the
[python-stdnum contributors](https://arthurdejong.org/python-stdnum/) for their
national identifier rules and independent implementation, used to cross-check
our synthetic checksum examples. Local checks do not establish VAT registration;
that still requires a matched VIES response.

## License

MIT with BSD-3-Clause address-rule material and Apache-2.0 checksum arithmetic;
the package license expression is `MIT AND BSD-3-Clause AND Apache-2.0`. See [LICENSE](https://github.com/medentic/eu-company-lookup/blob/main/LICENSE) and
[THIRD-PARTY-LICENSE.txt](https://github.com/medentic/eu-company-lookup/blob/main/THIRD-PARTY-LICENSE.txt). These licenses cover the code;
provider services and data remain subject to their own terms.
