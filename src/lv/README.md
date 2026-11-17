# Latvian registry open data

The [Register of Enterprises](https://www.ur.gov.lv/en/registers-open-data-latvian-open-data-portal)
publishes its [daily updated dataset](https://data.gov.lv/dati/lv/dataset/uz)
on the official open-data portal. Anonymous CKAN `datastore_search` with an exact
`regcode` filter, `limit=2` and the fixed registry resource retrieves one record.
Require `success`, the expected resource ID, total one, one returned record and
exact full registration number. Zero or ambiguous results are source mismatches,
not proof of absence. API keys, bulk downloads and accounts are not required.

Accept eleven ASCII registration digits, retaining leading zeroes and stripping
ASCII whitespace only. JSON numeric codes are represented as decimal strings;
never pad a source code to force a match. VAT and personal IDs are not inferred.
Map `name`, `type_text` and the dataset's documented legal `address` text into
common name, legal form and registered-office full text. Preserve every original
address character. Structured office fields remain null: numeric location codes
and the native `index` field are retained without guessed formatting. Native
closed marker and registration/removal dates remain source facts; lifecycle is
unknown when a marker is supplied. VAT registration, VAT ID, fiscal address,
foreign name and company link remain null. SEPA creditor codes are not VAT IDs.

Default bounds are thirty-second deadline, three-second connection timeout,
two-second spacing, sixty-second failure cooldown and one-MiB body cap. No retries,
redirects or ambient proxy. HTTP errors never establish registry absence.
Synthetic tests cover identity, resource scope, ambiguity, field roles and anonymous
exact-filter transport. Collected source records and reviews remain local-only.

The private shared CKAN helper owns filter encoding and envelope uniqueness.
This adapter retains the resource constant, native identity/name checks and field mapping.
