# Iceland: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **Skatturinn company pages**. These outputs describe the 5.0 API. Implementation and reviewed live coverage remain separate.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("4101000000", "IS").await;
```

Ten legal-entity kennitala digits with the first two in 41–71. Hyphens, personal IDs and VAT prefixes are unsupported. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic page adds `Félag afskráð 13.12.2018` to its registration subtitle, establishing `closed`. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "IS",
  "taxId": "4101000000",
  "vatRegistered": null,
  "vatId": null,
  "source": "IcelandicSkatturinn",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic ehf.",
  "legalForm": "Einkahlutafélag (ehf)",
  "registeredOffice": {
    "fullAddressByRegistry": " Legal & road  200 Town ",
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
    "provider": "IcelandicSkatturinn",
    "data": {
      "nativeRegistryStatus": "Stofnað/Skráð: 01.01.2000(Félag afskráð 13.12.2018)",
      "kennitala": "4101000000",
      "registrationText": "Stofnað/Skráð: 01.01.2000(Félag afskráð 13.12.2018)",
      "postalAddress": " Postal  100 City ",
      "legalAddress": " Legal & road  200 Town ",
      "municipalityText": "1000Town",
      "legalFormText": "E1 Einkahlutafélag (ehf)",
      "legalFormCode": "E1",
      "legalFormDescription": "Einkahlutafélag (ehf)"
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
    "provider": "IcelandicSkatturinn",
    "country": "IS",
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
| Registration date only | A founding/registration date alone does not establish lifecycle. The identity-bound free overview can resolve it; without an explicit recognized status, lookup returns `unresolved_data`. |
| Closure wording | Only the exact company deregistration notice with a valid date establishes closure. VAT removal text or unrelated page text does not. |
| Legal vs postal address | `Lögheimili` supplies original registered-office text. Keep postal address and municipality in native details; structured office components stay null. |
| VIES | Iceland is outside VIES. Absence of a VAT table is not a domestic VAT-negative observation. |

## Example basis

The success output is serialized through the current decoder and common-result mapping from the `source` fixture with the lifecycle change described above in the [synthetic tests](../../src/is/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/is/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.

## Active registration from the free overview

When the public page has no lifecycle status, the client reads the free registration
overview offered for that identifier. Its explicit `Virk skráning` status establishes
active registration. The document date, identifier and decoded status remain in
native details. PDF text extraction may omit visual spaces in the status text.

```json
{
  "country": "IS",
  "taxId": "4101000000",
  "vatRegistered": null,
  "vatId": null,
  "source": "IcelandicSkatturinn",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic ehf.",
  "legalForm": "Einkahlutafélag (ehf)",
  "registeredOffice": {
    "fullAddressByRegistry": " Legal & road  200 Town ",
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
    "provider": "IcelandicSkatturinn",
    "data": {
      "nativeRegistryStatus": "Virk skráning",
      "kennitala": "4101000000",
      "registrationText": "Stofnað/Skráð: 01.01.2000",
      "postalAddress": " Postal  100 City ",
      "legalAddress": " Legal & road  200 Town ",
      "municipalityText": "1000Town",
      "legalFormText": "E1 Einkahlutafélag (ehf)",
      "legalFormCode": "E1",
      "legalFormDescription": "Einkahlutafélag (ehf)",
      "registrationOverview": {
        "kennitala": "4101000000",
        "issuedOn": "2026-11-18",
        "nativeStatus": "Virk skráning"
      }
    }
  }
}
```

This synthetic example is checked through the PDF decoder, common mapping and
serializer in the country tests. Unknown document statuses and failed document
requests remain errors through the public company lookup.
