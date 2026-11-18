# Estonian public register search

Use the anonymous autocomplete endpoint used by the official
[e-Business Register portal](https://ariregister.rik.ee/eng). Public company queries
are available without login; the separate XML service requires a contract, as
explained by [RIK](https://www.rik.ee/en/e-business-register/business-register-queries).
The adapter uses the portal's observed `eng/api/autocomplete?q=<code>&results=10`
interface. It is not a versioned XML contract or a certified full extract.

Accept eight ASCII registry-code digits; remove ASCII whitespace, preserve zeroes
and reject VAT and personal identifiers. Syntax makes no checksum claim. Require
an OK envelope and exactly one exact registry-code match, never the first search
hit. Numeric source codes become decimal strings without padding or truncation;
string source codes retain zeroes. Empty and ambiguous searches remain errors.
Require a nonblank source name. Historical names, native status and legal-form
codes remain typed details. Status R maps to active and K to closed; liquidation L and bankruptcy N
map to their distinct proceeding states, following RIK's
[published status classification](https://avaandmed.ariregister.rik.ee/en/open-data-api/detailed-company-data-query).
Verified legal-form codes 3 and 5 map to their Estonian labels.

`legal_address` supplies the original registered-office text and `zip_code` the
postcode. No delivery, locality or country components are inferred from this
text. Missing address and postcode mean no common office. Fiscal address, foreign
name, VAT registration and VAT ID remain null. A source HTTPS portal URL maps
to the common link only with the official host and exact company-code path,
without credentials, query parameters or fragments.
No historical-date or selectable-language operation is supported; source names
remain original, with the English portal route selected.

Reuse private shared transport: thirty-second total deadline, three-second connect
timeout, two-second spacing, sixty-second cooldown, one-MiB body limit. No retries,
redirects, credentials or ambient proxy. HTTP, access and schema failures never
prove absence. Synthetic tests protect exact identity and field preservation;
source observations and review journals stay local-only.

See the [mapping audit](../../docs/development/company-mapping-audit.md) for source definitions and remaining null fields.

After successful decoding, public lookup requires resolved lifecycle and returns
`unresolved_data` for missing, unsupported or ambiguous lifecycle evidence.
Missing required envelope/identity fields remain decoding errors. Pure decoding/projection
retains the original observation. See the
[runtime mapping audit](../../docs/development/company-mapping-audit.md#runtime-enforcement).

## Access, pacing and recovery

One portal autocomplete request uses the default two-second start interval;
no numerical quota has been established for this observed route. This does not
inherit the access contract or limits of RIK's separate XML service. Require an
exact registry-code match even if other search hits are returned. Portal schema
changes and login/challenge responses need investigation, not a negative review.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.
