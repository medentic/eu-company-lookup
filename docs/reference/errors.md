# Lookup errors

All public lookup, validation and client-construction failures use one
`LookupError` object. VIES and national registries use the same error IDs and JSON
shape. `RegistryError` and the old `LookupError` enum variants are removed in 4.0.

A matched negative VIES observation is a successful result with `valid: false`.
An error never establishes VAT invalidity. A registry `not_found` is a definitive
miss only from the sources identified in its details. Skipping a check does not
produce a negative observation.

## Object contract

| Field | Meaning |
| --- | --- |
| `id` | Stable snake-case machine ID. Rust exposes `ErrorId`; match future variants defensively. |
| `title` | Short default English text suitable for users. `Display` returns this title. |
| `description` | Default English administrator explanation from the common catalog. |
| `isServiceUnavailable` | Service/network availability prevented verification. A connection failure may be local; it does not prove an upstream outage. False does not imply validity. |
| `details` | Structured context and optional raw diagnostics. Missing facts are null. |

`LookupError::default()` is the generic `unknown_error` fallback. Normal provider
failures receive a specific classification. IDs are stable; applications must not
parse titles or descriptions to determine behavior. Applications own localization
and their own HTTP status mapping; this library does not prescribe an API status.

`details` contains `provider`, ISO `country`, a more specific `reason`, a recognized
`upstreamCode` where available, actual `upstreamHttpStatus`, `responseContentType`,
`retryAfter`, `retryAfterSeconds`, rate-limit `scope`, disabled `operation` and
`identifierScheme`, and `debug`. Retry values preserve seconds or an absolute
HTTP date. `upstreamHttpStatus` and `responseContentType` are populated whenever
response headers arrived, including when raw debugging is disabled. A failure
before headers arrive has no upstream status. Country is supplied only when known;
VIES EL maps to GR and XI maps to GB.

## Error catalog

| ID | Default title | Administrator meaning |
| --- | --- | --- |
| `invalid_input` | Check the identifier | Local syntax or supported checksum validation failed; registration was not checked. |
| `unsupported_identifier` | Identifier type not supported | Unsupported country, VAT jurisdiction or identifier scheme. |
| `operation_disabled` | Lookup is disabled | Configuration disabled the operation; no request was sent. |
| `invalid_configuration` | Lookup configuration needs attention | Invalid client settings; inspect the reason. |
| `initialization_failed` | Lookup could not start | HTTP client initialization failed. |
| `busy` | Verification is busy | Local capacity or admission timing prevented sending a request. |
| `rate_limited` | Too many verification requests | The provider requested less traffic; respect retry instructions. |
| `cooling_down` | Please wait before retrying | An earlier response established a cooldown; no request was sent. |
| `service_unavailable` | Verification service unavailable | Server, network or timeout failure; validity remains unknown. |
| `access_denied` | Verification access denied | Authentication is required or access was denied. |
| `provider_rejected_input` | Verification request rejected | Provider rejected the request; this does not prove invalid registration. |
| `unexpected_response` | Verification could not be completed | Unexpected HTTP status, malformed data, schema change or unknown protocol fault. |
| `response_mismatch` | Verification response could not be confirmed | Response identifies a different subject. |
| `response_too_large` | Verification could not be completed | Response exceeded the configured transport size bound. |
| `not_found` | No matching company record found | Identified registry sources definitively missed; not VAT invalidity. |
| `unresolved_data` | Company status could not be resolved | Matched record has missing, unsupported or ambiguous lifecycle evidence; neither a registry miss nor an outage. |
| `unknown_error` | Verification could not be completed | Generic fallback; no validity conclusion follows. |

`service_unavailable` has `isServiceUnavailable: true`. Rate limits, local cooldowns,
access denial and malformed responses retain their distinct classifications.
For Greece's `MS_UNAVAILABLE`, the common ID is `service_unavailable`, provider is
`Vies`, country is `GR`, and reason is `member_state_unavailable`.

## Raw HTTP diagnostics

Raw capture is explicitly enabled on each adapter configuration:

```rust
use eu_company_lookup::{Client, ClientConfig, IeRegistryConfig};

let mut vies = ClientConfig::default();
vies.raw_debug = true;
let mut ireland = IeRegistryConfig::default();
ireland.raw_debug = true;
let client = Client::builder().vies(vies).irish_registry(ireland).build()?;
# Ok::<(), eu_company_lookup::LookupError>(())
```

All adapter configurations default `raw_debug` to false. Configure the relevant
adapter; VIES configuration also applies to BG's shared VIES enrichment when
standalone VIES is enabled. Native BG register calls use `BgRegistryConfig`.

With raw capture enabled, `details.debug.exchanges` is an array in request order.
Each element contains `request` and nullable `response`. Multiple requests, such
as a registry search followed by a detail lookup, retain separate entries.
Concurrent lookups have independent captures. Local failures have an empty array;
connection failures retain the request with a null response. Successful results
do not expose debug captures.

Request fields are `method`, full `url`, `headers`, `contentType`, `body`, `curl`,
`curlUnavailableReason`, and `captureLayer`. Response fields are `httpStatus`,
`headers`, `contentType`, `body`, `complete`, and `incompleteReason`.

Headers are arrays of `{ "name": "...", "value": { "encoding": "utf8", "data": "..." } }`.
Duplicate header names are retained. Bodies use the same `encoding`/`data` shape.
Non-UTF-8 values use `base64`, never replacement characters. An absent request body
is null; an empty received response body is empty UTF-8 data.

No redaction is applied: raw diagnostics can include submitted identifiers,
personal/company data, cookies and credentials. Expose or store them only in the
administrator/debug channel you intend. `Display` and `Debug` on `LookupError`
omit raw captures; explicit serialization includes them. Normal mode does not
retain request/response bodies or header arrays in errors.

Capture is at the HTTP-client layer, not the packet layer. `captureLayer` is
`prepared_request`: explicitly configured headers are captured, but transport-added
headers, original header casing/order, HTTP framing and TLS bytes are not wire
captures. Response bytes are the body exposed by the HTTP client.

The cURL command shell-quotes request values and sends body bytes via base64 and
`--data-binary @-`. It reproduces method, URL, captured headers and body; it does
not reproduce connection state, transport-added headers or TLS negotiation. If
header bytes cannot be represented safely in a shell command, cURL is null and
`curlUnavailableReason` explains why; the captured bytes are still retained.
The library never executes the command or sends an extra diagnostic request.

Raw mode reads error bodies that normal mode may skip. Existing total deadlines
and response-size limits still apply. Within those bounds, bodies are retained in
full. A size-limit interruption, network failure or timeout yields a partial body,
`complete: false`, and an explicit reason; it is never labeled a complete response.
Reading diagnostics can consume the remaining deadline. A received authoritative
rate-limit/access status remains that classification if the diagnostic read times
out. Retry/cooldown policy and definitive-miss rules remain provider-owned.

## Migrating from 3.x

All operations still return `Result<T, LookupError>`. Replace enum destructuring
with checks of `error.id` and structured fields in `error.details`:

```rust
use eu_company_lookup::{ErrorId, LookupError};

fn explain(error: &LookupError) -> &str {
    match error.id {
        ErrorId::ServiceUnavailable => "Verification unavailable; validity unknown",
        ErrorId::NotFound => "No company record in the checked registry sources",
        _ => error.title,
    }
}
```

Replace `LookupError::Registry { provider, error }` with `details.provider`, the
common `id`, and `details.reason`. `MemberStateUnavailable`, `ProviderTimeout`,
transport failures and registry availability failures share `service_unavailable`;
the reason and provider retain the distinction. `OperationDisabled` moves its
operation/scheme fields into details. The old Bulgarian `NotFound` uses reason
`both_bulgarian_registers_missed`; a single registry miss identifies its provider.
`is_service_unavailable()` remains available as a convenience over the public flag.
Private transport classifications are implementation details, not exported errors.

## Unresolved company lifecycle

All national lookup methods, including native registry clients, require a resolved
company lifecycle before returning success. A matched record with missing lifecycle
evidence returns `unresolved_data` with reason `missing_lifecycle`; unsupported or
contradictory evidence uses `unmapped_or_ambiguous_lifecycle`.
Details include the provider, country and `nativeRegistryStatus` when the source
supplied one. Source text is preserved in explicit error serialization, while
Display/Debug omit it. Raw HTTP diagnostics remain opt-in.
This error has `isServiceUnavailable: false`, establishes no negative registration
result and does not trigger registry fallback or an outage cooldown.
