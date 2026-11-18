# Latvia: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **Registry open data**. These outputs describe the 5.0 API. Implementation and reviewed live coverage remain separate.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("00123456789", "LV").await;
```

Eleven registration-number digits, preserving leading zeroes. SEPA creditor codes are not VAT IDs. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic fixture adds `terminated: "2020-01-01"`, establishing `closed`. Its blank native closed marker is preserved. The resolved closure needs no supplemental lifecycle queries, so `registryDetails.data.lifecycle` is null. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "LV",
  "taxId": "00123456789",
  "vatRegistered": null,
  "vatId": null,
  "source": "LatvianOpenData",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic SIA",
  "legalForm": "Source legal form",
  "registeredOffice": {
    "fullAddressByRegistry": " Source legal address ",
    "address": null,
    "address2": null,
    "city": null,
    "postcode": null,
    "region": null,
    "municipality": null,
    "country": null,
    "countryISO2": null
  },
  "fiscalAddress": null,
  "status": "closed",
  "registryDetails": {
    "provider": "LatvianOpenData",
    "data": {
      "nativeRegistryStatus": null,
      "lifecycle": null,
      "regcode": "00123456789",
      "name": "Synthetic SIA",
      "sepa": "Unrelated creditor code",
      "regtype": null,
      "regtype_text": null,
      "type_text": "Source legal form",
      "registered": null,
      "terminated": "2020-01-01",
      "closed": " ",
      "address": " Source legal address ",
      "reregistration_term": null,
      "index": 1001,
      "addressid": null,
      "region": null,
      "city": null,
      "atvk": null
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
    "provider": "LatvianOpenData",
    "country": "LV",
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
| No termination date | A blank closed marker alone does not imply active. Complete supplemental lifecycle evidence and a valid registration date can resolve active or supported proceedings; unresolved evidence returns `unresolved_data`. |
| Supplemental lifecycle | Non-closed records can require three additional, identity-checked liquidation, suspension and insolvency dataset requests under the same lookup deadline. Native observations appear in `registryDetails.data.lifecycle`. |
| Unknown marker | A nonblank unverified closed marker alone cannot establish a resolved lifecycle. |
| Address | Keep the original legal address. Native `index` and location codes do not become guessed structured components. |
| Dataset result | Zero or ambiguous rows are response mismatches, not definitive registry absence. |

## Example basis

The success output is serialized through the current decoder and common-result mapping from the `source` fixture with the lifecycle change described above in the [synthetic tests](../../src/lv/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/lv/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.
