# Romanian taxpayer lookup

`Client::lookup_ro_company` and `RoRegistryClient::lookup` query the Romanian
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

The default lookup uses today's date in Europe/Bucharest. `lookup_ro_company_at`
and `RoRegistryClient::lookup_at` accept an explicit `chrono::NaiveDate`; future
dates are rejected locally. Historical fiscal status is not a historical extract
of every company detail. Both the requested date and UTC observation time are
included in the result.

```no_run
use eu_company_lookup::{Client, LookupError};

# async fn example() -> Result<(), LookupError> {
let client = Client::new()?;
// Synthetic CUI; no registration claim.
let record = client.lookup_ro_company("1234").await?;
println!("{}: {}", record.tax_id, record.company_name);
println!("ANAF VAT registration: {:?}", record.tax_status.vat_registered);
# Ok(())
# }
```

## Result and failures

`RoCompanyLookupResult` is separate from the Bulgarian `BgCompanyLookupResult`
and its typed `BgBusinessId`. JSON uses camelCase; `source` is `RomanianAnaf`.
The result retains company name, trade-register number, registration-status/date
text, legal form and CAEN code when supplied. Original fiscal-address text and
source structured registered-office/fiscal-address fields remain separate.
No address country, translation, managers or owners are inferred.

`taxStatus` contains nullable `vatRegistered`, `vatOnCollection`, `inactive`,
`splitVat` and `eInvoiceRegistered` flags. Missing flags are null. Domestic VAT
registration is distinct from VIES validity; fiscal inactivity is distinct from
legal dissolution. This operation never creates a VIES `valid` result.

Only an otherwise successful response with no found records and exactly the
requested CUI in `notFound` yields
`LookupError::RegistryNotFound { provider: Provider::RomanianAnaf }`.
That means ANAF missed the identifier, not that every Romanian register did.
Existing `LookupError::NotFound` retains its Bulgarian meaning.

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

`status` uses the same `CompanyStatus` enum as Bulgarian registry results. Until
ANAF registration descriptions have verified lifecycle mappings, a supplied
`registrationStatus` yields `Unknown` (JSON `unknown`); absent status yields null.
`nativeRegistryStatus` consistently exposes the original ANAF status text,
including when normalized; it is null when absent. `registrationStatus` retains
the same original text for compatibility. Fiscal inactivity and VAT
registration remain separate tax flags and never imply company closure or activity.
