# Lookup output examples

[Main README](../../README.md) · [Company fields](../reference/company-result.md) · [Error reference](../reference/errors.md)

Explore complete success and error JSON plus provider-specific edge cases. There is one page for each implemented national adapter and one for [VIES](vies.md), whose result shape is shared across its 28 supported VAT prefixes. Countries with no national adapter are listed in the [main support table](../../README.md#country-support); they have no national success output to demonstrate.

Version 5.1 adds a [printable VIES result](vies.md#printable-evidence) with a consultation number.

## Choose an example

| Country | Company country code | National source | Availability |
| --- | --- | --- | --- |
| [Austria](at.md) | `AT` | Firmenbuch | 5.0 |
| [Belgium](be.md) | `BE` | NBB Consult | 5.0 |
| [Bulgaria](bg.md) | `BG` | Commercial Register / BULSTAT | 5.0 |
| [Czechia](cz.md) | `CZ` | ARES | 5.0 |
| [Estonia](ee.md) | `EE` | RIK public search | 5.0 |
| [Finland](fi.md) | `FI` | PRH YTJ | 5.0 |
| [France](fr.md) | `FR` | API Recherche Entreprises | 5.0 |
| [Greece](gr.md) | `GR` | GEMI | 5.0 |
| [Ireland](ie.md) | `IE` | CRO open data | 5.0 |
| [Iceland](is.md) | `IS` | Skatturinn pages and free registration overviews | 5.0 |
| [Lithuania](lt.md) | `LT` | JAR open data | 5.0 |
| [Latvia](lv.md) | `LV` | Registry open data | 5.0 |
| [Norway](no.md) | `NO` | Enhetsregisteret | 5.0 |
| [Poland](pl.md) | `PL` | KRS current extracts and confirmed removals | 5.0 |
| [Romania](ro.md) | `RO` | ANAF taxpayer lookup | 5.0 |
| [Slovakia](sk.md) | `SK` | RÚZ accounting entities | 5.0 |

[VIES positive, negative, withheld-details and service-failure examples](vies.md) cover the separate VAT operation, including EL/GR and XI/GB behavior.

## Reading the examples

- All company names, identifiers and records in this gallery are synthetic test data. Do not use the sample identifiers as a live collection queue or treat generated record links as real-company references.
- Outputs describe the 5.0 contract, including lifecycle enforcement, proceeding states and VIES checksum preflight. The [API contract](../reference/api-stability.md) explains version boundaries.
- Success JSON is the serialized inner result, not a Rust `Result` wrapper. Error JSON is the serialized `LookupError`. The library does not choose an HTTP status for your application.
- A matched closed company or a company in proceedings can be a successful lookup. A VIES `valid: false` is also a successful response with a negative VAT conclusion. A company `not_found` is an error with a narrowly defined source scope.
- Missing optional facts are explicit nulls. `vatRegistered: null` means unknown; it is not false. Original source text and leading zeroes are preserved. Native provider keys can use a different naming style from common fields.
- Missing or ambiguous lifecycle on main returns `unresolved_data`. Provider failures, mismatches and unresolved records never prove nonregistration. Only documented definitive misses become `not_found`.
- The examples are serialized from synthetic fixtures and source mappings offline. They are not retained real responses and do not establish current availability or complete country verification. No production calls are required to review them.

## Serialize success and failure

This complete program uses 5.0 APIs. Running it contacts a provider; substitute an identifier you are authorized to look up. The synthetic output shown in the country pages is illustrative and is not promised for this live call.

```rust,no_run
use eu_company_lookup::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new()?;
    match client.lookup_company("131129282", "BG").await {
        Ok(company) => println!("{}", serde_json::to_string_pretty(&company)?),
        Err(error) => println!("{}", serde_json::to_string_pretty(&error)?),
    }
    Ok(())
}
```

Use the dependencies in [Install](../../README.md#install). For VAT, use `client.lookup_vat(...)` in the same match. Reuse the client across calls. Match `error.id` in application logic; do not parse its English message. Raw diagnostics are off by default, so the example errors have `details.debug: null`.

## Unresolved lifecycle example

A matched Latvian row with a blank closure marker, no termination date and no valid registration date remains unresolved even after complete empty supplemental lifecycle responses. Main returns this complete error instead of a partial successful company or `not_found`:

```json
{
  "id": "unresolved_data",
  "title": "Company status could not be resolved",
  "description": "The registry record matched, but its lifecycle is missing, unsupported or ambiguous. This is not a negative registration result.",
  "isServiceUnavailable": false,
  "details": {
    "provider": "LatvianOpenData",
    "country": "LV",
    "reason": "unmapped_or_ambiguous_lifecycle",
    "nativeRegistryStatus": null,
    "upstreamCode": null,
    "upstreamHttpStatus": 200,
    "responseContentType": "application/json",
    "retryAfter": null,
    "retryAfterSeconds": null,
    "scope": null,
    "operation": null,
    "identifierScheme": null,
    "debug": null
  }
}
```

Absent lifecycle evidence uses reason `missing_lifecycle`; present but unsupported or contradictory evidence uses `unmapped_or_ambiguous_lifecycle`, with native status retained when available. Other optional fields may still be null on a successful, lifecycle-resolved record.

## Maintaining examples

Keep JSON aligned with the provider's synthetic tests, decoder, mapping and shared serializer. When changing a field or lifecycle rule, regenerate affected outputs offline and review their meaning; do not replace them with collected production records. Update version notes when the exact candidate is released. Keep the gallery linked from the main README and included in documentation/distribution checks.

Offline `cargo test --locked documented_` checks the gallery against synthetic
provider fixtures, current mappings and serializers, including negative/failure
HTTP paths. These checks make no live calls and do not establish country coverage.

Each national provider test module contains
`documented_company_output_matches_synthetic_source`, which constructs the exact
fixture variant used by its positive example. Additional transport checks cover
the gallery errors, VIES variants and BULSTAT fallback. Missing required wire
fields fail decoding before the lifecycle success check.
