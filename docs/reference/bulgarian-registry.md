# Bulgarian business lookup

Use `Client::lookup_company(code, "BG")` or `BgRegistryClient::lookup(code)` to look up
a Bulgarian business identifier. The adapter tries the Commercial Register first
and BULSTAT only after a definitive miss. It does not infer VAT registration.

## Input and fallback

`BgBusinessId` accepts nine or thirteen ASCII digits after removing ASCII whitespace.
It preserves leading zeroes and the complete branch suffix. Raw inputs over 64
bytes, non-ASCII input, BG-prefixed VAT IDs, ten-digit personal identifiers and
other punctuation are rejected. Validation checks format, not checksum or existence.

| Provider result | Behavior |
| --- | --- |
| Exact Commercial Register record | Return it; do not query BULSTAT. |
| Empty Commercial Register HTTP 200, with absent or JSON content type | Query BULSTAT. |
| Exact BULSTAT search and detail match | Return the BULSTAT record. |
| Recognized empty BULSTAT search after a Commercial Register miss | Return `LookupError::NotFound`. |
| Timeout, throttling, access denial, unexpected schema or mismatched identity | Return an error; do not interpret it as absence or trigger fallback. |

Whitespace, JSON null, HTML and endpoint 404s are not Commercial Register misses.
BULSTAT can return related subjects; the full code must match both the search row
and detail record. An unmatched nonempty report is inconclusive and returns an error.

## Company record

The facade returns `CompanyLookupResult`: the code is in `taxId` and its country in `country`, the
address in `registeredOffice`, and source-specific fields in typed `registryDetails`.
The table below describes the native `BgCompanyLookupResult` returned by the
lower-level `BgRegistryClient`. Conversion to the shared result preserves these
source details.

| JSON field | Meaning |
| --- | --- |
| `taxId` | Complete requested business code, without a VAT prefix. |
| `source` | `BulgarianCommercialRegister` or `Bulstat`; the provider discriminator. |
| `registryUrl` | Official public record URL. |
| `companyName` | Source legal name, with Commercial Register deletion markup removed. |
| `companyEnglishName` | Separately registered foreign-script name when supplied; not a translation guarantee. |
| `legalForm` | Recognized Bulgarian abbreviation, otherwise the original description or null. |
| `address` | Decoded source text and conservative structured fields. |
| `status` | Source-specific lifecycle information; never a VAT-validity flag. |
| `nativeRegistryStatus` | Original status regardless of mapping: decoded BULSTAT text or the Commercial Register numeric code as a string. Null when unavailable. |
| `registryDetails` | Provider-specific fields described below. |

The adapter requests Bulgarian data. It does not fetch the portals' English views.
Recognized legal forms map to ЕТ, ЕООД, ООД, АД, ЕАД and СД. Unknown descriptions are
preserved; legal form is not inferred from the company name.

A closed main record, explicit deletion field or recognized deleted-company state
maps to `closed`, overriding an open subrecord. Recognized open states map to
`active`; unknown Commercial Register codes map to `unknown` and are retained in
`registryDetails.statusCode`. BULSTAT `развиващ дейност` maps to the typed `CompanyStatus::Active`
(JSON `active`); unrecognized descriptions map to `CompanyStatus::Unknown` (JSON `unknown`).
The original BULSTAT status is retained in `registryDetails.statusText`. These values do not establish a universal business status.

### Commercial Register details

`registryDetails` contains the original `statusCode`, `registeredName`, `identifierEntryDate`,
`identifierEntryYear`, `managers`, `owners`, `capital`, `businessActivity`,
`documents` and `hasInstructions`.

- The entry date comes from the UIC field; the year is derived. Neither is labelled
  incorporation date. Court-file years and other field dates are not substitutes.
- Managers are the registered managers; owners currently mean sole capital owners.
  A null owners list does not establish absence of partners or other ownership forms.
  Explicit corporate-holder names can be separated from labelled business-code
  metadata; ambiguous descriptions remain available in `sourceText`.
- Role holders retain source descriptions. Names and countries are extracted only
  when recognizable; unknown details remain null.
- Capital has registered/paid-in values, exact decimal strings, recognized currencies
  and original source text. No floating-point monetary conversion is performed.
- Documents are constitutive-document references for the matching main subrecord,
  not document downloads or a full history. Only official DocumentAccess URLs are
  returned. A local announcement date belongs to an individual source block;
  ambiguous dates remain null and no timezone is invented.
- Optional unavailable or uninterpretable fields remain null. Unexpected required
  structure, unknown field operations and conflicting duplicates fail explicitly.

### BULSTAT details

`registryDetails.subjectText` retains the decoded subject description;
`registryDetails.statusText` retains the original status, or null if absent. The adapter
uses the separately supplied search name, checks it against the detail description,
and does not split compound names at arbitrary commas. It does not extract managers,
ownership, historical documents or entry dates from BULSTAT.

### Addresses

`fullAddressByRegistry` retains decoded visible source text. Shared fields include
`address`, `address2`, `city`, `postcode`, `region`, `municipality`, `country` and
`countryISO2`. Uncertain components remain null.

Explicit recognized countries are normalized to the shared native name; missing
address countries default to `България` / `BG`. Bulgarian delivery normalization applies only when the normalized or default country
is BG. City districts and labelled contact fields remain in the original text but
are omitted from structured delivery fields. Explicit building details and
comma-delimited office (`кабинет`) details with values move to `address2`; empty
labels and words embedded in street names do not trigger that split. See [address policy](api-stability.md#address-policy).

## Operating limits

The adapter uses observed interfaces of the
[Commercial Register portal](https://portal.registryagency.bg/) and
[BULSTAT public enquiry](https://www.bulstat.bg/bg/view/spravki).
These interfaces can change without a library-level stability guarantee.

See the [README](../../README.md#handle-failures-and-configure-limits) for client
defaults. A total deadline covers the whole chain, including provider spacing.
By default there is no artificial request spacing: BULSTAT detail retrieval
starts immediately after a matching search response. Callers can opt into spacing
with `start_interval`. Clones share one lookup slot and provider cooldowns. Separate applications require
their own coordination. Successful small samples do not establish a safe bulk quota.

## Coverage limitations

Offline tests cover identity checks, direct/fallback hits, definitive misses,
malformed fields, closed records, source-preserving addresses and operational
failure handling. Reviewed source fixtures supplement synthetic tests in the
repository and are excluded from the crate archive.

Live coverage remains limited for successful thirteen-digit branch records,
natural-person BULSTAT records and legal-form transitions. Saved address reviews
do not certify every role holder, capital value or document. There is no selectable
English fetch, checksum verification or complete address-format coverage.

## Shared VAT fields

`Client::lookup_company` delegates to the Bulgarian company adapter, which always
performs sequential VAT enrichment after a successful Commercial Register or
BULSTAT lookup. VIES is a required internal dependency of that adapter. For a
nine-digit matched business ID it checks the candidate `BG` plus the unchanged ID.
Thirteen-digit branch IDs are skipped; no suffix is removed or parent inferred.
The native `BgRegistryClient` and native-to-common conversions remain registry-only.

Both shared BG details variants contain nullable `vatVerification`, a full
`VatLookupResult` with its own `source: "Vies"`. It is null when skipped or produced by native conversion. A completed negative or name-mismatched VIES
response remains available there. Primary `source`, registry URL, lifecycle,
company name and registry addresses are not replaced by VIES fields.

Common `vatRegistered` becomes true and `vatId` is copied from VIES only when the
response is valid, its echoed VAT ID matches the candidate, and its company name
matches the registry name after Unicode case, punctuation and whitespace
normalization. Word boundaries and legal-form tokens are preserved. There is no
fuzzy matching, translation, transliteration or suffix removal. An absent or
different name leaves both common fields null even when VIES says valid. A negative
VIES result also leaves them null: it does not prove domestic nonregistration.
This conservative comparison may leave genuine registrations unenriched.

VIES failures propagate as `LookupError`; registry success does not hide a failed
verification. Registry errors or definitive misses never trigger VIES. Sequential
calls retain their individual configured deadlines (BG 30 seconds, then VIES 8
seconds by default), spacing, admission limits and cooldowns. There are no retries.
There is no opt-out for the shared BG company lookup. `without_vies()` disables
only standalone `lookup_vat` / `is_vat_valid` operations; it does not disable this
internal dependency. When standalone VIES is configured, the BG adapter shares
its client, configuration and admission limits. Otherwise BG creates its required
VIES dependency with default bounds. RO still uses ANAF without a VIES call.
Capability discovery lists enabled public operations, not internal dependencies.

The [NRA public VAT register](https://portal.nra.bg/details/report-cash-terminal)
is a future source of domestic registration evidence. Its observed form asks for
a VAT number; automated access and EIK-to-VAT discovery need investigation. The
header's `ЕИК по БУЛСТАТ 131063188` identifies NRA, not the queried company.

Missing or blank address countries default to `България` / `BG` for both
Commercial Register and BULSTAT results. This is a library default based on the
provider jurisdiction; explicit source countries take precedence. Country names
use title case and recognized ISO codes use uppercase. Original address and party
source text remain unchanged. Manager/owner countries are normalized when supplied,
but missing party countries remain null: an address default does not establish
a person’s country.
