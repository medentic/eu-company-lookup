# VIES VAT verification

`Client::lookup_vat` queries VIES for an exact VAT identifier.
`Client::is_vat_valid` performs the same request and returns its validity flag.
Neither operation checks company registration in a national business register.

## Identifiers and coverage

`VatId::parse` and `validate_vat_syntax` remove ASCII whitespace, uppercase letters
and check conservative national formats. Leading zeroes are retained; punctuation
is not generally removed, although supported Irish formats contain `*` or `+`.
Non-ASCII input and raw inputs longer than 64 bytes are rejected. Deserialization
and `TryFrom` also validate. No checksum is calculated.

Supported prefixes are AT, BE, BG, CY, CZ, DE, DK, EE, EL, ES, FI, FR, HR, HU, IE,
IT, LT, LU, LV, MT, NL, PL, PT, RO, SE, SI, SK and XI. EL maps to GR; XI maps to
GB with subdivision GB-NIR. GB, GR and CH are unsupported VAT prefixes.

EU and IM identifiers are rejected with `UnsupportedJurisdiction`. The adapter
does not verify OSS/IOSS registrations; this rejection is not an invalid-registration
result. See the Commission's [VIES explanation](https://europa.eu/youreurope/business/finance-and-tax/vat/check-vat-number-vies/index_en.htm)
and [SOAP contract](https://ec.europa.eu/taxation_customs/vies/checkVatService.wsdl)
for the provider's scope and interface.

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

Conservative address rules cover AT, BE, BG, CY, CZ, DK, EE, EL, FI, FR, IT, LT,
LU, NL, PL, PT, SE, SI and SK. Unsupported formats retain original text and country
metadata with unknown delivery components. A matching format does not establish
postal correctness. See [shared address policy](api-stability.md#address-policy).

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
