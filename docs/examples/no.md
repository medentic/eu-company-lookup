# Norway: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **Enhetsregisteret**. These outputs describe the 4.1 API, including resolved-lifecycle enforcement.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("001234567", "NO").await;
```

Nine organisation-number digits. Do not add `NO`, `MVA`, or replace the organisation number with a personal identifier. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic record explicitly sets `konkurs: true`, producing `bankruptcy`; it also reports domestic VAT registration. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "NO",
  "taxId": "001234567",
  "vatRegistered": true,
  "vatId": null,
  "source": "NorwegianBrreg",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic AS",
  "legalForm": "Aksjeselskap",
  "registeredOffice": null,
  "fiscalAddress": null,
  "status": "bankruptcy",
  "registryDetails": {
    "provider": "NorwegianBrreg",
    "data": {
      "nativeRegistryStatus": null,
      "respons_klasse": null,
      "underRekonstruksjonsforhandlingDato": null,
      "underUtenlandskInsolvensbehandlingDato": null,
      "paategninger": null,
      "organisasjonsnummer": "001234567",
      "navn": "Synthetic AS",
      "organisasjonsform": {
        "kode": "AS",
        "beskrivelse": "Aksjeselskap"
      },
      "forretningsadresse": {
        "land": null,
        "landkode": null,
        "postnummer": "00100",
        "poststed": null,
        "kommune": null,
        "kommunenummer": null,
        "adresse": [
          "Original line"
        ]
      },
      "postadresse": {
        "land": null,
        "landkode": null,
        "postnummer": null,
        "poststed": null,
        "kommune": null,
        "kommunenummer": null,
        "adresse": [
          "Postal line"
        ]
      },
      "registrertIMvaregisteret": true,
      "registrertIForetaksregisteret": null,
      "konkurs": true,
      "underAvvikling": null,
      "underTvangsavviklingEllerTvangsopplosning": null,
      "stiftelsesdato": null,
      "slettedato": null,
      "registreringsdatoEnhetsregisteret": null,
      "registreringsdatoForetaksregisteret": null,
      "maalform": null,
      "hjemmeside": null,
      "naeringskode1": null,
      "naeringskode2": null,
      "naeringskode3": null
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
    "provider": "NorwegianBrreg",
    "country": "NO",
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
| Domestic VAT | `vatRegistered: true` can coexist with `vatId: null`: Norway is outside VIES and the shared VatId type is VIES-specific. |
| Address roles | Business and postal addresses remain native details; neither is silently substituted for a legal registered office or fiscal address. |
| Active lifecycle | Requires the current-entity discriminator, a valid registration date, all proceeding flags explicitly false, empty annotations, and no contradictory deletion/insolvency evidence. |
| Missing or conflicting lifecycle | Returns `unresolved_data`. False or absent proceeding flags alone do not prove active status. Active requires `Enhet`, a valid registration date, three explicit false flags and empty annotations. |

## Example basis

The success output is serialized through the current decoder and common-result mapping from the `source` fixture in the [synthetic tests](../../src/no/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/no/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.
