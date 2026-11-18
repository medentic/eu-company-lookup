# France: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **API Recherche Entreprises**. These outputs describe the 5.0 API. Implementation and reviewed live coverage remain separate.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("001234567", "FR").await;
```

Nine SIREN digits. Fourteen-digit SIRET establishment numbers and VAT IDs are not accepted as SIREN. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic company administrative status `A` maps to `active`; legal-form code `5710` maps to its French label. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "FR",
  "taxId": "001234567",
  "vatRegistered": null,
  "vatId": null,
  "source": "FrenchRechercheEntreprises",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic SAS",
  "legalForm": "SAS, société par actions simplifiée",
  "registeredOffice": {
    "fullAddressByRegistry": " 1 RUE EXEMPLE\n00100 VILLE ",
    "address": "1 RUE EXEMPLE",
    "address2": null,
    "city": "VILLE",
    "postcode": "00100",
    "region": null,
    "municipality": null,
    "country": null,
    "countryISO2": null
  },
  "fiscalAddress": null,
  "status": "active",
  "registryDetails": {
    "provider": "FrenchRechercheEntreprises",
    "data": {
      "nativeRegistryStatus": "A",
      "siren": "001234567",
      "nomComplet": "Synthetic SAS",
      "sirenConserve": null,
      "nomRaisonSociale": null,
      "sigle": null,
      "activitePrincipale": null,
      "categorieEntreprise": null,
      "dateCreation": null,
      "dateFermeture": null,
      "dateMiseAJour": null,
      "dateMiseAJourInsee": null,
      "dateMiseAJourRne": null,
      "etatAdministratif": "A",
      "natureJuridique": "5710",
      "statutDiffusion": null,
      "siege": {
        "siret": "00123456700001",
        "adresse": " 1 RUE EXEMPLE\n00100 VILLE ",
        "numeroVoie": "1",
        "indiceRepetition": null,
        "typeVoie": "RUE",
        "libelleVoie": "EXEMPLE",
        "complementAdresse": null,
        "distributionSpeciale": null,
        "codePostal": "00100",
        "libelleCommune": "VILLE",
        "commune": null,
        "cedex": null,
        "libelleCedex": null,
        "codePaysEtranger": null,
        "libellePaysEtranger": null,
        "libelleCommuneEtranger": null,
        "departement": null,
        "region": "11",
        "etatAdministratif": null,
        "dateCreation": null,
        "dateFermeture": null,
        "statutDiffusionEtablissement": null,
        "estSiege": true
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
    "provider": "FrenchRechercheEntreprises",
    "country": "FR",
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
| Public coverage | Non-diffusible entities and rejected registrations are outside this source; an empty search is not a definitive miss. |
| Headquarters identity | A supplied headquarters must have `est_siege: true`; a supplied SIRET must match the full SIREN prefix. |
| Company vs establishment | Company status drives lifecycle; establishment status does not replace it. |
| Address country | No domestic country or region name is invented from the jurisdiction or INSEE region code. Original headquarters text is preserved. |

## Example basis

The success output is serialized through the current decoder and common-result mapping from the `source` fixture in the [synthetic tests](../../src/fr/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/fr/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.
