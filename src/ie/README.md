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
Lifecycle remains unknown when source status is present. No tax status or VAT
identifier is inferred. The dataset's generically labelled address lines remain
native until their exact common office/fiscal role is established; shared address
fields, VAT fields, foreign name and company link remain null. Source dates do
not add a historical lookup operation.

Use the shared bounded transport: thirty-second deadline, three-second connect
timeout, two-second spacing, sixty-second cooldown and one-MiB body cap. No
retries, redirects, cookies or ambient proxy. Synthetic tests cover exact resource
and identity, ambiguity, native fields and anonymous filter queries. Real records
and source-reviewed expectations stay local-only.

The private shared CKAN helper owns filter encoding and envelope uniqueness.
This adapter retains the resource constant, native identity/name checks and field mapping.
