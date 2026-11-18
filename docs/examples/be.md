# Belgium: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **NBB Consult**. These outputs describe the 5.0 API. Implementation and reviewed live coverage remain separate.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("0012345678", "BE").await;
```

Ten enterprise-number digits; preserve leading zeroes. Do not include `BE` or dotted punctuation. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic NBB legal-situation code `000` establishes `active`. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "BE",
  "taxId": "0012345678",
  "vatRegistered": null,
  "vatId": null,
  "source": "BelgianNbb",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Example",
  "legalForm": null,
  "registeredOffice": {
    "fullAddressByRegistry": null,
    "address": "Source 12A",
    "address2": "003",
    "city": null,
    "postcode": "0010",
    "region": null,
    "municipality": null,
    "country": null,
    "countryISO2": "BE"
  },
  "fiscalAddress": null,
  "status": "active",
  "registryDetails": {
    "provider": "BelgianNbb",
    "data": {
      "nativeRegistryStatus": "Normal situation",
      "registeredOffice": {
        "streetName": "Source",
        "streetNumber": "12A",
        "boxNumber": "003",
        "postalCode": "0010",
        "town": null,
        "country": null,
        "countryCode": "BE"
      },
      "language": null,
      "languageRegime": null,
      "nisCode": null,
      "legalFormCode": null,
      "legalSituationCode": "000",
      "legalSituationDate": null
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
    "provider": "BelgianNbb",
    "country": "BE",
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
| Legal situation | Reviewed codes distinguish normal, closed and proceeding states; unsupported or missing lifecycle returns `unresolved_data`. |
| Box number | The source box `003` remains `address2`; it is not dropped or converted to a number. |
| Source scope | NBB supplies CBE-derived identity data; this is not a direct CBE lookup or VAT verification. |

## Closed company with no translated status label

A recognized legal-situation code is sufficient even when NBB omits its display
label. This synthetic `014` record maps to closed and retains `014` as the native
status. It is a matched company record, not a `not_found` result. Code `012`
would instead indicate liquidation, which remains a separate lifecycle state.

```json
{
  "country": "BE",
  "taxId": "0012345678",
  "vatRegistered": null,
  "vatId": null,
  "source": "BelgianNbb",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Example",
  "legalForm": null,
  "registeredOffice": {
    "fullAddressByRegistry": null,
    "address": "Source 12A",
    "address2": "003",
    "city": null,
    "postcode": "0010",
    "region": null,
    "municipality": null,
    "country": null,
    "countryISO2": "BE"
  },
  "fiscalAddress": null,
  "status": "closed",
  "registryDetails": {
    "provider": "BelgianNbb",
    "data": {
      "nativeRegistryStatus": "014",
      "registeredOffice": {
        "streetName": "Source",
        "streetNumber": "12A",
        "boxNumber": "003",
        "postalCode": "0010",
        "town": null,
        "country": null,
        "countryCode": "BE"
      },
      "language": null,
      "languageRegime": null,
      "nisCode": null,
      "legalFormCode": null,
      "legalSituationCode": "014",
      "legalSituationDate": null
    }
  }
}
```

## Example basis

The reviewed national sample meets the 200-distinct-ID gate for active records.
The closed and proceeding examples are synthetic; this sample does not establish
live coverage of those states or a definitive NBB absence outcome.

The success output is serialized through the current decoder and common-result mapping from `shared_view_preserves_box_and_native_status_without_vat_inference` in the [synthetic tests](../../src/be/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/be/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.
