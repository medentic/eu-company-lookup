# Estonia: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **RIK public search**. These outputs describe the 4.1 API. Implementation and reviewed live coverage remain separate.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("00123456", "EE").await;
```

Eight registry-code digits, preserving leading zeroes. VAT and personal identifiers are separate. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic result has status `R` and legal form `5`, mapped to `active` and `Osaühing`. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "EE",
  "taxId": "00123456",
  "vatRegistered": null,
  "vatId": null,
  "source": "EstonianRik",
  "registryUrl": "https://ariregister.rik.ee/eng/company/00123456/Synthetic",
  "foreignName": null,
  "companyName": "Synthetic OÜ",
  "legalForm": "Osaühing",
  "registeredOffice": {
    "fullAddressByRegistry": "Original address\nSecond line",
    "address": null,
    "address2": null,
    "city": null,
    "postcode": "00100",
    "region": null,
    "municipality": null,
    "country": null,
    "countryISO2": null
  },
  "fiscalAddress": null,
  "status": "active",
  "registryDetails": {
    "provider": "EstonianRik",
    "data": {
      "nativeRegistryStatus": "R",
      "companyId": 123,
      "regCode": "00123456",
      "name": "Synthetic OÜ",
      "historicalNames": [
        "Previous OÜ"
      ],
      "status": "R",
      "legalAddress": "Original address\nSecond line",
      "zipCode": "00100",
      "legalForm": "5",
      "url": "https://ariregister.rik.ee/eng/company/00123456/Synthetic"
    }
  }
}
```

## Negative case and failure boundary

This adapter does not expose a definitive `not_found` outcome. For example, upstream HTTP 404 produces the error below, not a negative registration result. Empty/ambiguous or malformed source responses also remain errors, with the exact classification depending on the response.

```json
{
  "id": "unexpected_response",
  "title": "Verification could not be completed",
  "description": "An unexpected HTTP status, malformed response, changed schema or unrecognized fault prevented verification.",
  "isServiceUnavailable": false,
  "details": {
    "provider": "EstonianRik",
    "country": "EE",
    "reason": null,
    "nativeRegistryStatus": null,
    "upstreamCode": null,
    "upstreamHttpStatus": 404,
    "responseContentType": "text/plain",
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
| Search identity | Exactly one full-code match is required; the first search hit is not sufficient. |
| Other lifecycle states | `K` maps to closed, `L` to liquidation and `N` to bankruptcy. Unsupported states return `unresolved_data`. |
| Partial address | Original legal address and postcode are retained; no city, street or country is inferred here. |
| Record link | Only a validated official portal URL bound to the exact company code becomes `registryUrl`. The synthetic URL below is not a real-company reference. |

## Example basis

The success output is serialized through the current decoder and common-result mapping from the `source` fixture in the [synthetic tests](../../src/ee/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/ee/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.
