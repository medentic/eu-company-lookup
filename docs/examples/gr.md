# Greece: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **GEMI**. These outputs describe the 5.0 API. Implementation and reviewed live coverage remain separate.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("001234567890", "GR").await;
```

Twelve GEMI digits. A nine-digit AFM tax number is not a GEMI registration number. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic identity/address fixture adds explicit status `ΕΝΕΡΓΗ`, producing `active`. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "GR",
  "taxId": "001234567890",
  "vatRegistered": null,
  "vatId": null,
  "source": "GreekGemi",
  "registryUrl": "https://publicity.businessportal.gr/company/001234567890",
  "foreignName": "Registered Latin Name",
  "companyName": "Source",
  "legalForm": null,
  "registeredOffice": {
    "fullAddressByRegistry": " Official ",
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
  "status": "active",
  "registryDetails": {
    "provider": "GreekGemi",
    "data": {
      "nativeRegistryStatus": "ΕΝΕΡΓΗ",
      "taxIdentifier": "012345678",
      "legalTypeCode": null,
      "statusCode": null,
      "startDate": null,
      "gemiRegistrationDate": null,
      "mapAddress": "Map",
      "isBranch": null,
      "isForeignBranch": null
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
    "provider": "GreekGemi",
    "country": "GR",
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
| Country and VAT prefix | National lookup uses `GR`; VIES uses `EL`. Neither the GEMI code nor native AFM alone proves VAT registration. |
| Names | `foreignName` is the separately registered Latin-script name, not an automatic translation. |
| Addresses | Use the official registered-office text; keep the shorter map address separately in details. |
| Lifecycle | Reviewed Greek labels distinguish active, closed, liquidation and bankruptcy; missing or unsupported labels return `unresolved_data`. |

## Example basis

The success output is serialized through the current decoder and common-result mapping from `shared_result_uses_official_address_and_registered_latin_name` plus an explicit active label in the [synthetic tests](../../src/gr/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/gr/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.
