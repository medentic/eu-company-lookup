# eu-company-lookup

Rust library for EU VAT syntax validation and live verification through the European Commission's VIES service. Returns typed results, optional source-provided owner details, structured addresses and explicit errors.

**0.1.0 release candidate.** VAT lookup is implemented; crates.io publication is pending. National company registries, BULSTAT, D-U-N-S and verification reports are future work. There is no full-registry download or database requirement.

## Install

Requires **Rust 1.98.1+** and a **Tokio runtime with I/O and time enabled**.

Before publication, use the Git repository (pin a reviewed commit with `rev` for reproducible application builds):

```toml
[dependencies]
eu-company-lookup = { git = "https://github.com/medentic/eu-company-lookup", branch = "main" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

After 0.1.0 is published, the dependency becomes `eu-company-lookup = "0.1"`.

## Check a VAT ID

```no_run
use eu_company_lookup::{Client, LookupError, validate_vat_syntax};

#[tokio::main]
async fn main() -> Result<(), LookupError> {
    // Synthetic example: syntax acceptance does not establish registration.
    let vat = validate_vat_syntax("bg 123456789")?;
    let client = Client::new()?;
    let check = client.lookup_vat(vat.as_str()).await?;

    println!("{}: valid={}", check.vat_id, check.valid);
    println!("Company: {:?}; address: {:?}", check.company, check.address);
    Ok(())
}
```

Reuse and clone one client across application requests. `is_vat_valid(vat_id)` returns `Result<bool, LookupError>` when only validity is needed. Do not call it and `lookup_vat` for the same check: both make a VIES request.

| Operation | Behavior |
| --- | --- |
| `validate_vat_syntax(value)` / `VatId::parse(value)` | Offline normalization and conservative national-format check; no checksum verification. |
| `client.lookup_vat(value)` | VIES result, checked identifier, company name and original/structured address. |
| `client.is_vat_valid(value)` | Same lookup, returning only its validity flag. |
| `client.capabilities()` | Local catalog grouped by jurisdiction and provider, with operations, scheme, checksum coverage and language behavior. No health probe. |

Inputs require a prefix. ASCII whitespace is removed and letters become uppercase; leading zeroes are preserved. Other punctuation is not stripped (the Irish `*` and `+` formats are supported). Non-ASCII input and inputs over 64 bytes are rejected. Deserializing `VatId` also validates it.

## Coverage and capabilities

VIES prefixes: **AT BE BG CY CZ DE DK EE EL ES FI FR HR HU IE IT LT LU LV MT NL PL PT RO SE SI SK XI**.

Greece uses VAT prefix **EL**, mapped to country **GR**. **XI** maps to **GB / GB-NIR** (Northern Ireland); it does not imply general UK or EU membership coverage. GB, GR and CH are not accepted VAT prefixes by this adapter.

```rust
use eu_company_lookup::{Client, LanguageSupport};

let client = Client::new()?;
let catalog = client.capabilities();
let greece = catalog.iter().find(|c| c.country_code == "GR").unwrap();
assert_eq!(greece.vat_prefix, "EL");
assert_eq!(greece.providers[0].languages, LanguageSupport::NotSelectable);
# Ok::<(), eu_company_lookup::LookupError>(())
```

VIES `checkVat` has **no language-selection parameter**. Company names and original addresses remain in the source language. Every serialized result has exactly this shape:

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
    "country": null
  }
}
```

`fullAddressByVies` preserves the decoded source text and line breaks, even when empty, withheld or unparseable; absent/nil elements are null. `address` is the first delivery line, including building details. `address2` contains remaining delivery lines and any parsed locality, joined with newlines. Uncertain components are null. `country` is populated only from explicit country text, never the VAT prefix. Parsing is heuristic, not address verification, and never changes VAT validity. Company and address values reflect what the matching source response supplied, even on a negative check; they are not proof of registration.

Conservative splitting rules, reused from Medentic, cover AT, BE, BG, CY, CZ, DK, EE, EL, FI, FR, IT, LT, LU, NL, PL, PT, SE, SI and SK. Unsupported countries/formats still return the original address, with null structured fields. A format match is not a promise that every real address can be split.

Source/observation timestamps remain private and are excluded from serialization. Parsing diagnostics are not part of the response. `VatCheck` implements Serde serialization.

National-format acceptance does not prove checksum validity, registration, company existence or entitlement to a particular tax treatment. VIES reports registration for intra-EU transactions. See the [official VIES explanation](https://europa.eu/youreurope/business/finance-and-tax/vat/check-vat-number-vies/index_en.htm), [SOAP contract](https://ec.europa.eu/taxation_customs/vies/checkVatService.wsdl) and [VAT format guidance](https://www.gov.uk/guidance/vat-on-movements-of-goods-between-northern-ireland-and-the-eu#vat-registration-numbers-format).

## Failures and traffic limits

`false` means an actual negative, identifier-matched response. Input errors, unavailable countries/services, SOAP faults, HTTP errors, rate limits, malformed/mismatched responses and deadlines are separate errors. Error text does not include the submitted VAT number, owner details or arbitrary provider body. Unknown SOAP faults remain errors with sanitized text.

Default settings:

- Eight-second total deadline, including admission spacing, HTTP and response reading; three-second connection timeout.
- Two admitted requests globally and one per VAT jurisdiction. Excess concurrent calls return `Busy` immediately.
- Two seconds between global starts and five seconds between starts for one jurisdiction. Waiting uses the total deadline.
- Sixty-second cooldown after provider capacity or unavailability errors, extended by a longer valid Retry-After value. Limits and cooldowns are shared across clones; coordinate separate processes in the application.
- 64 KiB response limit, including streamed bodies. No automatic retries, redirects, caching, cookies or ambient proxy configuration.

These are conservative client defaults, not a promised VIES quota. Configure them with `Client::with_config(ClientConfig { ... })`. HTTP Retry-After supports seconds and dates. Extreme values are preserved in errors; internal cooldown arithmetic is capped at ten years. Caller cancellation releases active slots; a request already sent can still be processing upstream.

## Development and testing

Source-owned tests and synthetic fixtures live under `src/vies/`; `tests/public_api.rs` exercises the consumer interface. Normal tests use a local fake HTTP server and never query VIES.

```sh
cargo fmt --all --check
cargo test --locked --all-targets
cargo test --locked --doc
cargo clippy --locked --all-targets -- -D warnings
cargo package --locked
```

One explicitly requested live check:

```sh
VIES_LIVE_VAT='<your-authorized-VAT-ID>' cargo test --locked live_vies -- --ignored --nocapture
```

The ignored `live_address_split` test additionally accepts `VIES_EXPECTED_ADDRESS_JSON`: an object of independently reviewed address fields to assert against a live, valid result. It prints the observation for manual review; run it only in an appropriate environment.

A live outage fails this test; it is not counted as a successful verification. Live tests are excluded from CI. `cargo run --example check_vat -- '<VAT-ID>'` prints the full returned observation, including owner data when available; use an appropriate terminal/environment.

## License and scope

MIT, copyright Medentic. Address rules include BSD-3-Clause material; see THIRD-PARTY-LICENSE.txt. The library license does not change the source service's terms. Use VIES for its intended VAT-verification purpose and observe its [service conditions](https://ec.europa.eu/taxation_customs/vies/checkVatService.wsdl). The package contains synthetic fixtures, not collected company datasets.
