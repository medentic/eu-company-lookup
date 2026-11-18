# Lithuanian legal-entity open data

The [Registrų centras dataset](https://data.gov.lt/datasets/1484/) is published
through the official open-data store. Query `JuridinisAsmuo?ja_kodas=<code>&limit(2)`
anonymously, require exactly one record, the expected `_type`, exact company code
and a nonblank published name. This is a periodically updated dataset, not a
certified live register extract. The dataset includes removed entities and some
entity types have anonymized names; preserve the published name without claiming
it is always a complete legal name. No credentials or bulk download are needed.

Accept modern nine-digit ASCII company codes, retaining leading zeroes and removing
ASCII whitespace. Legacy seven-digit codes are outside this adapter's scope.
Numeric source codes become decimal strings; no zero-padding to force a match.
VAT identifiers are distinct and never inferred. Missing or duplicate dataset
records remain mismatches, not definitive absence.

Common output maps identifier, published name and `pilnas_adresas` (the
documented registration address) to original registered-office text. A valid
`isreg_data` deregistration date supplies closed status; missing dates never
imply active. Status references resolve through the static official
[status classification](https://data.gov.lt/datasets/1624/versions/1213/api/getall/Statusas/):
normal/no-proceedings maps to active; explicit removal/liquidation completion to
closed; bankruptcy, liquidation, restructuring and initiated liquidation have
separate states. Corporate transformations and unrecognized references remain
unresolved. A malformed supplied cessation date blocks success. Original references
stay native; classification labels populate native status. Address/form references
remain unresolved. No additional runtime requests are made. Fiscal address, legal form, VAT fields,
foreign name and company link remain null.

Transport uses the shared bounded defaults: thirty-second deadline, three-second
connect timeout, two-second start spacing, sixty-second cooldown and one-MiB
response cap. No automatic retries, redirects or ambient proxies. Source captures
and reviewed expectations stay local-only. Synthetic tests cover dataset scope,
unique identity, preserved references and anonymous exact-code requests.

See the [mapping audit](../../docs/development/company-mapping-audit.md) for source definitions and remaining null fields.

After successful decoding, public lookup requires resolved lifecycle and returns
`unresolved_data` for missing, unsupported or ambiguous lifecycle evidence.
Missing required envelope/identity fields remain decoding errors. Pure decoding/projection
retains the original observation. See the
[runtime mapping audit](../../docs/development/company-mapping-audit.md#runtime-enforcement).

## Access, pacing and recovery

One exact-code dataset request uses the default two-second start interval; no
numerical provider quota has been established here. Status classification is local
and adds no per-company request. Refresh the reviewed official classification
when it changes; a new reference or transformation state is a mapping gap, not
proof of inactivity. Publication scope and anonymized names remain source limits,
even when HTTP and identity checks pass.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.

Cessation evidence requires an exact `YYYY-MM-DD` date or
`YYYY-MM-DDTHH:MM:SS` timestamp. Malformed or loosely padded supplied dates
remain unresolved. Missing dates alone do not establish active status.
Unrecognized supplied status references remain native status identifiers and use
`unmapped_or_ambiguous_lifecycle`, rather than being described as missing evidence.
