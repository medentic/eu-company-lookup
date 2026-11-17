# Shared company result

`Client::lookup_company(tax_id, country)` returns `CompanyLookupResult` for every
supported country. Use this object for business forms, company profiles and
billing details. Its purpose is a consistent company observation, not a complete
legal extract or a combined report from several authorities.

This reference describes the shared contract in version 4. See
[API stability and migration](api-stability.md#migrating-from-3x)
before upgrading from a released version. Lower-level BG/RO clients still expose
their native result types; converting them with `Into` produces this shared view.

## Common fields and source details

Top-level fields have the same names, types and meanings across countries.
Availability can differ: a missing common fact stays null rather than moving to
a different path. Provider-specific facts live in `registryDetails.data`. They
extend the observation without overriding the common fields.

For example, normalized company lifecycle is `status`; a registry's original
status code or text is `registryDetails.data.nativeRegistryStatus`. Domestic VAT
registration is a common fact; a provider's VAT-on-collection regime is a source
detail. No result field translates a name or invents an unavailable fact.

All documented keys are serialized, including null values. JSON property order
is not part of the contract. Consumers must tolerate additional fields and new
provider/status variants. These are library output contracts, not general-purpose
JSON input models. Library-produced values satisfy the relationships below;
public Rust fields remain mutable, so caller edits can invalidate them.

## Company fields

| JSON field | Rust field / type | Meaning and unavailable behavior |
| --- | --- | --- |
| `country` | `country: String` | Uppercase ISO alpha-2 jurisdiction of the national identifier, such as BG or RO. Required. Not necessarily the address country or VAT prefix. |
| `taxId` | `tax_id: String` | Complete matched national business/fiscal identifier. Required; preserves normalized caller spelling, leading zeroes and branch suffixes. Not a numeric JSON value and not automatically a VAT number. |
| `vatRegistered` | `vat_registered: Option<bool>` | VAT registration supported by provider-specific evidence. RO: ANAF article-316 flag. BG: true after valid, name-matched VIES enrichment; otherwise null. A negative VIES result never becomes domestic false. Does not cover every special registration. |
| `vatId` | `vat_id: Option<VatId>` | VAT number supported by the matched registration and a provider-specific rule. Null when unavailable; not proof that no VAT ID exists. Never blindly prefix or truncate `taxId`. |
| `source` | `source: Provider` | Primary registry provider; supplementary VIES evidence has its own source in details. Required; equals `registryDetails.provider` on returned results. |
| `registryUrl` | `registry_url: Option<String>` | Official human-readable link for this record, when supplied by the adapter. No API endpoint, generic homepage, session token or invented permalink. Access may require interaction. Currently populated for BG and GR; other adapters may leave it null. Null means the adapter supplies no link, not that no official lookup site exists. |
| `companyName` | `company_name: String` | Source legal/display company name. Required; no generated translation. |
| `foreignName` | `foreign_name: Option<String>` | Separately registered foreign-script name. Not necessarily English and never an automatic translation. Null if unavailable. |
| `legalForm` | `legal_form: Option<String>` | Source legal form or supported local abbreviation. Null if unavailable. Equal-looking forms from different countries are not asserted to be legally equivalent. |
| `registeredOffice` | `registered_office: Option<CompanyAddress>` | Registered-office address. Null if neither structured office data nor original office text is available. Never filled with the fiscal address merely to avoid null. |
| `fiscalAddress` | `fiscal_address: Option<CompanyAddress>` | Separately supplied fiscal address. Null if unavailable. Equal values do not make it interchangeable with `registeredOffice`. |
| `status` | `status: Option<CompanyStatus>` | Normalized lifecycle: `active`, `closed`, `unknown`, or null. Unknown means a source status exists but has no supported lifecycle mapping; null means none was supplied. VAT status and fiscal inactivity do not determine lifecycle. |
| `registryDetails` | `registry_details: CompanyDetails` | Required typed provider extension, described below. Native observations remain available here even when a common field can be mapped. |

Current company provider values are `BulgarianCommercialRegister`, `Bulstat`,
`RomanianAnaf`, `AustrianFirmenbuch`, `BelgianNbb`, `CzechAres`, `FinnishPrh`, `FrenchRechercheEntreprises`, `EstonianRik`, `NorwegianBrreg`, `PolishKrs`, `SlovakRuz`, `LatvianOpenData`, `LithuanianJar`, `IrishCro`, `IcelandicSkatturinn` and
`GreekGemi`. `Vies` is a VAT provider, not a company-lookup result source.
The support list is `Client::SUPPORTED_COMPANY_COUNTRIES`; capability discovery
also distinguishes compiled support from adapters enabled on a client.

For RO, `vatRegistered` reflects ANAF article 316 at `requestDate`. Only a positive
matched observation permits deriving RO plus its numeric fiscal identifier. A
synthetic requested `taxId: "001234"` therefore retains its padding while the
supported VAT number is `RO1234`. False/unknown yields a null VAT ID. BG enriches
nine-digit company results via its required internal VIES check, populating VAT
fields only on valid, name-matched responses. Special or alternate fiscal
registrations must not be inferred. See [VAT scope and derivation](romanian-registry.md#shared-vat-fields).
Pure native-to-common conversion makes no request. The BG company adapter owns
the additional VIES call; errors propagate. See [BG enrichment rules](bulgarian-registry.md#shared-vat-fields).

## Shared address fields

`registeredOffice` and `fiscalAddress` use the same `CompanyAddress` shape. An
available source section can produce an object whose individual fields are all
null. Every component below is nullable; postal codes remain strings.

| JSON field | Rust access | Meaning |
| --- | --- | --- |
| `fullAddressByRegistry` | `full_address_by_registry` | Original decoded address text, retaining line boundaries. Never reconstructed from components. Null if the source supplied components only. |
| `address` | `fields.address` | Primary delivery line. |
| `address2` | `fields.address2` | Remaining delivery details or secondary lines. |
| `city` | `fields.city` | Settlement name, where identifiable. |
| `postcode` | `fields.postcode` | Source postal code, preserving leading zeroes. |
| `region` | `fields.region` | Source administrative region or county. |
| `municipality` | `fields.municipality` | Municipality when distinguishable from settlement/region. |
| `country` | `fields.country` | Title-case display country name, with language and diacritics retained or a documented native-name mapping. Not required to be English. |
| `countryISO2` | `fields.country_iso2` | Uppercase ISO country code when the explicit source country is recognized. Null for unrecognized source countries. BG and ANAF use documented jurisdiction defaults for missing address countries. |

`CompanyAddress` also dereferences to its structured `fields`. Normalize a
working copy only; original full text remains unchanged. Country casing is shared
across adapters, but settlement and street normalization is country-specific.

## Provider extension

`registryDetails` always has `provider` and `data`. The provider tag selects the
Rust `CompanyDetails` variant; no arbitrary JSON extension map is exposed.
`data.nativeRegistryStatus` is a nullable original status string for every current
provider. It remains present even if normalized `status` is known. Numeric BG
status codes are represented as decimal strings here, with no invented labels.

The following are the remaining data fields; their provider-specific meanings
are defined in the linked references.

| Provider | `registryDetails.data` fields in addition to `nativeRegistryStatus` |
| --- | --- |
| `BulgarianCommercialRegister` | `statusCode`, `registeredName`, `identifierEntryDate`, `identifierEntryYear`, `managers`, `owners`, `capital`, `businessActivity`, `documents`, `hasInstructions`, `vatVerification`. See [BG field semantics](bulgarian-registry.md). |
| `Bulstat` | `subjectText`, `statusText`, `vatVerification`. See [BULSTAT limitations](bulgarian-registry.md#coverage-limitations). |
| `RomanianAnaf` | `requestDate`, `checkedAt`, `tradeRegisterNumber`, `registrationDate`, `registrationStatus`, `activityCode`, `registeredOffice`, `fiscalAddress`, `taxStatus`. See [ANAF fields and dates](romanian-registry.md). |

ANAF's nested addresses retain street and street number separately; the common
addresses provide the shared delivery-line view. Country display normalization
applies to both. Its `taxStatus` retains source flags: `vatRegistered`,
`vatOnCollection`, `inactive`, `splitVat`, and `eInvoiceRegistered`. The duplicated
VAT flag is the original observation behind the common field, not an independent
check or an override. `requestDate` is the fiscal date queried; `checkedAt` is the
UTC observation time. Neither establishes historical accuracy of every field.

BG `vatVerification` is a nullable full `VatLookupResult`, retaining the checked
VAT number, validity, source, company name and VIES address. It is null when
skipped or when converting native results. It remains present for
negative validity, missing names and name mismatches, even though those leave
common VAT fields null. Its address never replaces a registry address.

For BG, identifier entry date is not necessarily incorporation date, foreign
name is not a translation, null owners/documents does not prove absence, and
capital uses exact decimal strings with source currencies. Consult the provider
reference before interpreting these extensions.

## Outcomes and interpretation

| Outcome | What the caller may conclude |
| --- | --- |
| Successful company result | The source returned a matched observation under its documented rules; not a guarantee of VAT registration or provider completeness. |
| Optional field is null | That fact is unavailable in this observation. |
| `vatRegistered: false` | The source reports no standard domestic registration in the checked scope/date; special registrations and VIES remain separate. |
| `status: "unknown"` | A source status exists without a supported lifecycle mapping. |
| `not_found`, reason `both_bulgarian_registers_missed` | Both BG registries definitively missed. |
| `not_found`, provider `RomanianAnaf` | ANAF definitively missed; not proof of an ONRC miss. |
| Timeout, service/schema/identity error | The lookup failed. No absence or false registration conclusion follows. |

A new registry must preserve these meanings. Optionality handles unavailable
facts; it is not permission to redefine the common contract by country.

For ANAF addresses, missing country values default to `Romania` / `RO` based on
the provider jurisdiction. Explicit source countries take precedence; this default
is not an explicit ANAF observation. Entirely absent addresses remain null.

BG address envelopes likewise default missing countries to `България` / `BG`.
Country names in registry details use the same title-case normalization and
recognized ISO codes are uppercase. Missing manager/owner countries remain null;
these party fields are not company addresses. Original source text is unchanged.

## Norwegian Enhetsregisteret

`NorwegianBrreg` matches a nine-digit organisation number in the public entity
endpoint. It supplies the source name and organisation-form description. The
source `registrertIMvaregisteret` boolean supplies domestic `vatRegistered`, when
present. Norway is outside VIES; `vatId` remains null and no number is synthesized.
This is a current register observation, not a caller-selected historical date.

Typed `NoCompanyDetails` retains source name/number, organisation form, selected
registration dates, activity classifications, domestic VAT and insolvency flags,
and separate `forretningsadresse` and `postadresse` objects. These business/postal
roles do not establish the common registered-office or fiscal-address role; both
common addresses remain null. Native address components and their original text
remain available. No aggregate lifecycle status is inferred from separate flags;
common status and native aggregate status are null. Foreign name and registry
link also remain null. HTTP 404 is retained as a provider error in this adapter.

## Irish CRO open company records

`IrishCro` accepts one-to-seven-digit company numbers and requires one exact
record from the official company dataset. The source includes dissolved companies.
It maps published name and company-type description while retaining native status,
status/type codes, dates, company-address lines and Eircode. Common lifecycle is
unknown when source status is present. The dataset's generically labelled address
lines remain native pending a source-backed office/fiscal-role mapping; VAT fields,
shared addresses, foreign name and company link remain null. A numeric source
identifier is never padded to match input, and a missing record does not prove
absence from every Irish register.
