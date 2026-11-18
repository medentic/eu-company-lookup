# Austria: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **Firmenbuch**. These outputs describe the 5.0 API. Implementation and reviewed live coverage remain separate.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("00123A", "AT").await;
```

One to six digits and a final letter; preserve leading zeroes. Do not include `FN` or a VAT prefix. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic detail has source status `ACTIVE` and no legal-form label. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "AT",
  "taxId": "00123A",
  "vatRegistered": null,
  "vatId": null,
  "source": "AustrianFirmenbuch",
  "registryUrl": null,
  "foreignName": null,
  "companyName": " Example GmbH ",
  "legalForm": null,
  "registeredOffice": {
    "fullAddressByRegistry": null,
    "address": "Source 1",
    "address2": null,
    "city": "Town",
    "postcode": "0010",
    "region": null,
    "municipality": null,
    "country": null,
    "countryISO2": null
  },
  "fiscalAddress": null,
  "status": "active",
  "registryDetails": {
    "provider": "AustrianFirmenbuch",
    "data": {
      "nativeRegistryStatus": "ACTIVE",
      "recordId": "00123a_7",
      "domicile": null,
      "sourceDate": null,
      "legalForm": null,
      "registeredOffice": {
        "street": "Source 1",
        "zipCode": "0010",
        "city": "Town"
      }
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
    "provider": "AustrianFirmenbuch",
    "country": "AT",
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
| All-status search | Explicit `state=ALL` includes deleted entries; empty or ambiguous results remain errors. |
| Unknown source status | Returns `unresolved_data`; `DELETED` maps to `closed` when present in a matched detail. |
| Address country | Street, city and postcode are supplied; country and original full-address text stay null. |
| VAT | Firmenbuch registration alone leaves both VAT fields null. |

## Example basis

The success output is serialized through the current decoder and common-result mapping from the `DETAIL` fixture in the [synthetic tests](../../src/at/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/at/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.

## Matched deleted company

The same synthetic company with native `DELETED` is a resolved record with
`status: "closed"`. The all-status search makes this record reachable without
using a failed or empty search as evidence of closure.

```json
{
  "country": "AT",
  "taxId": "00123A",
  "vatRegistered": null,
  "vatId": null,
  "source": "AustrianFirmenbuch",
  "registryUrl": null,
  "foreignName": null,
  "companyName": " Example GmbH ",
  "legalForm": null,
  "registeredOffice": {
    "fullAddressByRegistry": null,
    "address": "Source 1",
    "address2": null,
    "city": "Town",
    "postcode": "0010",
    "region": null,
    "municipality": null,
    "country": null,
    "countryISO2": null
  },
  "fiscalAddress": null,
  "status": "closed",
  "registryDetails": {
    "provider": "AustrianFirmenbuch",
    "data": {
      "nativeRegistryStatus": "DELETED",
      "recordId": "00123a_7",
      "domicile": null,
      "sourceDate": null,
      "legalForm": null,
      "registeredOffice": {
        "street": "Source 1",
        "zipCode": "0010",
        "city": "Town"
      }
    }
  }
}
```
