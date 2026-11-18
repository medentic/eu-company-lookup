# Lithuania: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **JAR open data**. These outputs describe the 4.1 API. Implementation and reviewed live coverage remain separate.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("001234567", "LT").await;
```

Nine modern company-code digits; legacy seven-digit codes are outside this adapter. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic record has `isreg_data: "2020-01-01"`, establishing `closed` even though its status reference is opaque. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "LT",
  "taxId": "001234567",
  "vatRegistered": null,
  "vatId": null,
  "source": "LithuanianJar",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic UAB",
  "legalForm": null,
  "registeredOffice": {
    "fullAddressByRegistry": " Source address ",
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
    "provider": "LithuanianJar",
    "data": {
      "nativeRegistryStatus": "opaque-status",
      "_type": "datasets/gov/rc/jar/iregistruoti/JuridinisAsmuo",
      "_id": "synthetic-source-id",
      "_revision": null,
      "ja_kodas": "001234567",
      "ja_pavadinimas": "Synthetic UAB",
      "pilnas_adresas": " Source address ",
      "adresas": null,
      "forma": null,
      "statusas": {
        "_id": "opaque-status"
      },
      "reg_data": null,
      "isreg_data": "2020-01-01",
      "stat_data": null
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
    "provider": "LithuanianJar",
    "country": "LT",
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
| Dataset names | Preserve the published name; some entity types can have anonymized names. |
| Status references | Reviewed static status references can establish active, closed or proceedings. Unrecognized references are preserved as native status; they remain unresolved unless independent closure evidence resolves lifecycle. |
| Dates | Missing deregistration does not imply active; a malformed supplied cessation date blocks success. |
| Address and legal form | Keep full registration-address text. Unresolved address/form references do not become invented common components or labels. |

## Example basis

The success output is serialized through the current decoder and common-result mapping from the `source` fixture in the [synthetic tests](../../src/lt/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/lt/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.
