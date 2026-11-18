# Ireland: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **CRO open data**. These outputs describe the 5.0 API, including resolved-lifecycle enforcement.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("0123456", "IE").await;
```

One to seven company-number digits, preserving leading zeroes. CRO numbers are not VAT numbers. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic CRO record is `Dissolved`, producing a successful match with status `closed`. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "IE",
  "taxId": "0123456",
  "vatRegistered": null,
  "vatId": null,
  "source": "IrishCro",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic Limited",
  "legalForm": "Private limited by shares",
  "registeredOffice": {
    "fullAddressByRegistry": null,
    "address": " Original line ",
    "address2": null,
    "city": null,
    "postcode": "A00 A000",
    "region": null,
    "municipality": null,
    "country": null,
    "countryISO2": null
  },
  "fiscalAddress": null,
  "status": "closed",
  "registryDetails": {
    "provider": "IrishCro",
    "data": {
      "nativeRegistryStatus": "Dissolved",
      "company_num": "0123456",
      "company_name": "Synthetic Limited",
      "company_status": "Dissolved",
      "company_type": "Private limited by shares",
      "company_reg_date": null,
      "last_ar_date": null,
      "company_address_1": " Original line ",
      "company_address_2": null,
      "company_address_3": null,
      "company_address_4": null,
      "comp_dissolved_date": null,
      "nard": null,
      "last_accounts_date": null,
      "company_status_date": null,
      "eircode": "A00 A000",
      "company_name_eff_date": null,
      "company_type_eff_date": null,
      "princ_object_code": null,
      "company_status_code": null,
      "company_type_code": null,
      "nace_v2_code": null
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
    "provider": "IrishCro",
    "country": "IE",
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
| Closed is a match | Dissolved companies can be present in the dataset; closed does not mean not found. |
| Lifecycle disagreement | Conflicting status code/label pairs return `unresolved_data`; supported proceedings retain distinct statuses. |
| Registered office | Ordered company-address lines and Eircode are mapped without inventing a city, county or country. |
| Dataset scope | The fixed resource and unique exact company number must match. Zero or duplicate rows do not prove absence. |

## Example basis

The success output is serialized through the current decoder and common-result mapping from the `source` fixture in the [synthetic tests](../../src/ie/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/ie/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.
