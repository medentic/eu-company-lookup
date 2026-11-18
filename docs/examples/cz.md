# Czechia: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **ARES**. These outputs describe the 5.0 API. Implementation and reviewed live coverage remain separate.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("00123456", "CZ").await;
```

Eight IČO digits, preserving leading zeroes. DIČ and VAT-group identifiers are separate. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic record adds `datumZaniku: "2020-01-01"` to the identity/address fixture. A closed company is still a matched record. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "CZ",
  "taxId": "00123456",
  "vatRegistered": null,
  "vatId": null,
  "source": "CzechAres",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Example",
  "legalForm": null,
  "registeredOffice": {
    "fullAddressByRegistry": "  Source address  ",
    "address": null,
    "address2": null,
    "city": null,
    "postcode": "00123",
    "region": null,
    "municipality": null,
    "country": null,
    "countryISO2": null
  },
  "fiscalAddress": null,
  "status": "closed",
  "registryDetails": {
    "provider": "CzechAres",
    "data": {
      "nativeRegistryStatus": null,
      "registeredOffice": {
        "kodStatu": null,
        "nazevStatu": null,
        "kodKraje": null,
        "nazevKraje": null,
        "kodOkresu": null,
        "nazevOkresu": null,
        "kodObce": null,
        "nazevObce": null,
        "kodSpravnihoObvodu": null,
        "nazevSpravnihoObvodu": null,
        "kodMestskehoObvodu": null,
        "nazevMestskehoObvodu": null,
        "kodMestskeCastiObvodu": null,
        "kodUlice": null,
        "nazevMestskeCastiObvodu": null,
        "nazevUlice": null,
        "cisloDomovni": null,
        "doplnekAdresy": null,
        "kodCastiObce": null,
        "cisloOrientacni": null,
        "cisloOrientacniPismeno": null,
        "nazevCastiObce": null,
        "kodAdresnihoMista": null,
        "psc": null,
        "textovaAdresa": "  Source address  ",
        "cisloDoAdresy": null,
        "standardizaceAdresy": null,
        "pscTxt": "00123",
        "typCisloDomovni": null
      },
      "legalFormCode": null,
      "rosLegalFormCode": null,
      "taxIdentifier": "CZ00123456",
      "vatGroupIdentifier": null,
      "createdDate": null,
      "ceasedDate": "2020-01-01",
      "updatedDate": null,
      "primarySource": null,
      "sourceRegistrations": {
        "stavZdrojeRos": null,
        "stavZdrojeVr": null,
        "stavZdrojeRes": null,
        "stavZdrojeRzp": null,
        "stavZdrojeNrpzs": null,
        "stavZdrojeRpsh": null,
        "stavZdrojeRcns": null,
        "stavZdrojeSzr": null,
        "stavZdrojeDph": "AKTIVNI",
        "stavZdrojeSkDph": null,
        "stavZdrojeSd": null,
        "stavZdrojeIr": null,
        "stavZdrojeCeu": null,
        "stavZdrojeRs": null,
        "stavZdrojeRed": null,
        "stavZdrojeMonitor": null
      }
    }
  }
}
```

## Negative case and failure boundary

HTTP 404 becomes `not_found` only with the documented `NENALEZENO` JSON error and nonblank description. An arbitrary 404 is not enough.

```json
{
  "id": "not_found",
  "title": "No matching company record found",
  "description": "The identified registry sources definitively reported no matching company record. This does not establish VAT invalidity.",
  "isServiceUnavailable": false,
  "details": {
    "provider": "CzechAres",
    "country": "CZ",
    "reason": null,
    "nativeRegistryStatus": null,
    "upstreamCode": null,
    "upstreamHttpStatus": 404,
    "responseContentType": "application/json",
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
| Active lifecycle | Requires primary source `ros`, ROS state `AKTIVNI`, and IR/CEU states explicitly `NEEXISTUJICI`; a tax-source `AKTIVNI` flag alone is insufficient. |
| Closed ROS record | Primary source `ros` with ROS state `ZANIKLY` establishes closure. |
| Missing lifecycle | Without a supported lifecycle signal the public lookup returns `unresolved_data`. |
| VAT | Native `dic` and `dicSkDph` do not establish VAT registration. |
| Addresses | Registered office comes from `sidlo`. A delivery address is not a fiscal address; original `textovaAdresa` stays unchanged. |

## Example basis

The success output is serialized through the current decoder and common-result mapping from `exact_identity_and_required_name` plus an explicit cessation date in the [synthetic tests](../../src/cz/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/cz/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.
