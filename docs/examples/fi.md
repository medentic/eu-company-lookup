# Finland: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **PRH YTJ**. These outputs describe the 5.0 API. Implementation and reviewed live coverage remain separate.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("0012345-6", "FI").await;
```

Seven digits, a hyphen, and one digit. Keep the hyphen and leading zeroes; FI VAT numbers are separate. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic Trade Register status `4` maps to `closed`. The latest name remains usable even with a name end date. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "FI",
  "taxId": "0012345-6",
  "vatRegistered": null,
  "vatId": null,
  "source": "FinnishPrh",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic Oy",
  "legalForm": "Osakeyhtiö",
  "registeredOffice": null,
  "fiscalAddress": null,
  "status": "closed",
  "registryDetails": {
    "provider": "FinnishPrh",
    "data": {
      "nativeRegistryStatus": "4",
      "businessId": {
        "value": "0012345-6",
        "source": "3",
        "registrationDate": null
      },
      "euId": null,
      "names": [
        {
          "name": "Synthetic Oy",
          "type": "1",
          "version": 1,
          "source": "1",
          "registrationDate": null,
          "endDate": "2020-01-01"
        }
      ],
      "mainBusinessLine": null,
      "website": null,
      "companyForms": [
        {
          "type": "16",
          "descriptions": [
            {
              "languageCode": "1",
              "description": "Osakeyhtiö"
            }
          ],
          "version": 1,
          "source": "1",
          "registrationDate": null,
          "endDate": null
        }
      ],
      "companySituations": [],
      "registeredEntries": [],
      "addresses": [
        {
          "type": 2,
          "source": "1",
          "street": null,
          "postCode": "00100",
          "postOffices": [],
          "postOfficeBox": null,
          "buildingNumber": null,
          "entrance": null,
          "apartmentNumber": null,
          "apartmentIdSuffix": null,
          "co": null,
          "country": null,
          "freeAddressLine": "ORIGINAL_text",
          "registrationDate": null
        }
      ],
      "tradeRegisterStatus": "4",
      "status": null,
      "registrationDate": null,
      "endDate": null,
      "lastModified": null
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
    "provider": "FinnishPrh",
    "country": "FI",
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
| Names and forms | Select the unique version-1 main name and Finnish legal-form label; other names remain native details. |
| Lifecycle | Status `1` can establish active only without conflicting dissolution or ongoing-proceeding evidence. Status `2` alone does not establish closure. |
| Contact addresses | Street/postal contact roles remain in details; they do not populate common registered-office or fiscal-address fields. |
| Tax entries | Do not infer a VAT identifier from register-entry types. Missing `lastModified` is allowed. |

## Example basis

The success output is serialized through the current decoder and common-result mapping from the `source` fixture in the [synthetic tests](../../src/fi/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/fi/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.
