# Romania: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **ANAF taxpayer lookup**. These outputs describe the 5.0 API, including resolved-lifecycle enforcement.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("001234", "RO").await;
```

A supported two-to-ten-digit CUI, retaining caller leading zeroes. Do not include the `RO` VAT prefix. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic ANAF fixture adds `INREGISTRAT DIN DATA 01.01.2000`. It demonstrates active registration, negative article-316 VAT registration and fiscal inactivity as separate facts. The fiscal date and checked-at time are fixed illustrative values. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "RO",
  "taxId": "001234",
  "vatRegistered": false,
  "vatId": null,
  "source": "RomanianAnaf",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "EXEMPLU SRL",
  "legalForm": null,
  "registeredOffice": null,
  "fiscalAddress": {
    "fullAddressByRegistry": "  Strada Exemplu 1\nBucurești  ",
    "address": null,
    "address2": null,
    "city": "București",
    "postcode": "012345",
    "region": null,
    "municipality": null,
    "country": "Romania",
    "countryISO2": "RO"
  },
  "status": "active",
  "registryDetails": {
    "provider": "RomanianAnaf",
    "data": {
      "nativeRegistryStatus": "INREGISTRAT DIN DATA 01.01.2000",
      "requestDate": "2025-02-14",
      "checkedAt": "2025-02-14T12:00:00Z",
      "tradeRegisterNumber": "J00/1/2000",
      "registrationDate": null,
      "registrationStatus": "INREGISTRAT DIN DATA 01.01.2000",
      "activityCode": null,
      "registeredOffice": null,
      "fiscalAddress": {
        "street": null,
        "streetNumber": null,
        "locality": "București",
        "county": null,
        "country": "Romania",
        "countryISO2": "RO",
        "postcode": "012345",
        "details": null
      },
      "taxStatus": {
        "vatRegistered": false,
        "vatOnCollection": null,
        "inactive": true,
        "splitVat": null,
        "eInvoiceRegistered": false
      }
    }
  }
}
```

## Negative case and failure boundary

An otherwise successful ANAF envelope with no found record and exactly the requested numeric CUI in `notFound` returns this error. It establishes an ANAF miss only.

```json
{
  "id": "not_found",
  "title": "No matching company record found",
  "description": "The identified registry sources definitively reported no matching company record. This does not establish VAT invalidity.",
  "isServiceUnavailable": false,
  "details": {
    "provider": "RomanianAnaf",
    "country": "RO",
    "reason": null,
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

## Edge cases

| Case | Behavior |
| --- | --- |
| VAT false vs unknown | `vatRegistered: false` is an ANAF article-316 observation. Missing evidence stays null; neither case supplies a VAT ID. |
| Positive VAT with padded input | For matched `taxId: "001234"` and article-316 true, the provider-specific VAT value is `RO1234`; the common tax ID keeps its padding. |
| Fiscal inactivity | `taxStatus.inactive: true` does not by itself close a company. Lifecycle uses supported registration wording. |
| Addresses | The fiscal address does not fill a missing registered office. Missing source country defaults to Romania / RO; explicit source country takes precedence. |
| Source and date | ANAF is a taxpayer source, not an ONRC extract. Explicit fiscal dates use `RoRegistryClient::lookup_at`; the facade uses its default date. |

## Example basis

The success output is serialized through the current decoder and common-result mapping from `fixture` plus explicit registration wording and a fixed checked-at time in the [synthetic tests](../../src/ro/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/ro/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.

A source status such as `TRANSFER(SOSIRE)` or `TRANSFER(PLECARE)` produces
`unresolved_data`, preserving the native wording in the error. Neither transfer
direction proves active or closed lifecycle. A separately dated ONRC snapshot
must not be presented as part of the ANAF response.

A manual current ONRC/BRIS check can establish lifecycle in that separate source,
but the adapter does not reuse browser evidence or substitute it into an ANAF
result. Its unattended lookup still needs its own explicit lifecycle observation.
