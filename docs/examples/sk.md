# Slovakia: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **RÚZ accounting entities, with an RPO lifecycle supplement when needed**. These outputs describe the 5.0 API. Implementation and reviewed live coverage remain separate.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("00123456", "SK").await;
```

Eight IČO digits, preserving leading zeroes. DIČ and VAT identifiers are distinct. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic fixture adds `datumZrusenia: "2020-01-01"`, establishing a closed accounting entity. No RPO query is needed, so `rpoLifecycle` is null. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "SK",
  "taxId": "00123456",
  "vatRegistered": null,
  "vatId": null,
  "source": "SlovakRuz",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic s.r.o.",
  "legalForm": "Spol. s r. o.",
  "registeredOffice": null,
  "fiscalAddress": null,
  "status": "closed",
  "registryDetails": {
    "provider": "SlovakRuz",
    "data": {
      "rpoLifecycle": null,
      "nativeRegistryStatus": null,
      "id": 42,
      "ico": "00123456",
      "nazovUJ": "Synthetic s.r.o.",
      "dic": "0012345678",
      "ulica": " Source street ",
      "mesto": "Town",
      "psc": "00100",
      "pravnaForma": "112",
      "datumZalozenia": null,
      "datumZrusenia": "2020-01-01",
      "datumPoslednejUpravy": null,
      "zdrojDat": null,
      "sidlo": null,
      "kraj": null,
      "okres": null,
      "skNace": null,
      "velkostOrganizacie": null,
      "druhVlastnictva": null,
      "stav": null
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
    "provider": "SlovakRuz",
    "country": "SK",
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
| Two requests | Resolve one public internal ID, then require exact internal ID and IČO in the detail response. Both requests share the lookup deadline. |
| Lifecycle | Without a closure date, an exact active-only RPO search and matched current detail can resolve active or supported proceedings. Missing RPO matches and malformed/ambiguous lifecycle return `unresolved_data`; API availability `stav` is not lifecycle. |
| Supplemental failures | RPO throttling or access failure remains an operational error. Up to four requests share the total deadline and configured pacing. |
| Address role | Accounting-entity address components stay native; common registered office and fiscal address remain null. |
| Missing public data | Empty, ambiguous, deleted or private records do not prove absence from national registers. |

## Example basis

The success output is serialized through the current decoder and common-result mapping from the `source` fixture with the lifecycle change described above in the [synthetic tests](../../src/sk/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/sk/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.

If an exact RÚZ record has no dissolution date and RPO returns an empty successful
search, the result remains `unresolved_data`. HTTP 503 or a timeout while querying
RPO is instead an operational failure. Neither outcome establishes closure.
