# Irish CRO open company records

The [CRO open-data portal](https://opendata.cro.ie/) offers anonymous company
records through CKAN `datastore_search`. This is distinct from the older CRO
Open Services API requiring signup. The [CRO announcement](https://cro.ie/the-companies-registration-office-cro-announces-the-launch-of-new-open-data-portal/)
documents the public API and CC BY 4.0 data licence. No accounts, keys or bulk
registry download are needed for this adapter.

Accept one to seven ASCII company-number digits, preserving leading zeroes and
removing ASCII whitespace. This is a supported company-number syntax range,
not a claim about VAT, business names or registration. Require the fixed company
resource, `success`, total one, one record, exact `company_num` and nonblank
`company_name`. Numeric source numbers become decimal text without invented
padding. Missing or duplicate records do not prove nonregistration. The dataset
contains dissolved companies as well as current ones.

Map the published name and `company_type` description. Keep native status text,
status/type codes, dates, company-address lines and Eircode in typed details.
Normal/reinstated states map to active; terminal dissolved, struck-off and
ceased states map to closed. Official CRO codes also work without labels;
contradictory code/label pairs remain unresolved; documented proceedings map to distinct states. No tax status or VAT
identifier is inferred. CRO's company-address lines describe the statutory registered office. Map
nonblank lines in order and Eircode as postcode, without inventing city/county,
country or full-address text. Fiscal address, VAT fields, foreign name and company
link remain null. Source dates do
not add a historical lookup operation.

Use the shared bounded transport: thirty-second deadline, three-second connect
timeout, two-second spacing, sixty-second cooldown and one-MiB body cap. No
retries, redirects, cookies or ambient proxy. Synthetic tests cover exact resource
and identity, ambiguity, native fields and anonymous filter queries. Real records
and source-reviewed expectations stay local-only.

The private shared CKAN helper owns filter encoding and envelope uniqueness.
This adapter retains the resource constant, native identity/name checks and field mapping.

See the [mapping audit](../../docs/development/company-mapping-audit.md) for source definitions and remaining null fields.

After successful decoding, public lookup requires resolved lifecycle and returns
`unresolved_data` for missing, unsupported or ambiguous lifecycle evidence.
Missing required envelope/identity fields remain decoding errors. Pure decoding/projection
retains the original observation. See the
[runtime mapping audit](../../docs/development/company-mapping-audit.md#runtime-enforcement).

## Access, pacing and recovery

One filtered CKAN request uses the default two-second start interval. No
numerical CRO open-data quota has been established here; do not apply limits or
credentials from the separate Open Services product. A complete exact-identity
result can be dissolved or in proceedings. Missing rows, truncated envelopes and
contradictory status code/label pairs cannot become negative registry cases.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.
