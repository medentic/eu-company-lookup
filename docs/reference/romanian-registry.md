# Romanian taxpayer lookup

`Client::lookup_company(code, "RO")` and `RoRegistryClient::lookup` query the Romanian
National Agency for Fiscal Administration (ANAF) by CUI. This is taxpayer-register
information, not an ONRC trade-register extract. Romanian VAT IDs are separately
supported by `Client::lookup_vat` through VIES.

The adapter follows ANAF's [v9 service instructions](https://static.anaf.ro/static/10/Anaf/Informatii_R/Servicii_web/doc_WS_V9.txt)
and [service directory](https://static.anaf.ro/static/10/Anaf/Informatii_R/Servicii_web/servicii_sincron.html).
Synthetic tests cover parser and HTTP failures; authorized live observations and
reviewed expectations remain local-only. The official example is descriptive rather
than executable JSON. Observed HTTP 200 responses omit its top-level `cod` and
`message` fields, so these are optional; when present, they must indicate success.
Exact identifier/date matching and unambiguous result arrays remain mandatory.

## Input and dates

`RoBusinessId::parse` and `validate_ro_business_id_syntax` accept two to ten ASCII
digits after removing ASCII whitespace, with a 64-byte input limit. They preserve
leading zeroes and do not verify checksums or registration. The `RO` VAT prefix,
punctuation and thirteen-digit personal identifiers are rejected.

ANAF's request uses a numeric CUI. The adapter converts the validated digits to
an integer for transport and matches the response's numeric identity; the result
retains the requested identifier spelling. This conversion does not change the
string representation returned to consumers.

The default lookup uses today's date in Europe/Bucharest. `RoRegistryClient::lookup_at` accepts an explicit `eu_company_lookup::Date`; future
dates are rejected locally. Historical fiscal status is not a historical extract
of every company detail. Both the requested date and UTC observation time are
included in the result.

```no_run
use eu_company_lookup::{Client, CompanyDetails, LookupError};

# async fn example() -> Result<(), LookupError> {
let client = Client::new()?;
// Synthetic CUI; no registration claim.
let record = client.lookup_company("1234", "RO").await?;
println!("{}: {}", record.tax_id, record.company_name);
if let CompanyDetails::RomanianAnaf { details, .. } = &record.registry_details {
    println!("ANAF VAT registration: {:?}", details.tax_status.vat_registered);
}
# Ok(())
# }
```

## Result and failures

The facade returns `CompanyLookupResult` with typed ANAF `registryDetails`.
The lower-level `RoCompanyLookupResult` is separate from the Bulgarian `BgCompanyLookupResult`
and its typed `BgBusinessId`. JSON uses camelCase; `source` is `RomanianAnaf`.
The result retains company name, trade-register number, registration-status/date
text, legal form and CAEN code when supplied. Original fiscal-address text and
source structured registered-office/fiscal-address fields remain separate.
Country names use title-case display formatting. `countryISO2` is uppercase
when the supplied country is recognized, otherwise null. This applies to both
common and native ANAF addresses; original full fiscal text stays unchanged.
Missing, null or blank address countries default to `Romania` / `RO` based on
the ANAF jurisdiction, including fiscal addresses available only as raw text.
Explicit source countries take precedence. Absent addresses remain absent.
This is a library default, not an explicit ANAF observation. Translations,
managers and owners are not inferred.

`taxStatus` contains nullable `vatRegistered`, `vatOnCollection`, `inactive`,
`splitVat` and `eInvoiceRegistered` flags. Missing flags are null. Domestic VAT
registration is distinct from VIES validity; fiscal inactivity is distinct from
legal dissolution. This operation never creates a VIES `valid` result.

Only an otherwise successful response with no found records and exactly the
requested CUI in `notFound` yields
`LookupError` with ID `not_found` and `details.provider: RomanianAnaf`.
That means ANAF missed the identifier, not that every Romanian register did.
A Bulgarian definitive miss uses the same ID with reason `both_bulgarian_registers_missed`.

Success requires exactly one matching CUI and requested date. Conflicting,
duplicate, empty or malformed envelopes are errors. Operational failures,
redirects, endpoint 404s and mismatches never become negative observations.
Registry errors are sanitized and identify ANAF as the provider.

## Client policy

Use `RoRegistryConfig::default()` and assign fields. Defaults are a 30-second total
deadline, 3-second connection timeout, 2-second request spacing, 60-second failure
cooldown and 1 MiB body limit. Only one lookup is admitted across clones; occupied
capacity or spacing that cannot fit the deadline returns `Busy`. Longer valid
Retry-After instructions extend cooldowns. The adapter sends one CUI per request.

ANAF documents at most one request per second per client. Configuration rejects
spacing below one second. Independently constructed clients and processes require
application-level coordination. There are no automatic retries, redirects,
ambient proxies, cookies or persistent cache.

`Client::with_registry_configs(vies, bulgarian, romanian)` configures all three
clients. Existing constructors continue to work and use Romanian defaults.
No credentials, database, bulk collection or ONRC fallback are added.

## Shared company lifecycle

`status` uses the shared `CompanyStatus` enum. Exact `INREGISTRAT` and `RELUARE ACTIVITATE` map to active;
`SUSPENDARE ACTIVITATE` maps to inactive. `RADIERE`, `RADIAT` and
`DIZOLVARE FARA LICHIDARE(FUZIUNE)` map to closed. An optional `din data` suffix
must contain a strict day.month.year date. `TRANSFER(SOSIRE)` does not establish
a company lifecycle. Other descriptions remain unknown; absent status is null.
`registryDetails.data.nativeRegistryStatus` in the shared result exposes the original ANAF status text,
including when normalized; it is null when absent. `registrationStatus` retains
the same original text for compatibility. Lower-level native results retain
top-level `nativeRegistryStatus`. Fiscal inactivity and VAT
registration remain separate tax flags and never imply company closure or activity.

## Shared VAT fields

`CompanyLookupResult.vat_registered` (`vatRegistered`) reports standard domestic
VAT registration for the matched fiscal identifier at `requestDate`. It copies
ANAF's nullable `scpTVA` flag; the source flag also remains in the native tax status.
The [ANAF v9 specification](https://static.anaf.ro/static/10/Anaf/Informatii_R/Servicii_web/doc_WS_V9.txt)
limits this registration check to article 316. False does not rule out special
article-317 registration, and neither value is a VIES observation.

When this flag is explicitly true, `vatId` is `RO` plus the matched numeric fiscal
ID, validated against the supported VAT syntax. `taxId` retains caller-supplied
leading zeroes; the derived VAT ID uses the numeric identity matched by ANAF.
For example, synthetic `taxId: "001234"` produces `vatId: "RO1234"` only after a
positive matched registration. False/missing flags or an unusable numeric VAT
identity leave `vatId` null. No additional network call is made.

This is not a rule for arbitrary business IDs. The
[ANAF VAT return instructions](https://static.anaf.ro/static/10/Anaf/legislatie/OPANAF_77_2022.pdf)
describe a VAT identifier as the fiscal identifier prefixed with RO, but
[ANAF's public-institution guidance](https://static.anaf.ro/static/10/Anaf/AsistentaContribuabili_r/Brosura_TVA_Instituii_Publice.pdf)
explains that separate taxable activities or special registrations can have a
different numeric identifier from the institution's ordinary fiscal ID. Never
infer or merge those alternate registrations from the requested identifier.
