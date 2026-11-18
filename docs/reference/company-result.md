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
| `source` | `source: Provider` | Primary registry provider; supplementary VIES and RPO evidence stays separately identified in native details. Required; equals `registryDetails.provider` on returned results. |
| `registryUrl` | `registry_url: Option<String>` | Official human-readable link for this record, when supplied by the adapter. No API endpoint, generic homepage, session token or invented permalink. Access may require interaction. Currently populated for BG, EE and GR; other adapters may leave it null. Null means the adapter supplies no link, not that no official lookup site exists. |
| `companyName` | `company_name: String` | Source legal/display company name. Required; no generated translation. |
| `foreignName` | `foreign_name: Option<String>` | Separately registered foreign-script name. Not necessarily English and never an automatic translation. Null if unavailable. |
| `legalForm` | `legal_form: Option<String>` | Source legal form or supported local abbreviation. Null if unavailable. Equal-looking forms from different countries are not asserted to be legally equivalent. |
| `registeredOffice` | `registered_office: Option<CompanyAddress>` | Registered-office address. Null if neither structured office data nor original office text is available. Never filled with the fiscal address merely to avoid null. |
| `fiscalAddress` | `fiscal_address: Option<CompanyAddress>` | Separately supplied fiscal address. Null if unavailable. Equal values do not make it interchangeable with `registeredOffice`. |
| `status` | `status: Option<CompanyStatus>` | Resolved lifecycle on successful lookups; see the lifecycle contract below. Pure native-to-common projections can retain `unknown` or null for inspection. Unknown means supplied status metadata cannot be mapped safely; null means no usable lifecycle evidence was supplied. Documented cessation dates can establish closed without a status label. VAT status and fiscal inactivity do not determine lifecycle. |
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
| `unresolved_data` | Matched record lacks a resolved lifecycle. Native status remains in error details; this is not absence. |
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
this endpoint supplies no single company lifecycle status. A valid source cessation date maps to closed; its absence does not imply active.
Unambiguous ARES RES legal-form codes map to Czech labels. VAT fields remain null. No fiscal-address substitution is made.

`CzRegistryClient::lookup` supplies the native result. Its conversion to the common
view is pure and performs no VIES lookup. Native address text is preserved exactly;
recognized country presentation follows shared normalization. No country is inferred
from the lookup jurisdiction when absent from the source.

## Austrian public Firmenbuch

The anonymous JustizOnline adapter resolves an exact Firmenbuch number through
public search and detail responses. The common name and legal-form label come
from the matched detail. Street, city and postal code populate the registered
office; no full-address text or address country is invented. VAT fields, fiscal
address and record URL remain null. Native `ACTIVE` maps to active and `DELETED` to closed; other states remain unknown. Native status, route
identifier, domicile, source date and structured fields remain in
`registryDetails.data`. Empty active-only searches do not establish absence.

## Belgian NBB Consult

NBB Consult supplies company identity derived from CBE, with English legal-form
and legal-situation labels requested by the adapter. The source remains
`BelgianNbb`; this is not a direct CBE verification. Street and building number
form the delivery line; the source box number is retained as `address2`. Country
normalization uses the source country fields. Original components and native
legal-situation codes/date stay under `registryDetails.data`. VAT fields,
full-address text and record URL remain null. Reviewed CBE codes map to active/closed or explicit liquidation, bankruptcy and
restructuring; unrecognized codes remain unknown in pure projections, and absent code/label remains null. An NBB miss
does not prove absence from CBE.

## Greek GEMI

Greek company lookup accepts the twelve-digit GEMI number, not the separate
nine-digit AFM tax number. Exact source identity is required. The registered
name, Latin-script name, legal-form label and official full address populate
the common fields. Structured address components stay null; the shorter map
address is retained only in native details. AFM, source status, codes and dates
remain under `registryDetails.data`. VAT fields remain null. Exact Greek active/deleted labels map to active/closed; dissolution/liquidation and
bankruptcy labels map to their distinct proceedings. Other statuses remain unresolved
and missing code/label stays null. The public record URL uses the matched full GEMI number.

## Finnish PRH YTJ mapping

The anonymous [PRH open-data service](https://avoindata.prh.fi/en) supplies company
information by exact Finnish business ID. `FinnishPrh` retains typed source
`businessId`, `euId`, `names`, `mainBusinessLine`, `website`, `companyForms`,
`companySituations`, `registeredEntries`, `addresses`, `tradeRegisterStatus`,
`status`, `registrationDate`, `endDate` and optional `lastModified` in
`registryDetails.data`, alongside `nativeRegistryStatus`.

The shared result uses the exact hyphenated business ID, the latest type-1 primary
name and a unique latest Finnish legal-form label when supplied. Dissolved records
can retain a latest name with an end date. Trade Register code 1 maps to active and code 4 to closed; other codes remain
unknown. Ongoing SANE/SELTILA/KONK situations map to restructuring/liquidation/bankruptcy
for registered records. Conflicting simultaneous situations or dissolution evidence
prevent code 1 asserting active. Tax-register codes do not cause blanket VAT inference. VAT fields, fiscal address,
foreign name and registry link remain null. PRH street/postal contact-address
roles remain native; they are not substituted for a proven legal registered
office. Source address strings and language codes are preserved. PRH is the
original data source; the library does not represent itself as PRH.

## French public company search

`FrenchRechercheEntreprises` uses exact nine-digit SIREN identity from the anonymous
DINUM public search API. SIRET establishment IDs and VAT IDs are separate schemes.
The source display name supplies `companyName`; native legal-form codes remain in
`natureJuridique` and map through the official INSEE list to French labels.
Company administrative A maps to active and C to closed; unknown codes remain unknown.
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
Status R maps to active, K to closed, L to liquidation and N to bankruptcy.
Verified legal-form codes 3/5 map to Estonian labels. Official source URLs map
only after host, path and exact identity validation. VAT fields, fiscal address
and foreign name remain null.
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
remain available. Explicit bankruptcy/liquidation flags map to their corresponding proceeding and
a valid deletion date maps to closed. Conflicting evidence remains unresolved;
active requires the `Enhet` response class, valid registration date, three explicit false proceeding flags and explicitly empty annotations. Native aggregate status remains null. Foreign name and registry
link also remain null. HTTP 404 is retained as a provider error in this adapter.

## Polish KRS business extracts

`PolishKrs` requires a ten-digit KRS code and matches the current P or S extract header to the requested register.
P is tried first; only HTTP 404 triggers S fallback. It supplies source company name, legal-form label and available
registered-office address components. The complete KRS remains `taxId`; NIP and
REGON remain native identifiers and never become inferred VAT numbers. Common VAT
fields, native aggregate status, fiscal address, registry link and
foreign name remain null. A single ongoing sixth-section liquidation, bankruptcy or restructuring resolves lifecycle; unsupported endings or competing proceedings remain unresolved. A dated discontinued bankruptcy is historical. A complete current extract with ordered header dates and no unresolved restrictions maps to `active` (KRS registration, not trading or solvency). Unrecognized section-six names are retained as `unmappedLifecycleFields` and block that inference. No full address is synthesized from components.

`PlCompanyDetails` retains selected extract header, subject and seat/address fields.
Selected dissolution descriptions and derived liquidator counts support narrow
reconciliation of continued-existence resolutions and duplicated opening orders;
see the [Polish mapping rules](../examples/pl.md#continued-existence-after-liquidation).
`corporateEvents` preserves structured reorganization roles and source parties;
`successorKrs` contains exact, non-placeholder receiver KRS values for an absorbed
party. A merger proposal, historical acquisition or partial asset division does
not by itself establish current lifecycle. These receiver IDs do not replace
`taxId` and are not automatically looked up.

HTTP 204 from the current route permits one bounded full-extract recovery. A
matched `Pełny` extract from the same P or S register with ordered entries, terminal final removal and
valid dates establishes `closed`; an explicitly non-final removal maps to `closed`. `fullExtractRemoval` preserves that evidence.
Identity and address fields are selected from records present at removal, with
earlier withdrawn records excluded. This is not a general historical-query API.

Foreign-branch sections set `foreignBranch`; foreign undertaking names and parent
head-office addresses remain native and never replace the Polish branch identity
or registered office. Contradictory subject/address families fail decoding. This
adapter excludes CEIDG sole traders. Each register's HTTP 204 permits full-extract
recovery in that same register. HTTP errors never prove absence or closure. An S
record retains `RejS` in its native header and uses the same common-field meanings.
S merger descriptions and P/S counterparty REGON values remain typed native facts,
without guessed successor KRS numbers.

## Slovak public accounting entities

`SlovakRuz` accepts eight-digit IČO, resolves a unique internal entity ID and
requires both identifiers to match the detail response. It maps the source name
and requested identifier, plus official RÚZ/ŠÚSR legal-form labels. DIČ, legal-form
codes, dates and accounting-entity address
components remain native. They do not establish VAT registration or the common
registered-office/fiscal address role; those address fields stay null. A valid dissolution date establishes closure. Otherwise a unique exact-IČO `onlyActive=true` RPO search and matched current detail establish active membership or a supported current CL010108 proceeding. `rpoLifecycle` retains the supplemental fields; query failure remains an operational error.
Missing, private or deleted public accounting data never proves unregistration.

## Latvian registry open data

`LatvianOpenData` accepts an eleven-digit registration number and requires one
exact record from the official registry dataset. It maps the source name and
legal-form description, preserving the documented legal-address text as
`registeredOffice.fullAddressByRegistry`. Numeric location/postal codes remain
native without guessed formatting. Native closed markers and dates are retained;
a valid removal date maps to closed. A marker alone stays unknown, and absence
of a date alone never implies active. Nonremoved records require complete identity-filtered liquidation, suspension and insolvency queries. Those results resolve active, liquidation or suspended activity, with native rows retained in `lifecycle`. Unclassified ongoing insolvency remains unresolved. VAT identifiers and registration are not
inferred from registration or SEPA creditor numbers. Empty or ambiguous dataset
results remain operational mismatches, not registry-absence findings.

## Lithuanian published legal entities

`LithuanianJar` accepts modern nine-digit company codes and requires one exact
record in the official JAR open dataset. Legacy seven-digit codes are outside
scope. The periodically updated dataset includes removed entities and anonymizes
names for some legal forms. `companyName` preserves the published name.
Native full-address text, opaque address/form/status references and source dates
remain in typed details. Registration address text populates the common office;
a valid deregistration date maps to closed. Missing dates do not imply active.
Official status references resolve ordinary membership, closure and supported proceedings; unsupported transformations remain unresolved. Unrecognized references are preserved as native status identifiers and produce unmapped-data diagnostics unless independent closure evidence resolves lifecycle. Referenced legal forms, VAT and fiscal fields stay null. An empty response does not prove absence.

## Irish CRO open company records

`IrishCro` accepts one-to-seven-digit company numbers and requires one exact
record from the official company dataset. The source includes dissolved companies.
It maps published name and company-type description while retaining native status,
status/type codes, dates, company-address lines and Eircode. Reviewed CRO labels/codes map normal and reinstated records to active and
terminal states to closed. Documented proceedings have explicit resolved states. Interim/conversion states
and contradictory code/label pairs remain unresolved. CRO company-address lines and Eircode populate registered-office delivery lines
and postcode; city/county, country and original full text are not synthesized.
VAT fields, fiscal address, foreign name and company link remain null. A numeric source
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

See the [company mapping audit](https://github.com/medentic/eu-company-lookup/blob/main/docs/development/company-mapping-audit.md) for
provider-specific status values, code-list sources and remaining null-field decisions.

## Lifecycle success contract

Successful national lookups return a resolved lifecycle: `active`, `closed`,
`liquidation`, `bankruptcy`, `receivership`, `liquidationAndReceivership`,
`restructuring`, `closed`, `liquidationStayed`, `administration`, or
`inactive`. Proceedings do not imply closure, VAT invalidity or an identity miss.
Known source codes and labels map only in their owning provider.

Missing, unsupported and contradictory lifecycle evidence returns
[`unresolved_data`](errors.md#unresolved-company-lifecycle) from both shared and
native lookup methods. The nullable Rust fields and `Unknown` variant remain
available for pure conversions and retained parser observations; they are not
successful runtime lookup outcomes. Other optional facts, such as foreign names
or fiscal addresses, remain nullable.

The provider mapping descriptions above describe the underlying projections.
If a selected source cannot establish lifecycle (including some PL and IS observations and
unsupported records in other sources), its matched record cannot pass the success gate.
