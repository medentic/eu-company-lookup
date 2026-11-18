# Poland: example outputs

[All examples](README.md) · [Main README](../../README.md#country-support) · [Shared fields](../reference/company-result.md)

Source: **KRS business and association extracts**. These outputs describe the 5.0 API. Implementation and reviewed live coverage remain separate.

These are **synthetic, offline examples**, not observations of real companies or identifiers to query. JSON blocks show the complete serialized value unless explicitly marked as excerpts. They illustrate the output shape; they do not establish live coverage.

## Input

```rust,ignore
let result = client.lookup_company("0001234567", "PL").await;
```

Ten KRS digits. NIP, REGON and VAT identifiers are different schemes. This call illustrates input syntax; running it would contact the provider. See [how to serialize either outcome](README.md#serialize-success-and-failure).

## Positive case: matched record

The synthetic extract adds an explicit ongoing liquidation opening in section six, producing `liquidation`. “Positive” means the record matched and its lifecycle was resolved, not necessarily that it is active or VAT-registered.

```json
{
  "country": "PL",
  "taxId": "0001234567",
  "vatRegistered": null,
  "vatId": null,
  "source": "PolishKrs",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic sp. z o.o.",
  "legalForm": "Source form",
  "registeredOffice": {
    "fullAddressByRegistry": null,
    "address": "Source street 1",
    "address2": "2",
    "city": "Town",
    "postcode": "00-001",
    "region": null,
    "municipality": null,
    "country": "Polska",
    "countryISO2": "PL"
  },
  "fiscalAddress": null,
  "status": "liquidation",
  "registryDetails": {
    "provider": "PolishKrs",
    "data": {
      "nativeRegistryStatus": null,
      "dzial6": {
        "likwidacja": [
          {
            "otwarcieLikwidacji": "Opening",
            "zakonczenieLikwidacji": null
          }
        ],
        "postepowanieUpadlosciowe": null,
        "postepowanieRestrukturyzacyjneNaprawczePrzymusowaRestrukturyzacjaUporzadkowanaLikwidacja": null
      },
      "foreignBranch": false,
      "naglowek": {
        "rejestr": "RejP",
        "numerKRS": "0001234567",
        "dataCzasOdpisu": null,
        "stanZDnia": null,
        "dataRejestracjiWKRS": null,
        "dataOstatniegoWpisu": null,
        "stanPozycji": 1
      },
      "danePodmiotu": {
        "nazwaFormaPrawnaPrzedsiebiorcyZagranicznego": null,
        "innyRejestr": null,
        "nazwa": "Synthetic sp. z o.o.",
        "formaPrawna": "Source form",
        "identyfikatory": {
          "nip": "0012345678",
          "regon": "00123456700000"
        }
      },
      "siedzibaIAdres": {
        "siedzibaIAdresZakladuGlownego": null,
        "siedziba": null,
        "adres": {
          "wojewodztwo": null,
          "ulica": "Source street",
          "nrDomu": "1",
          "nrLokalu": "2",
          "miejscowosc": "Town",
          "kodPocztowy": "00-001",
          "poczta": null,
          "kraj": "POLSKA"
        }
      }
    }
  }
}
```

## Negative case and failure boundary

This adapter does not expose a definitive `not_found` outcome. For example, upstream HTTP 404 produces the error below, not a negative registration result. A P-register 404 triggers an S-register lookup. The error below results when that route also returns 404; neither response proves an access outage or company closure. Empty/ambiguous or malformed source responses also remain errors, with the exact classification depending on the response.

```json
{
  "id": "unexpected_response",
  "title": "Verification could not be completed",
  "description": "An unexpected HTTP status, malformed response, changed schema or unrecognized fault prevented verification.",
  "isServiceUnavailable": false,
  "details": {
    "provider": "PolishKrs",
    "country": "PL",
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
| Scope | Business register P and association register S current extracts and narrowly scoped removal recovery are supported; CEIDG sole traders are separate. |
| No proceeding information | A complete, identity-matched current extract with ordered dates and no unresolved restriction maps to `active`; an empty section or position code alone is insufficient. |
| Merger direction | `corporateEvents` distinguishes acquirer, absorbed party, resulting entity, asset recipient, partial division and transformation. Transaction role alone does not establish current lifecycle. |
| Successors | Retain supplied receiver KRS numbers on absorbed-party events; preserve leading zeroes and omit malformed/all-zero placeholders from `successorKrs`. Source party values remain available. |
| Non-final removal | Return `closed` and retain the provider’s non-finality marker; the marker distinguishes it from a final removal. |
| No current extract (HTTP 204) | Check a bounded full extract. Require exact identity and a terminal removal with finality evidence. Final and explicitly non-final removals map to `closed`, with finality retained; an unsuccessful supplement remains an error. |
| Ended or conflicting proceedings | Ending liquidation is not automatically deletion. Competing or unsupported proceeding evidence remains unresolved. |
| Foreign branches | Retain the full Polish branch identifier. Foreign-parent name/address do not replace common branch identity/address; unrestricted current registration maps to `active` for the branch only; proceeding scope can remain unresolved. |
| Wrong register | After a P-register 404, look up S and require an exact `RejS` identity. Outages and malformed P responses never trigger register fallback. |
| Native tax identifiers | Keep full NIP and REGON in details without claiming VAT registration. |

## Example basis

The success output is serialized through the current decoder and common-result mapping from the `source` fixture with the lifecycle change described above in the [synthetic tests](../../src/pl/tests.rs). Failure output is serialized from an offline loopback HTTP response through the public client and shared [error conversion](../../src/model/error.rs). Field and lifecycle rules live in the [provider mapping](../../src/pl/mapping.rs). See [reading these examples](README.md#reading-the-examples) for null, failure and version semantics.

## Confirmed removal after a merger

This synthetic example starts with HTTP 204 from the current-extract endpoint.
The identity-matched full extract supplies a final removal entry, so the old
company is returned as `closed`. The successor KRS is separate from `taxId`;
lookup does not redirect to the surviving company. `successorKrs` identifies the
receiver named in that transaction, not its independently verified current status.
The selected identity fields are those retained at terminal removal.

```json
{
  "country": "PL",
  "taxId": "0001234567",
  "vatRegistered": null,
  "vatId": null,
  "source": "PolishKrs",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic absorbed company",
  "legalForm": "SPÓŁKA Z OGRANICZONĄ ODPOWIEDZIALNOŚCIĄ",
  "registeredOffice": null,
  "fiscalAddress": null,
  "status": "closed",
  "registryDetails": {
    "provider": "PolishKrs",
    "data": {
      "nativeRegistryStatus": null,
      "corporateEvents": [
        {
          "role": "absorbed",
          "successorKrs": [
            "0007654321"
          ],
          "source": {
            "okreslenieOkolicznosci": "PRZEJĘCIE PRZEZ INNĄ SPÓŁKĘ",
            "opisPolaczeniaPodzialuPrzeksztalcenia": "Synthetic merger into the named receiver",
            "podmiotyPowstalePrzejmujace": [
              {
                "nazwa": "Synthetic surviving company",
                "krajNazwaRejestru": "POLSKA,KRAJOWY REJESTR SĄDOWY",
                "numerWRejestrzeAlboEwidencji": "0007654321",
                "krs": {
                  "krs": "0007654321"
                }
              }
            ],
            "podmiotyPrzejmowane": null
          }
        }
      ],
      "fullExtractRemoval": {
        "entryNumber": 3,
        "description": "WYKREŚLENIE Z KRAJOWEGO REJESTRU SĄDOWEGO",
        "entryDate": "10.02.2025",
        "finalDate": "01.03.2025"
      },
      "dzial6": null,
      "foreignBranch": false,
      "naglowek": {
        "rejestr": "RejP",
        "numerKRS": "0001234567",
        "dataCzasOdpisu": null,
        "stanZDnia": "10.03.2025",
        "dataRejestracjiWKRS": null,
        "dataOstatniegoWpisu": null,
        "stanPozycji": 2
      },
      "danePodmiotu": {
        "nazwaFormaPrawnaPrzedsiebiorcyZagranicznego": null,
        "innyRejestr": null,
        "nazwa": "Synthetic absorbed company",
        "formaPrawna": "SPÓŁKA Z OGRANICZONĄ ODPOWIEDZIALNOŚCIĄ",
        "identyfikatory": null
      },
      "siedzibaIAdres": null
    }
  }
}
```

The [merger tests](../../src/pl/tests/mergers.rs) check this complete serialized
example and exercise proposals, historical acquisitions, partial divisions,
missing/future finality, mismatched identity and supplemental HTTP errors.

## Unrestricted current registration

One current-extract request is sufficient when identity, ordered header dates and
lifecycle facts agree. `active` describes KRS registration without a reported
restriction, not proof of trading or solvency. This synthetic example is checked
against `active_source` in the provider tests.

```json
{
  "country": "PL",
  "taxId": "0001234567",
  "vatRegistered": null,
  "vatId": null,
  "source": "PolishKrs",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic sp. z o.o.",
  "legalForm": "Source form",
  "registeredOffice": {
    "fullAddressByRegistry": null,
    "address": "Source street 1",
    "address2": "2",
    "city": "Town",
    "postcode": "00-001",
    "region": null,
    "municipality": null,
    "country": "Polska",
    "countryISO2": "PL"
  },
  "fiscalAddress": null,
  "status": "active",
  "registryDetails": {
    "provider": "PolishKrs",
    "data": {
      "nativeRegistryStatus": null,
      "dzial6": {
        "likwidacja": null,
        "postepowanieUpadlosciowe": null,
        "postepowanieRestrukturyzacyjneNaprawczePrzymusowaRestrukturyzacjaUporzadkowanaLikwidacja": null
      },
      "foreignBranch": false,
      "naglowek": {
        "rejestr": "RejP",
        "numerKRS": "0001234567",
        "dataCzasOdpisu": null,
        "stanZDnia": "01.03.2025",
        "dataRejestracjiWKRS": "01.01.2001",
        "dataOstatniegoWpisu": "01.02.2025",
        "stanPozycji": 1
      },
      "danePodmiotu": {
        "nazwaFormaPrawnaPrzedsiebiorcyZagranicznego": null,
        "innyRejestr": null,
        "nazwa": "Synthetic sp. z o.o.",
        "formaPrawna": "Source form",
        "identyfikatory": {
          "nip": "0012345678",
          "regon": "00123456700000"
        }
      },
      "siedzibaIAdres": {
        "siedzibaIAdresZakladuGlownego": null,
        "siedziba": null,
        "adres": {
          "wojewodztwo": null,
          "ulica": "Source street",
          "nrDomu": "1",
          "nrLokalu": "2",
          "miejscowosc": "Town",
          "kodPocztowy": "00-001",
          "poczta": null,
          "kraj": "POLSKA"
        }
      }
    }
  }
}
```

## Continued existence after liquidation

A registered continued-existence resolution ends each liquidation period. The
old dissolution description must match an opening, all ending dates must be
after their openings and no later than the snapshot, and the current name must
no longer carry the liquidation designation. Complete current-registration
evidence is still required. Retained liquidator entries do not override those
explicit endings; only their count is exposed.

```json
{
  "country": "PL",
  "taxId": "0001234567",
  "vatRegistered": null,
  "vatId": null,
  "source": "PolishKrs",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic sp. z o.o.",
  "legalForm": "SPÓŁKA Z OGRANICZONĄ ODPOWIEDZIALNOŚCIĄ",
  "registeredOffice": {
    "fullAddressByRegistry": null,
    "address": "Source street 1",
    "address2": "2",
    "city": "Town",
    "postcode": "00-001",
    "region": null,
    "municipality": null,
    "country": "Polska",
    "countryISO2": "PL"
  },
  "fiscalAddress": null,
  "status": "active",
  "registryDetails": {
    "provider": "PolishKrs",
    "data": {
      "nativeRegistryStatus": null,
      "dzial6": {
        "likwidacja": [
          {
            "otwarcieLikwidacji": "UCHWAŁA NR 1 Z DNIA 01.01.2010 O LIKWIDACJI SPÓŁKI",
            "zakonczenieLikwidacji": "UCHWAŁA NR 2 Z DNIA 01.01.2011 O DALSZYM ISTNIENIU SPÓŁKI. , 01.01.2011"
          },
          {
            "otwarcieLikwidacji": "UCHWAŁA NR 3 Z DNIA 01.01.2020 O LIKWIDACJI SPÓŁKI",
            "zakonczenieLikwidacji": "UCHWAŁA NR 4 Z DNIA 01.01.2021 O DALSZYM ISTNIENIU SPÓŁKI. , 01.01.2021",
            "liquidatorCount": 1
          }
        ],
        "postepowanieUpadlosciowe": null,
        "postepowanieRestrukturyzacyjneNaprawczePrzymusowaRestrukturyzacjaUporzadkowanaLikwidacja": null,
        "rozwiazanieUniewaznienie": {
          "okreslenieOkolicznosci": "ROZWIĄZANIE",
          "informacjaORozwiazaniuUniewaznieniu": "UCHWAŁA NR 3 Z DNIA 01.01.2020 O LIKWIDACJI SPÓŁKI, 01.01.2020"
        }
      },
      "foreignBranch": false,
      "naglowek": {
        "rejestr": "RejP",
        "numerKRS": "0001234567",
        "dataCzasOdpisu": null,
        "stanZDnia": "01.03.2025",
        "dataRejestracjiWKRS": "01.01.2001",
        "dataOstatniegoWpisu": "01.02.2025",
        "stanPozycji": 1
      },
      "danePodmiotu": {
        "nazwaFormaPrawnaPrzedsiebiorcyZagranicznego": null,
        "innyRejestr": null,
        "nazwa": "Synthetic sp. z o.o.",
        "formaPrawna": "SPÓŁKA Z OGRANICZONĄ ODPOWIEDZIALNOŚCIĄ",
        "identyfikatory": {
          "nip": "0012345678",
          "regon": "00123456700000"
        }
      },
      "siedzibaIAdres": {
        "siedzibaIAdresZakladuGlownego": null,
        "siedziba": null,
        "adres": {
          "wojewodztwo": null,
          "ulica": "Source street",
          "nrDomu": "1",
          "nrLokalu": "2",
          "miejscowosc": "Town",
          "kodPocztowy": "00-001",
          "poczta": null,
          "kraj": "POLSKA"
        }
      }
    }
  }
}
```

## Opening order duplicated into the ending field

An identical court order appointing a liquidator in both fields does not establish
an ending. With a current liquidator entry, a valid order date and the statutory
liquidation name suffix, the result remains `liquidation`, not `closed`. These
synthetic examples are asserted by the [liquidation tests](../../src/pl/tests/liquidation.rs).

```json
{
  "country": "PL",
  "taxId": "0001234567",
  "vatRegistered": null,
  "vatId": null,
  "source": "PolishKrs",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic sp. z o.o. W LIKWIDACJI",
  "legalForm": "SPÓŁKA Z OGRANICZONĄ ODPOWIEDZIALNOŚCIĄ",
  "registeredOffice": {
    "fullAddressByRegistry": null,
    "address": "Source street 1",
    "address2": "2",
    "city": "Town",
    "postcode": "00-001",
    "region": null,
    "municipality": null,
    "country": "Polska",
    "countryISO2": "PL"
  },
  "fiscalAddress": null,
  "status": "liquidation",
  "registryDetails": {
    "provider": "PolishKrs",
    "data": {
      "nativeRegistryStatus": null,
      "dzial6": {
        "likwidacja": [
          {
            "liquidatorCount": 1,
            "otwarcieLikwidacji": "SĄD REJESTROWY, POSTANOWIENIEM ORZEKŁ O JEJ ROZWIĄZANIU I USTANOWIŁ LIKWIDATORA, 01.01.2020",
            "zakonczenieLikwidacji": "SĄD REJESTROWY, POSTANOWIENIEM ORZEKŁ O JEJ ROZWIĄZANIU I USTANOWIŁ LIKWIDATORA, 01.01.2020"
          }
        ],
        "postepowanieUpadlosciowe": null,
        "postepowanieRestrukturyzacyjneNaprawczePrzymusowaRestrukturyzacjaUporzadkowanaLikwidacja": null
      },
      "foreignBranch": false,
      "naglowek": {
        "rejestr": "RejP",
        "numerKRS": "0001234567",
        "dataCzasOdpisu": null,
        "stanZDnia": "01.03.2025",
        "dataRejestracjiWKRS": "01.01.2001",
        "dataOstatniegoWpisu": "01.02.2025",
        "stanPozycji": 1
      },
      "danePodmiotu": {
        "nazwaFormaPrawnaPrzedsiebiorcyZagranicznego": null,
        "innyRejestr": null,
        "nazwa": "Synthetic sp. z o.o. W LIKWIDACJI",
        "formaPrawna": "SPÓŁKA Z OGRANICZONĄ ODPOWIEDZIALNOŚCIĄ",
        "identyfikatory": {
          "nip": "0012345678",
          "regon": "00123456700000"
        }
      },
      "siedzibaIAdres": {
        "siedzibaIAdresZakladuGlownego": null,
        "siedziba": null,
        "adres": {
          "wojewodztwo": null,
          "ulica": "Source street",
          "nrDomu": "1",
          "nrLokalu": "2",
          "miejscowosc": "Town",
          "kodPocztowy": "00-001",
          "poczta": null,
          "kraj": "POLSKA"
        }
      }
    }
  }
}
```

## Association-register fallback

A P-register 404 triggers one current S-register request. This example is active
on complete current-registration evidence; `naglowek.rejestr` preserves `RejS`.
A 204 on that S route requests the full S extract, with the same strict removal
checks and the agreed non-final-removal policy. Neither HTTP code implies closure.

```json
{
  "country": "PL",
  "taxId": "0001234567",
  "vatRegistered": null,
  "vatId": null,
  "source": "PolishKrs",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic association",
  "legalForm": "STOWARZYSZENIE",
  "registeredOffice": {
    "fullAddressByRegistry": null,
    "address": "Source street 1",
    "address2": "2",
    "city": "Town",
    "postcode": "00-001",
    "region": null,
    "municipality": null,
    "country": "Polska",
    "countryISO2": "PL"
  },
  "fiscalAddress": null,
  "status": "active",
  "registryDetails": {
    "provider": "PolishKrs",
    "data": {
      "nativeRegistryStatus": null,
      "dzial6": {
        "likwidacja": null,
        "postepowanieUpadlosciowe": null,
        "postepowanieRestrukturyzacyjneNaprawczePrzymusowaRestrukturyzacjaUporzadkowanaLikwidacja": null
      },
      "foreignBranch": false,
      "naglowek": {
        "rejestr": "RejS",
        "numerKRS": "0001234567",
        "dataCzasOdpisu": null,
        "stanZDnia": "01.03.2025",
        "dataRejestracjiWKRS": "01.01.2001",
        "dataOstatniegoWpisu": "01.02.2025",
        "stanPozycji": 3
      },
      "danePodmiotu": {
        "nazwaFormaPrawnaPrzedsiebiorcyZagranicznego": null,
        "innyRejestr": null,
        "nazwa": "Synthetic association",
        "formaPrawna": "STOWARZYSZENIE",
        "identyfikatory": {
          "nip": "0012345678",
          "regon": "00123456700000"
        }
      },
      "siedzibaIAdres": {
        "siedzibaIAdresZakladuGlownego": null,
        "siedziba": null,
        "adres": {
          "wojewodztwo": null,
          "ulica": "Source street",
          "nrDomu": "1",
          "nrLokalu": "2",
          "miejscowosc": "Town",
          "kodPocztowy": "00-001",
          "poczta": null,
          "kraj": "POLSKA"
        }
      }
    }
  }
}
```

## Removal recovered from register S

After P returns 404 and current S returns 204, a matched full S extract can
establish removal. The selected source identity remains unchanged. Non-final
removal markers, when present, remain visible under the agreed `closed` policy.

```json
{
  "country": "PL",
  "taxId": "0001234567",
  "vatRegistered": null,
  "vatId": null,
  "source": "PolishKrs",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic absorbed company",
  "legalForm": "SPÓŁKA Z OGRANICZONĄ ODPOWIEDZIALNOŚCIĄ",
  "registeredOffice": null,
  "fiscalAddress": null,
  "status": "closed",
  "registryDetails": {
    "provider": "PolishKrs",
    "data": {
      "nativeRegistryStatus": null,
      "corporateEvents": [
        {
          "role": "absorbed",
          "successorKrs": [
            "0007654321"
          ],
          "source": {
            "okreslenieOkolicznosci": "PRZEJĘCIE PRZEZ INNĄ SPÓŁKĘ",
            "opisPolaczeniaPodzialuPrzeksztalcenia": "Synthetic merger into the named receiver",
            "podmiotyPowstalePrzejmujace": [
              {
                "nazwa": "Synthetic surviving company",
                "krajNazwaRejestru": "POLSKA,KRAJOWY REJESTR SĄDOWY",
                "numerWRejestrzeAlboEwidencji": "0007654321",
                "krs": {
                  "krs": "0007654321"
                }
              }
            ],
            "podmiotyPrzejmowane": null
          }
        }
      ],
      "fullExtractRemoval": {
        "entryNumber": 3,
        "description": "WYKREŚLENIE Z KRAJOWEGO REJESTRU SĄDOWEGO",
        "entryDate": "10.02.2025",
        "finalDate": "01.03.2025"
      },
      "dzial6": null,
      "foreignBranch": false,
      "naglowek": {
        "rejestr": "RejS",
        "numerKRS": "0001234567",
        "dataCzasOdpisu": null,
        "stanZDnia": "10.03.2025",
        "dataRejestracjiWKRS": null,
        "dataOstatniegoWpisu": null,
        "stanPozycji": 7
      },
      "danePodmiotu": {
        "nazwaFormaPrawnaPrzedsiebiorcyZagranicznego": null,
        "innyRejestr": null,
        "nazwa": "Synthetic absorbed company",
        "formaPrawna": "SPÓŁKA Z OGRANICZONĄ ODPOWIEDZIALNOŚCIĄ",
        "identyfikatory": null
      },
      "siedzibaIAdres": null
    }
  }
}
```

## Cancelled liquidation with withdrawn opening

An explicit continued-existence resolution cancels liquidation and dismisses the liquidator. Complete current registration and no conflicting current liquidation facts support `active`.

```json
{
  "country": "PL",
  "taxId": "0001234567",
  "vatRegistered": null,
  "vatId": null,
  "source": "PolishKrs",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic sp. z o.o.",
  "legalForm": "SPÓŁKA Z OGRANICZONĄ ODPOWIEDZIALNOŚCIĄ",
  "registeredOffice": {
    "fullAddressByRegistry": null,
    "address": "Source street 1",
    "address2": "2",
    "city": "Town",
    "postcode": "00-001",
    "region": null,
    "municipality": null,
    "country": "Polska",
    "countryISO2": "PL"
  },
  "fiscalAddress": null,
  "status": "active",
  "registryDetails": {
    "provider": "PolishKrs",
    "data": {
      "nativeRegistryStatus": null,
      "dzial6": {
        "likwidacja": [
          {
            "otwarcieLikwidacji": null,
            "zakonczenieLikwidacji": "01.01.2021 R., NOTARIUSZ SYNTHETIC, UCHWAŁA NR 2 W SPRAWIE DALSZEGO ISTNIENIA SPÓŁKI, UCHYLENIA LIKWIDACJI I ODWOŁANIA LIKWIDATORA , 01.01.2021"
          }
        ],
        "postepowanieUpadlosciowe": null,
        "postepowanieRestrukturyzacyjneNaprawczePrzymusowaRestrukturyzacjaUporzadkowanaLikwidacja": null
      },
      "foreignBranch": false,
      "naglowek": {
        "rejestr": "RejP",
        "numerKRS": "0001234567",
        "dataCzasOdpisu": null,
        "stanZDnia": "01.03.2025",
        "dataRejestracjiWKRS": "01.01.2001",
        "dataOstatniegoWpisu": "01.02.2025",
        "stanPozycji": 1
      },
      "danePodmiotu": {
        "nazwaFormaPrawnaPrzedsiebiorcyZagranicznego": null,
        "innyRejestr": null,
        "nazwa": "Synthetic sp. z o.o.",
        "formaPrawna": "SPÓŁKA Z OGRANICZONĄ ODPOWIEDZIALNOŚCIĄ",
        "identyfikatory": {
          "nip": "0012345678",
          "regon": "00123456700000"
        }
      },
      "siedzibaIAdres": {
        "siedzibaIAdresZakladuGlownego": null,
        "siedziba": null,
        "adres": {
          "wojewodztwo": null,
          "ulica": "Source street",
          "nrDomu": "1",
          "nrLokalu": "2",
          "miejscowosc": "Town",
          "kodPocztowy": "00-001",
          "poczta": null,
          "kraj": "POLSKA"
        }
      }
    }
  }
}
```

## Current registration after bankruptcy ended

The dated court order is in the bankruptcy ending field. Ordered dates and independent current registration support `active`; an ending alone does not establish company closure.

```json
{
  "country": "PL",
  "taxId": "0001234567",
  "vatRegistered": null,
  "vatId": null,
  "source": "PolishKrs",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic sp. z o.o.",
  "legalForm": "Source form",
  "registeredOffice": {
    "fullAddressByRegistry": null,
    "address": "Source street 1",
    "address2": "2",
    "city": "Town",
    "postcode": "00-001",
    "region": null,
    "municipality": null,
    "country": "Polska",
    "countryISO2": "PL"
  },
  "fiscalAddress": null,
  "status": "active",
  "registryDetails": {
    "provider": "PolishKrs",
    "data": {
      "nativeRegistryStatus": null,
      "dzial6": {
        "likwidacja": null,
        "postepowanieUpadlosciowe": [
          {
            "informacjaOOgloszeniuUpadlosci": {
              "data": "01.01.2015",
              "dataNadaniaKlauzuliWykonalnosci": null,
              "rodzajPostepowania": null
            },
            "opisZakonczeniaProcesuUpadlosci": {
              "opisSposobuZakonczeniaPostepowania": "POSTANOWIENIE SĄDU REJONOWEGO W SYNTHETIC X GUP 1/15",
              "dataZakonczeniaPostepowania": "01.06.2015"
            }
          }
        ],
        "postepowanieRestrukturyzacyjneNaprawczePrzymusowaRestrukturyzacjaUporzadkowanaLikwidacja": null
      },
      "foreignBranch": false,
      "naglowek": {
        "rejestr": "RejP",
        "numerKRS": "0001234567",
        "dataCzasOdpisu": null,
        "stanZDnia": "01.03.2025",
        "dataRejestracjiWKRS": "01.01.2001",
        "dataOstatniegoWpisu": "01.02.2025",
        "stanPozycji": 1
      },
      "danePodmiotu": {
        "nazwaFormaPrawnaPrzedsiebiorcyZagranicznego": null,
        "innyRejestr": null,
        "nazwa": "Synthetic sp. z o.o.",
        "formaPrawna": "Source form",
        "identyfikatory": {
          "nip": "0012345678",
          "regon": "00123456700000"
        }
      },
      "siedzibaIAdres": {
        "siedzibaIAdresZakladuGlownego": null,
        "siedziba": null,
        "adres": {
          "wojewodztwo": null,
          "ulica": "Source street",
          "nrDomu": "1",
          "nrLokalu": "2",
          "miejscowosc": "Town",
          "kodPocztowy": "00-001",
          "poczta": null,
          "kraj": "POLSKA"
        }
      }
    }
  }
}
```

## Surviving healthcare entity

Historical transformation and acquisition do not close the surviving entity. The acquired party’s REGON remains native; it is not converted to a successor KRS number.

```json
{
  "country": "PL",
  "taxId": "0001234567",
  "vatRegistered": null,
  "vatId": null,
  "source": "PolishKrs",
  "registryUrl": null,
  "foreignName": null,
  "companyName": "Synthetic sp. z o.o.",
  "legalForm": "Source form",
  "registeredOffice": {
    "fullAddressByRegistry": null,
    "address": "Source street 1",
    "address2": "2",
    "city": "Town",
    "postcode": "00-001",
    "region": null,
    "municipality": null,
    "country": "Polska",
    "countryISO2": "PL"
  },
  "fiscalAddress": null,
  "status": "active",
  "registryDetails": {
    "provider": "PolishKrs",
    "data": {
      "nativeRegistryStatus": null,
      "dzial6": {
        "likwidacja": null,
        "postepowanieUpadlosciowe": null,
        "postepowanieRestrukturyzacyjneNaprawczePrzymusowaRestrukturyzacjaUporzadkowanaLikwidacja": null,
        "polaczeniePodzialPrzeksztalcenie": [
          {
            "okreslenieOkolicznosci": "PRZEKSZTAŁCENIE",
            "opisPolaczeniaPodzialuPrzeksztalcenia": "Synthetic transaction description",
            "podmiotyPowstalePrzejmujace": null,
            "podmiotyPrzejmowane": null
          },
          {
            "okreslenieOkolicznosci": "POŁĄCZENIE",
            "opisPolaczeniaPodzialuPrzeksztalcenia": "Synthetic hospital acquisition",
            "podmiotyPowstalePrzejmujace": null,
            "podmiotyPrzejmowane": [
              {
                "nazwa": "Synthetic acquired hospital",
                "krajNazwaRejestru": null,
                "krs": null,
                "identyfikator": {
                  "nip": null,
                  "regon": "123456789"
                }
              }
            ]
          }
        ]
      },
      "foreignBranch": false,
      "naglowek": {
        "rejestr": "RejS",
        "numerKRS": "0001234567",
        "dataCzasOdpisu": null,
        "stanZDnia": "01.03.2025",
        "dataRejestracjiWKRS": "01.01.2001",
        "dataOstatniegoWpisu": "01.02.2025",
        "stanPozycji": 3
      },
      "danePodmiotu": {
        "nazwaFormaPrawnaPrzedsiebiorcyZagranicznego": null,
        "innyRejestr": null,
        "nazwa": "Synthetic sp. z o.o.",
        "formaPrawna": "Source form",
        "identyfikatory": {
          "nip": "0012345678",
          "regon": "00123456700000"
        }
      },
      "siedzibaIAdres": {
        "siedzibaIAdresZakladuGlownego": null,
        "siedziba": null,
        "adres": {
          "wojewodztwo": null,
          "ulica": "Source street",
          "nrDomu": "1",
          "nrLokalu": "2",
          "miejscowosc": "Town",
          "kodPocztowy": "00-001",
          "poczta": null,
          "kraj": "POLSKA"
        }
      },
      "corporateEvents": [
        {
          "role": "transformation",
          "successorKrs": [],
          "source": {
            "okreslenieOkolicznosci": "PRZEKSZTAŁCENIE",
            "opisPolaczeniaPodzialuPrzeksztalcenia": "Synthetic transaction description",
            "podmiotyPowstalePrzejmujace": null,
            "podmiotyPrzejmowane": null
          }
        },
        {
          "role": "acquirer",
          "successorKrs": [],
          "source": {
            "okreslenieOkolicznosci": "POŁĄCZENIE",
            "opisPolaczeniaPodzialuPrzeksztalcenia": "Synthetic hospital acquisition",
            "podmiotyPowstalePrzejmujace": null,
            "podmiotyPrzejmowane": [
              {
                "nazwa": "Synthetic acquired hospital",
                "krajNazwaRejestru": null,
                "krs": null,
                "identyfikator": {
                  "nip": null,
                  "regon": "123456789"
                }
              }
            ]
          }
        }
      ]
    }
  }
}
```
