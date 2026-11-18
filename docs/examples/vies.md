# VIES: example outputs

[All examples](README.md) · [Main README](../../README.md#examples) · [VIES reference](../reference/vies.md)

VIES VAT lookup supports all 28 VAT prefixes. These examples describe the 4.1 API, including local checksum preflight. They use synthetic SOAP responses, not live VAT observations or identifiers to query.

## Input

```rust,ignore
let result = client.lookup_vat("BG123456786").await;
```

The result is `Result<VatLookupResult, LookupError>`. See [serialization](README.md#serialize-success-and-failure). `is_vat_valid` makes its own request and returns `Result<bool, LookupError>`; read `result.valid` when you already have the full result.

## Positive case: valid VAT

Complete serialized inner success value:

```json
{
  "vatId": "BG123456786",
  "valid": true,
  "source": "Vies",
  "company": " Синтетична & Δοκιμή SRL ",
  "address": {
    "fullAddressByVies": "Примерна улица 1\nСофия 1000",
    "address": null,
    "address2": null,
    "city": null,
    "postcode": null,
    "region": null,
    "country": "България",
    "countryISO2": "BG"
  }
}
```

Validity is independent of address parsing. This synthetic address is preserved verbatim while uncertain components remain null. Names are source text, not translated. Country metadata comes from the VAT jurisdiction and does not prove the physical address country.

## Negative case: matched invalid VAT

The response echoes the exact requested VAT identity and explicitly reports false. This is `Ok(result)` with `valid: false`, not an error:

```json
{
  "vatId": "BG123456786",
  "valid": false,
  "source": "Vies",
  "company": null,
  "address": {
    "fullAddressByVies": null,
    "address": null,
    "address2": null,
    "city": null,
    "postcode": null,
    "region": null,
    "country": "България",
    "countryISO2": "BG"
  }
}
```

A negative result does not establish absence from the national company register. If VIES supplies identity/address text with false, the adapter preserves that text; it is not required to be null.

## Edge case: valid but company details withheld

Missing owner fields do not turn a positive VAT observation into a negative one:

```json
{
  "vatId": "BG123456786",
  "valid": true,
  "source": "Vies",
  "company": null,
  "address": {
    "fullAddressByVies": null,
    "address": null,
    "address2": null,
    "city": null,
    "postcode": null,
    "region": null,
    "country": "България",
    "countryISO2": "BG"
  }
}
```

Missing or nil address text becomes null. Empty/placeholder address text, when present, stays in `fullAddressByVies`; blank or withheld company-name placeholders become null. Country metadata can still be populated when no address text is supplied.

## Service failure: no validity conclusion

A recognized `MS_UNAVAILABLE` SOAP fault for the Bulgarian request produces this complete error with raw diagnostics disabled:

```json
{
  "id": "service_unavailable",
  "title": "Verification service unavailable",
  "description": "Server, network or timeout failure prevented verification. Identifier validity remains unknown.",
  "isServiceUnavailable": true,
  "details": {
    "provider": "Vies",
    "country": "BG",
    "reason": "member_state_unavailable",
    "nativeRegistryStatus": null,
    "upstreamCode": "MS_UNAVAILABLE",
    "upstreamHttpStatus": 500,
    "responseContentType": "text/xml",
    "retryAfter": null,
    "retryAfterSeconds": null,
    "scope": null,
    "operation": null,
    "identifierScheme": null,
    "debug": null
  }
}
```

Do not turn this into `valid: false`. Other failures include `rate_limited`, `cooling_down`, `unexpected_response` and `response_mismatch`; respect available retry guidance.

## Country and input edges

| Case | Behavior |
| --- | --- |
| Greece | Use VAT prefix `EL`; address jurisdiction is `GR`. National GEMI lookup separately uses `GR` and a GEMI identifier. |
| Northern Ireland | Use `XI`; address jurisdiction is `GB`, with GB-NIR in capability metadata. General GB VAT IDs are unsupported. |
| Norway and Iceland | National company lookup exists, but these countries are outside VIES. |
| OSS/IOSS | Prefixes `EU` and `IM` are unsupported; rejection does not establish invalid registration. |
| Whitespace and case | Parsing removes ASCII whitespace and uppercases letters; leading zeroes remain. Punctuation is not generally removed. |
| Bad format or supported checksum failure | Returns `invalid_input` locally on main. No negative VIES observation has been made. |
| Unsupported checksum variant | Local `VatChecksumResult::Unsupported` does not mean invalid; lookup can still contact VIES. |
| Wrong echoed identity | Returns `response_mismatch`, even if the response says valid. |
| Missing/ambiguous address components | Keep original decoded text; uncertain components remain null. Parsing success does not control VAT validity. |

The [VIES reference](../reference/vies.md#identifiers-and-coverage) lists all supported prefixes and checksum variants. National company output examples are linked from the [gallery](README.md#choose-an-example).

## Example basis

The positive output uses the [synthetic SOAP fixture](../../src/vies/fixtures/valid.xml), the [SOAP decoder cases](../../src/vies/tests/soap.rs), and the same address projection used by the client. The negative variant changes `valid` to false and omits owner fields; the withheld variant omits owner fields with valid still true. The error is serialized from an offline HTTP 500 response carrying the SOAP fault through the public client, including transport metadata. No lookup timestamps are serialized in `VatLookupResult`.
