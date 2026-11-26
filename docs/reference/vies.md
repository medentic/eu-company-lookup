# VIES VAT verification

`Client::lookup_vat` queries VIES for an exact VAT identifier.
`Client::is_vat_valid` performs the same request and returns its validity flag.
Neither operation checks company registration in a national business register.

## Accountant PDF

`Client::vies_result_pdf(checked_vat, requester_vat)` returns
`Result<Vec<u8>, LookupError>`. Pass two country-prefixed VAT IDs, or set
`ClientConfig::requester_vat` once and pass `None` as the second argument.

```rust,no_run
use eu_company_lookup::{ClientBuilder, ClientConfig, VatId};

# async fn example(checked_vat: &str, my_vat: &str) -> Result<(), Box<dyn std::error::Error>> {
let mut config = ClientConfig::default();
config.requester_vat = Some(VatId::parse(my_vat)?);
let client = ClientBuilder::empty().vies(config).build()?;
let pdf = client.vies_result_pdf(checked_vat, None).await?;
# Ok(())
# }
```

An explicit requester overrides the default for that call, including across
clones and concurrent calls. Invalid identifiers fail before Chromium starts.
Supplying neither requester returns `invalid_configuration`.

The operation starts a private temporary headless Chrome or Chromium profile.
Install a browser that can launch under the application's account and Poppler's
`pdftotext` on its PATH (Debian/Ubuntu package `poppler-utils`, Homebrew `poppler`). If necessary,
set `CHROME` to the browser executable. The adapter submits the English VIES web
form once, checks the displayed country, VAT number, verdict, date and consultation
number, then prints that page. One document can report either a matched valid
or a matched invalid number. A missing consultation number, unavailable service,
or mismatched identifier returns an error. Check the printed verdict before
using the document for a decision.

The browser tries portrait A4 at progressively smaller scales down to 75%.
It verifies one page and A4 dimensions, applies the PDF permissions, then
verifies the displayed result fields can be read from the finished bytes. A document that cannot fit legibly returns `document_unavailable`.
The file allows password-free opening, copying and high-quality printing. Its
AES-256 owner permissions restrict editing in viewers that honor PDF permissions.
The date comes from VIES. An edit restriction provides no independent timestamp
or tamper-evident signature.

Browser lookups share admission and cooldowns with SOAP lookups. Set
`ClientConfig::pdf_timeout` to change their separate total deadline (default
60 seconds, maximum 300 seconds). SOAP response-body limits and raw HTTP
diagnostics apply to the HTTP adapter. The library leaves archiving and
file naming to its caller.

## National services and availability

The request path is application → this library → EU VIES → the relevant national
VAT system. VIES retrieves information from national VAT databases; it does not
maintain an independent copy of every country's registrations. One country's
service can therefore be unavailable while other countries continue to respond.
See the Commission's [VIES explanation](https://europa.eu/youreurope/business/finance-and-tax/vat/check-vat-number-vies/index_en.htm).

Check the official [self-monitoring page](https://ec.europa.eu/taxation_customs/vies/#/self-monitoring)
or [JSON availability endpoint](https://ec.europa.eu/taxation_customs/vies/rest-api/check-status)
for the central service and individual countries. Greece appears as `EL`.
Availability is diagnostic information, not a VAT validation result or a guarantee
that a particular request will succeed. An available country can still reject
requests because of capacity limits. The library does not currently fetch this
status endpoint automatically.

| Observation | Meaning for the caller |
| --- | --- |
| `MS_UNAVAILABLE` | The country's VAT service is unavailable through VIES; validity remains unknown. |
| `MS_MAX_CONCURRENT_REQ` | The country's request-capacity limit was reached; this does not establish an outage or an invalid VAT ID. |
| Matched response with `valid: false` | VIES returned a negative validation result for that identifier. |

National business-register lookup is a separate operation and can succeed during
a VIES outage. A company-register record alone does not prove VIES VAT validity.
The documented [Bulgarian enrichment flow](bulgarian-registry.md#shared-vat-fields)
also calls VIES and therefore retains that dependency.

## Identifiers and coverage

`VatId::parse` and `validate_vat_syntax` remove ASCII whitespace, uppercase letters
and check conservative national formats. Leading zeroes are retained; punctuation
is not generally removed, although supported Irish formats contain `*` or `+`.
Non-ASCII input and raw inputs longer than 64 bytes are rejected. Deserialization
and `TryFrom` also validate syntax; these constructors do not calculate checksums.

For offline validation, parse the identifier and call `VatId::validate_checksum()`.
It returns `VatChecksumResult::Passed`, `Failed`, or `Unsupported` synchronously,
without constructing a client or contacting VIES:

```rust
use eu_company_lookup::{VatChecksumResult, VatId};

let vat = VatId::parse("BG423456789")?;
assert_eq!(vat.validate_checksum(), VatChecksumResult::Passed);
# Ok::<(), eu_company_lookup::LookupError>(())
```

`Unsupported` means the parsed identifier's variant has no implemented checksum
check; it is not a failed checksum. Malformed input or an unsupported VAT prefix
still returns a parsing error. A passed checksum does not establish registration.
The result serializes as `"passed"`, `"failed"`, or `"unsupported"`.

Every supported VAT prefix has its own local length and character rules. Both
`lookup_vat` and `is_vat_valid` apply them before network access; malformed input
returns `InvalidInput`, rather than `valid: false`. Fixed format components are
checked too, including Austria's initial `U`, the Netherlands' `B`, and Sweden's
terminal `01`. See the Finnish Tax Administration's
[VAT format reference](https://www.vero.fi/en/businesses-and-corporations/taxes-and-charges/vat/international-commerce/vat-number/vat-numbers-issued-by-different-eu-countries/).
Before admission or HTTP, both lookup methods also check supported national
checksums. A definite mismatch returns `InvalidInput`, never `valid: false`.
Capabilities report `ChecksumSupport::SupportedFormats` for VIES. This applies to
VAT lookup, not the syntax-only constructors or national registry identifiers.

Checks are implemented in private local Rust code, adapted from
[eu-vat 0.1.11](https://docs.rs/eu-vat/0.1.11/eu_vat/) with Apache-2.0 attribution.
There is no checksum-library dependency. The national rules are also checked
against [python-stdnum](https://arthurdejong.org/python-stdnum/).
Every supported prefix has checksum coverage, but not every identifier variant
does; see the [variant table](#checksum-coverage-by-country-and-variant).
Checks do not validate birth dates, allocation ranges or registration.
Identifiers are never rewritten by checksum arithmetic.

Passing local checks permits a VIES request; it does not establish registration.
Saved matched negative observations remain valid evidence even if their IDs now
fail request preflight. Local rejections do not count toward live coverage.
Repository-only raw transport probes deliberately bypass lookup preflight so they
can collect matched negative responses; collector queue checks remain syntax-only.

Supported prefixes are AT, BE, BG, CY, CZ, DE, DK, EE, EL, ES, FI, FR, HR, HU, IE,
IT, LT, LU, LV, MT, NL, PL, PT, RO, SE, SI, SK and XI. EL maps to GR; XI maps to
GB with subdivision GB-NIR. GB, GR and CH are unsupported VAT prefixes.

EU and IM identifiers are rejected with `UnsupportedJurisdiction`. The adapter
does not verify OSS/IOSS registrations; this rejection is not an invalid-registration
result. See the Commission's [VIES explanation](https://europa.eu/youreurope/business/finance-and-tax/vat/check-vat-number-vies/index_en.htm)
and [SOAP contract](https://ec.europa.eu/taxation_customs/vies/checkVatService.wsdl)
for the provider's scope and interface.

### Checksum coverage by country and variant

Formats below exclude the country prefix; positions are one-based within that
national part. This table describes the implemented checks, not all numbers a
country may issue. Syntax validation happens first: malformed input is a parsing
error, not `Unsupported`. A checked variant returns `Passed` or `Failed`; an
unsupported variant returns `Unsupported` and may still be sent to VIES.
“None” means every syntax-accepted variant has a checksum check.

| Country / prefix | Formats with a checksum check | Syntax-accepted variants returning `Unsupported` |
| --- | --- | --- |
| Austria `AT` | `U` + 8 digits | None |
| Belgium `BE` | 10 digits beginning `0` or `1` | None |
| Bulgaria `BG` | 9 digits; 10 digits checked against personal, foreign-person and other registration schemes (any passing scheme suffices) | None |
| Cyprus `CY` | 8 digits + letter | None |
| Czechia `CZ` | 8 digits not beginning `9`; 9 digits beginning `6`; 10 digits | 8 digits beginning `9`; 9 digits not beginning `6` |
| Germany `DE` | 9 digits | None |
| Denmark `DK` | 8 digits | None |
| Estonia `EE` | 9 digits | None |
| Greece `EL` | 9 digits | None |
| Spain `ES` | 8 digits + letter; letter + 7 digits + check character, with initial letter in `ABCDEFGHJKLMNPQRSUVWXYZ` (DNI, NIE and company schemes) | Initial letter `I`, `O` or `T` |
| Finland `FI` | 8 digits | None |
| France `FR` | 11 digits: numeric 2-digit VAT key over the following 9 digits; no separate SIREN checksum | Either of the first 2 characters is a letter |
| Croatia `HR` | 11 digits | None |
| Hungary `HU` | 8 digits | None |
| Ireland `IE` | 7 digits + 1 letter; 7 digits + 2 letters when the second letter is `A`–`W`; legacy digit + letter/`*`/`+` + 5 digits + letter | 7 digits + 2 letters when the second letter is `X`, `Y` or `Z` |
| Italy `IT` | 11 digits with positions 8–10 equal to `001`–`100`, `120`, `121`, `888` or `999` | Other values at positions 8–10 |
| Lithuania `LT` | 9 or 12 digits with penultimate digit `1` | Penultimate digit other than `1` |
| Luxembourg `LU` | 8 digits | None |
| Latvia `LV` | 11 digits: company scheme for initial `4`–`9`; personal scheme for initial `0`–`3`, except `32` | Begins `32` |
| Malta `MT` | 8 digits | None |
| Netherlands `NL` | 9 digits + `B` + 2 digits: old BSN-based or newer whole-ID modulo-97 check (either may pass); branch allocation is not checked | None |
| Poland `PL` | 10 digits | None |
| Portugal `PT` | 9 digits | None |
| Romania `RO` | 2–10 digits | None |
| Sweden `SE` | 10 digits + `01`; checksum covers the first 10 digits, syntax enforces `01` | None |
| Slovenia `SI` | 8 digits | None |
| Slovakia `SK` | 10 digits: company or historical personal-number arithmetic (either may pass) | None |
| Northern Ireland `XI` | 9 or 12 digits; checksum covers the first 9, not the optional 3-digit branch suffix | `GD` + 3 digits; `HA` + 3 digits |

## Result and address semantics

Illustrative serialized result using synthetic data:

```json
{
  "vatId": "BG123456789",
  "valid": true,
  "source": "Vies",
  "company": "Synthetic Company",
  "address": {
    "fullAddressByVies": "ул. ПРИМЕР 1 обл.РУСЕ, гр.РУСЕ 7000",
    "address": "ул. ПРИМЕР 1",
    "address2": null,
    "city": "Русе",
    "postcode": "7000",
    "region": "Русе",
    "country": "България",
    "countryISO2": "BG"
  }
}
```

- `valid` comes from the matched provider response, independently of address parsing.
- `company` preserves meaningful source text; blank or dash-only names become null.
- `fullAddressByVies` preserves decoded source text and line breaks, including empty
  or unparseable values. Absent/nil source elements become null.
- `address` is the primary delivery line. `address2` contains remaining delivery
  lines and any parsed locality, separated by newlines. Bulgarian labelled building,
  entrance, floor and apartment details are split into `address2`.
- Uncertain extracted components remain null. `country` preserves parsed country
  text or falls back to the VAT jurisdiction's native country name. `countryISO2`
  identifies the VAT jurisdiction, not necessarily the physical address country.
- Source/observation timestamps are private; they are not serialized. Parsing
  diagnostics and the shared address model's `municipality` are not in this envelope.

The provider has no language-selection parameter. Owner information may be absent
or withheld. Names and addresses can also be supplied with a negative result;
those values must not override `valid: false`.

Conservative address rules cover AT, BE, BG, CY, CZ, DK, EE, EL, FI, FR, HR, HU, IT, LT,
LU, MT, NL, PL, PT, RO, SE, SI and SK. Unsupported formats retain original text and country
metadata with unknown delivery components. A matching format does not establish
postal correctness. See [shared address policy](api-stability.md#address-policy).

Romanian locality-first formats retain the labelled settlement and source postcode.
Four-, five- and six-digit source postcodes are kept without padding; all-zero
placeholders remain null. Explicit building, staircase, floor and apartment labels
start `address2`. Bucharest sector text stays with the city. Parent commune/town
labels are available in the shared parser's `municipality`, while the unchanged
full source remains in the VIES envelope. No county is inferred from a settlement.

### Known source limitations

Maltese VIES addresses can contain a short postcode that differs from the
postcode published by the company. The parser retains the supplied postcode;
it does not certify postal correctness or replace it with external data.

Portuguese VIES addresses can omit the postcode and combine city and
neighbourhood in an unlabelled line. For recognized partial formats, the parser
retains the street and remaining delivery text while leaving uncertain city and
postcode fields null. It does not enrich missing fields from other sources.

These are known limitations of the supplied address data, independent of VAT
validity. A successful source-preservation replay confirms the library's handling,
not the completeness or correctness of the upstream address. Applications should
allow customers to review billing details.

## Failures and admission

Only a matching negative response returns `valid: false`. Invalid input, malformed
SOAP, unknown faults, mismatched identifiers, HTTP failures and timeouts are errors.
Known SOAP capacity and availability faults retain typed classifications, including
when returned with HTTP 500/503. HTTP 408/504 map to provider timeout; HTTP 429 is a
rate limit and plain server errors are unavailable. Unknown fault text is sanitized.

The [README limits](../../README.md#handle-failures-and-configure-limits) include waiting,
HTTP and body reading. An occupied slot or spacing that cannot fit the remaining
deadline returns `Busy`. Capacity and availability failures establish shared
cooldowns. Valid Retry-After seconds or dates can extend them; extreme values are
preserved in the error while cooldown arithmetic is capped at ten years.

Production requests are single-attempt. Development retry probes are separate
ignored tests, not part of the production client's behavior or latency guarantee.
