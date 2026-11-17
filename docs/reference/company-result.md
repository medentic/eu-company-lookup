# Shared company result

`Client::lookup_company(tax_id, country)` returns `CompanyLookupResult` for every
supported country. Use this object for business forms, company profiles and
billing details. Its purpose is a consistent company observation, not a complete
legal extract or a combined report from several authorities.

This reference describes the shared contract in version 3. See
[API stability and migration](api-stability.md#migrating-from-2x)
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

## Czech ARES extension

`CzechAres` uses the existing common field meanings for eight-digit IČO identities.
Its typed details retain native legal-form codes, tax and VAT-group identifiers,
source creation/cessation/update dates, primary-source code, registered-office
components and per-register status codes. `nativeRegistryStatus` is null because
this endpoint supplies no single company lifecycle status. Common lifecycle and
VAT fields remain null pending source-backed mappings. Legal-form codes are not
exposed as common legal-form labels. No fiscal-address substitution is made.

`CzRegistryClient::lookup` supplies the native result. Its conversion to the common
view is pure and performs no VIES lookup. Native address text is preserved exactly;
recognized country presentation follows shared normalization. No country is inferred
from the lookup jurisdiction when absent from the source.

## Austrian public Firmenbuch

The anonymous JustizOnline adapter resolves an exact Firmenbuch number through
public search and detail responses. The common name and legal-form label come
from the matched detail. Street, city and postal code populate the registered
office; no full-address text or address country is invented. VAT fields, fiscal
address, normalized lifecycle and record URL remain null. Native status, route
identifier, domicile, source date and structured fields remain in
`registryDetails.data`. Empty active-only searches do not establish absence.

## Belgian NBB Consult

NBB Consult supplies company identity derived from CBE, with English legal-form
and legal-situation labels requested by the adapter. The source remains
`BelgianNbb`; this is not a direct CBE verification. Street and building number
form the delivery line; the source box number is retained as `address2`. Country
normalization uses the source country fields. Original components and native
legal-situation codes/date stay under `registryDetails.data`. VAT fields,
normalized lifecycle, full-address text and record URL remain null. An NBB miss
does not prove absence from CBE.

## Greek GEMI

Greek company lookup accepts the twelve-digit GEMI number, not the separate
nine-digit AFM tax number. Exact source identity is required. The registered
name, Latin-script name, legal-form label and official full address populate
the common fields. Structured address components stay null; the shorter map
address is retained only in native details. AFM, source status, codes and dates
remain under `registryDetails.data`. VAT fields and normalized lifecycle remain
unknown. The public record URL uses the matched full GEMI number.

## Finnish PRH YTJ mapping

The anonymous [PRH open-data service](https://avoindata.prh.fi/en) supplies company
information by exact Finnish business ID. `FinnishPrh` retains typed source
`businessId`, `euId`, `names`, `mainBusinessLine`, `website`, `companyForms`,
`companySituations`, `registeredEntries`, `addresses`, `tradeRegisterStatus`,
`status`, `registrationDate`, `endDate` and optional `lastModified` in
`registryDetails.data`, alongside `nativeRegistryStatus`.

The shared result uses the exact hyphenated business ID, the latest type-1 primary
name and a unique latest Finnish legal-form label when supplied. Dissolved records
can retain a latest name with an end date. Native status and tax-register codes
are not converted into lifecycle or VAT assertions. VAT fields, fiscal address,
foreign name and registry link remain null. PRH street/postal contact-address
roles remain native; they are not substituted for a proven legal registered
office. Source address strings and language codes are preserved. PRH is the
original data source; the library does not represent itself as PRH.

## French public company search

`FrenchRechercheEntreprises` uses exact nine-digit SIREN identity from the anonymous
DINUM public search API. SIRET establishment IDs and VAT IDs are separate schemes.
The source display name supplies `companyName`; native legal-form codes remain in
`natureJuridique`, with common legal form null. Administrative codes remain in
`etatAdministratif` and `nativeRegistryStatus`; lifecycle is unknown when supplied.
No VAT registration or number is inferred.

A matched `siege` supplies registered-office text and delivery/postal components.
Original `adresse` text is unchanged. Country is populated only from an explicit
source country label; no domestic default or INSEE-to-ISO guess is made. Region
and commune codes remain native codes, not generated names. Geocoded addresses
never replace source headquarters. Fiscal address, foreign name and registry link
remain null. Absent headquarters remain null.

Typed `FrCompanyDetails` retains SIREN, conserved SIREN, source names, legal-form,
activity, company-category, diffusion and administrative codes, source dates and
`FrHeadOffice` details. Native source field names serialize in camel case. Empty
public searches do not prove absence outside the source's diffusion coverage;
HTTP and schema errors remain operational failures.

## Estonian public registry

`EstonianRik` matches the full eight-digit registry code in public portal search.
It supplies the source name, original legal-address text and postcode. Remaining
address components are unknown. Native legal-form and status codes, previous
names, internal company ID and source URL remain in typed `EeCompanyDetails`.
A supplied status maps to unknown lifecycle; no active/closed inference. Common
legal form, VAT fields, fiscal address, foreign name and registry link remain null.
Empty or ambiguous searches are errors, not definitive registry absence.

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

## Polish KRS business extracts

`PolishKrs` requires a ten-digit KRS code and matches the current business-register
P extract header. It supplies source company name, legal-form label and available
registered-office address components. The complete KRS remains `taxId`; NIP and
REGON remain native identifiers and never become inferred VAT numbers. Common VAT
fields, lifecycle, native aggregate status, fiscal address, registry link and
foreign name remain null. No full address is synthesized from components.

`PlCompanyDetails` retains selected extract header, subject and seat/address fields.
Foreign-branch sections set `foreignBranch`; foreign undertaking names and parent
head-office addresses remain native and never replace the Polish branch identity
or registered office. Contradictory subject/address families fail decoding. This
adapter excludes association-register S and CEIDG sole traders. HTTP errors and
empty/current-unavailable responses never prove absence from other registers.

## Slovak public accounting entities

`SlovakRuz` accepts eight-digit IČO, resolves a unique internal entity ID and
requires both identifiers to match the detail response. It maps the source name
and requested identifier. DIČ, legal-form codes, dates and accounting-entity address
components remain native. They do not establish VAT registration or the common
registered-office/fiscal address role; those fields and lifecycle stay null.
Missing, private or deleted public accounting data never proves unregistration.

## Latvian registry open data

`LatvianOpenData` accepts an eleven-digit registration number and requires one
exact record from the official registry dataset. It maps the source name and
legal-form description, preserving the documented legal-address text as
`registeredOffice.fullAddressByRegistry`. Numeric location/postal codes remain
native without guessed formatting. Native closed markers and dates are retained;
no active/closed lifecycle is inferred. VAT identifiers and registration are not
inferred from registration or SEPA creditor numbers. Empty or ambiguous dataset
results remain operational mismatches, not registry-absence findings.

## Lithuanian published legal entities

`LithuanianJar` accepts modern nine-digit company codes and requires one exact
record in the official JAR open dataset. Legacy seven-digit codes are outside
scope. The periodically updated dataset includes removed entities and anonymizes
names for some legal forms. `companyName` preserves the published name.
Native full-address text, opaque address/form/status references and source dates
remain in typed details. This projection does not resolve references or infer
current lifecycle, office roles, legal-form labels or VAT registration; the
corresponding common fields remain null. An empty response does not prove absence.

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

## Icelandic public company pages

`IcelandicSkatturinn` accepts canonical ten-digit legal-entity kennitala with the
entity day prefix 41–71, excluding individual identifiers. It requires the exact
heading identity and labelled source table. The decoded `Lögheimili` cell supplies
registered-office full text with whitespace preserved; postal address remains
native. Legal-form code/description are taken only from two explicit source text
segments. Only the registration-labelled subtitle is retained, excluding contact
subtitles. No officers or document-ordering data are returned. Lifecycle, VAT
fields, fiscal address, foreign name and company link remain null. Iceland is
outside VIES; a missing domestic VAT table is not treated as nonregistration.
