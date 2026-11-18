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
closed marker and registration/removal dates remain source facts. A valid
explicit removal date maps to closed; an unverified nonblank marker alone stays unknown.
Base records alone cannot establish active status. For nonremoved records, three exact-identity CKAN queries check current liquidation, economic-activity suspension and insolvency datasets. Complete empty results, a valid registration date and a blank closure marker establish active; current liquidation maps to liquidation and economic-activity suspension to inactive. Liquidation takes precedence when both are present. Unclassified ongoing insolvency remains unresolved. Typed `lifecycle` preserves all supplemental rows. Truncated, mismatched or failed queries never establish absence. VAT registration, VAT ID, fiscal address,
foreign name and company link remain null. SEPA creditor codes are not VAT IDs.

Default bounds are thirty-second deadline, three-second connection timeout,
two-second spacing, sixty-second failure cooldown and one-MiB body cap. No retries,
redirects or ambient proxy. HTTP errors never establish registry absence.
Synthetic tests cover identity, resource scope, ambiguity, field roles and anonymous
exact-filter transport. Collected source records and reviews remain local-only.

The private shared CKAN helper owns filter encoding and envelope uniqueness.
This adapter retains resource constants, supplemental completeness and identity checks, and field mapping. Supplemental capture hashes bind every response to its review.

See the [mapping audit](../../docs/development/company-mapping-audit.md) for source definitions and remaining null fields.

After successful decoding, public lookup requires resolved lifecycle and returns
`unresolved_data` for missing, unsupported or ambiguous lifecycle evidence.
Missing required envelope/identity fields remain decoding errors. Pure decoding/projection
retains the original observation. See the
[runtime mapping audit](../../docs/development/company-mapping-audit.md#runtime-enforcement).

## Access, pacing and recovery

A closed base record needs one request. Other records require the base query plus
three lifecycle queries: current liquidation, suspension and insolvency. All four
share the 30-second deadline; the default two-second interval also applies between
exchanges and after completion. Increasing pacing must leave time for every stage.
No numerical CKAN quota has been established here.

Each supplemental response must match its resource, exact registration filter and
identity, with a complete result count. A failed or truncated query is not an empty
proceeding list. Preserve all supplemental bodies and bind their hashes to the
review; replaying only the base body cannot validate an enriched result. Liquidation
and economic-activity suspension can coexist; liquidation takes precedence while
both observations remain native.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.

Cessation evidence requires an exact `YYYY-MM-DD` date or
`YYYY-MM-DDTHH:MM:SS` timestamp. Malformed or loosely padded supplied dates
remain unresolved. Missing dates alone do not establish active status.
