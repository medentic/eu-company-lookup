# Bulgaria: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **Commercial Register / BULSTAT**. These outputs describe the 4.1 API, including resolved-lifecycle enforcement.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("123456786", "BG").await;
```

Nine-digit UIC/BULSTAT codes or supported thirteen-digit branch codes; retain the entire code. Do not include `BG`. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic registry company is active. A valid, name-matched VIES observation supplies the common VAT fields. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "BG",
  "taxId": "123456786",
  "vatRegistered": true,
  "vatId": "BG123456786",
  "source": "BulgarianCommercialRegister",
  "registryUrl": "https://portal.registryagency.bg/CR/Reports/ActiveConditionTabResult?uic=123456786",
  "foreignName": "Example & Co",
  "companyName": "\"ПРИМЕР\" ЕООД",
  "legalForm": "ЕООД",
  "registeredOffice": {
    "fullAddressByRegistry": "Държава: БЪЛГАРИЯ\nОбласт: Русе, Община: Русе\nНаселено място: гр. Русе, п.к. 7000\nул. Примерна № 1",
    "address": "ул. Примерна № 1",
    "address2": null,
    "city": "Русе",
    "postcode": "7000",
    "region": "Русе",
    "municipality": "Русе",
    "country": "България",
    "countryISO2": "BG"
  },
  "fiscalAddress": null,
  "status": "active",
  "registryDetails": {
    "provider": "BulgarianCommercialRegister",
    "data": {
      "nativeRegistryStatus": "1",
      "vatVerification": {
        "vatId": "BG123456786",
        "valid": true,
        "source": "Vies",
        "company": "ПРИМЕР ЕООД",
        "address": {
          "fullAddressByVies": "София 1000",
          "address": null,
          "address2": null,
          "city": null,
          "postcode": null,
          "region": null,
          "country": "България",
          "countryISO2": "BG"
        }
      },
      "statusCode": 1,
      "registeredName": null,
      "identifierEntryDate": null,
      "identifierEntryYear": null,
      "managers": null,
      "owners": null,
      "capital": {
        "registered": null,
        "paidIn": null
      },
      "businessActivity": null,
      "documents": null,
      "hasInstructions": null
    }
  }
}
```

## Negative case and failure boundary

A definitive Commercial Register miss followed by a definitive BULSTAT miss returns this error. Neither an outage nor a malformed response qualifies. The provider names the outer lookup context; the reason identifies both misses. HTTP 200 is the last upstream report response, not an application success recommendation.

```json
{
  "id": "not_found",
  "title": "No matching company record found",
  "description": "The identified registry sources definitively reported no matching company record. This does not establish VAT invalidity.",
  "isServiceUnavailable": false,
  "details": {
    "provider": "BulgarianCommercialRegister",
    "country": "BG",
    "reason": "both_bulgarian_registers_missed",
    "nativeRegistryStatus": null,
    "upstreamCode": null,
    "upstreamHttpStatus": 200,
    "responseContentType": "text/html",
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
| Fallback | BULSTAT is queried only after a definitive Commercial Register miss. A Commercial Register timeout never triggers fallback. |
| VIES failure | A failure in required enrichment returns an error even when the registry matched. |
| Name mismatch or withheld name | Keep the VIES observation in details, but leave common VAT fields null. |
| Thirteen-digit branches | Keep the complete branch identifier; do not truncate it into a VAT candidate. The internal VIES check is skipped. |
| Address roles | Keep the registry office; never replace it with the VIES address. Missing registry country defaults to България / BG. |

### Registry match with a negative VIES observation

The company lookup still succeeds. The common VAT fields remain null; they do not become false. This is an excerpt of the full result:

```json
{
  "vatRegistered": null,
  "vatId": null
}
```

The retained `registryDetails.data.vatVerification.valid` is `false`. The registry company name, office and lifecycle remain unchanged.

### BULSTAT fallback match

After a definitive Commercial Register miss, the synthetic BULSTAT record matches. Its required VIES observation is negative, so common VAT fields remain null. This synthetic VIES response omits the owner name; negative validity alone does not erase any supplied name or address.

```json
{
  "country": "BG",
  "taxId": "123456786",
  "vatRegistered": null,
  "vatId": null,
  "source": "Bulstat",
  "registryUrl": "https://reports.bulstat.bg/bulstat-ireports/preview?__report=https%3A%2F%2Freports.bulstat.bg%2Fbulstat-ireports%2Fv_rpt1.rptdesign&__format=html&__locale=bg&report_title=%D0%A1%D0%BF%D1%80%D0%B0%D0%B2%D0%BA%D0%B0+%D0%B7%D0%B0+%D1%81%D1%83%D0%B1%D0%B5%D0%BA%D1%82+%D0%BD%D0%B0+%D0%91%D0%A3%D0%9B%D0%A1%D0%A2%D0%90%D0%A2&uic=123456786",
  "foreignName": null,
  "companyName": "Пример",
  "legalForm": "Сдружение",
  "registeredOffice": {
    "fullAddressByRegistry": "седалище и адрес на управление: БЪЛГАРИЯ, пощенски код 1000, гр.София, ул. Примерна №1",
    "address": "ул. Примерна №1",
    "address2": null,
    "city": "София",
    "postcode": "1000",
    "region": null,
    "municipality": null,
    "country": "България",
    "countryISO2": "BG"
  },
  "fiscalAddress": null,
  "status": "active",
  "registryDetails": {
    "provider": "Bulstat",
    "data": {
      "nativeRegistryStatus": "развиващ дейност",
      "vatVerification": {
        "vatId": "BG123456786",
        "valid": false,
        "source": "Vies",
        "company": null,
        "address": {
          "fullAddressByVies": "София 1000",
          "address": null,
          "address2": null,
          "city": null,
          "postcode": null,
          "region": null,
          "country": "България",
          "countryISO2": "BG"
        }
      },
      "subjectText": "Пример, Example, държава: БЪЛГАРИЯ, правна форма: Сдружение",
      "statusText": "развиващ дейност"
    }
  }
}
```

## Example basis

The success output is serialized through the current decoder and common-result mapping from `deed`, `vat_response` and the facade enrichment fixtures in the [synthetic tests](../../src/bg/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/bg/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.
