# Slovak public accounting entities

The Ministry of Finance [RÚZ Open API](https://www.registeruz.sk/cruz-public/home/api)
provides anonymous public accounting-entity JSON. It is an accounting source, not
an extract from the commercial register. No keys or accounts are required.

Accept eight ASCII IČO digits with leading zeroes retained; remove ASCII whitespace.
DIČ and VAT identifiers are distinct and are not inferred. Resolve IČO through
`uctovne-jednotky?zmenene-od=2000-01-01&ico=<ICO>&max-zaznamov=2`, requiring
one internal ID and `existujeDalsieId: false`. Fetch `uctovna-jednotka?id=<id>`
and require both exact internal ID and full IČO plus a nonblank `nazovUJ`.
Empty, ambiguous, deleted or private public data never establishes unregistration.
HTTP failures remain operational errors. Lifecycle enrichment follows a matched RÚZ record, never a failed lookup.

Map name, identifier and the official RÚZ/ŠÚSR legal-form short label. Preserve DIČ, source codes, dates,
legal-form code and source address components in typed native details. Accounting
entity address components do not establish the shared registered-office or fiscal
address role. Those fields stay null, as do VAT registration/identifier,
foreign name and company link. A valid ISO `datumZrusenia` maps to closed for the
source accounting entity; missing dates do not establish active status, and malformed
dates remain unresolved. API-record availability (`stav`) is not entity lifecycle.
For records without `datumZrusenia`, the [public RPO API](https://rpo.minv.sk/rpo-api-doc.html) supplies lifecycle through a unique exact-IČO `onlyActive=true` search and identity-matched current detail. Establishment and current identifiers are required; current CL010108 liquidation, bankruptcy and restructuring labels override ordinary active membership. Conflicting or unsupported states remain unresolved. An empty RPO search leaves the matched RÚZ observation unresolved; it never proves company absence. Typed `rpoLifecycle` retains this supplemental source separately from RÚZ fields. RPO updates nightly and may lag changes by up to 24 hours.

Native dates do not imply a historical
lookup. No financial reports, officers or owners are fetched.

A lookup has two RÚZ requests and, when needed, two RPO requests under one admission and thirty-second deadline.
Wait the configured interval after each response before the next exchange,
at least one second; defaults are
two-second spacing, three-second connect timeout, sixty-second failure cooldown
and one-MiB response cap per response. No automatic retries, redirects or proxies.
Capture and replay all source responses locally, binding supplemental captures to the review hash; synthetic tests cover ambiguity,
identity, minimal fields and the four-request anonymous transport contract.

See the [mapping audit](../../docs/development/company-mapping-audit.md) for source definitions and remaining null fields.

After successful decoding, public lookup requires resolved lifecycle and returns
`unresolved_data` for missing, unsupported or ambiguous lifecycle evidence.
Missing required envelope/identity fields remain decoding errors. Pure decoding/projection
retains the original observation. See the
[runtime mapping audit](../../docs/development/company-mapping-audit.md#runtime-enforcement).

## Access, pacing and recovery

A closed RÚZ record needs two requests. Otherwise the adapter adds an RPO search
and, only if it matches, RPO detail: up to four requests under one 30-second
deadline. An empty RPO search ends after three requests with unresolved lifecycle.
The default two-second interval applies between exchanges and after completion;
it is library policy, not a verified quota for either provider.

RPO can throttle paced anonymous lookups. Respect 429 and any `Retry-After`, allow
the cooldown, then reduce collection pressure with an explicitly bounded recovery.
Do not treat one successful recovery pace as a published or guaranteed quota.
Budget the full multi-request flow when increasing the interval or timeout, and
coordinate independent callers. Preserve the original interrupted journal.

RÚZ and RPO coverage can differ. An exact RÚZ match absent from RPO is unresolved,
not proof of closure or nonregistration. RPO transport failure remains operational;
it must not be hidden by returning an active accounting record. Replay requires
both RÚZ responses and every available RPO stage with source-bound review hashes.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.

An empty RPO search remains a source-data gap when RÚZ has a matched company but
no dissolution date. The [Ministry of Justice commercial register](https://www.orsr.sk/search_ico.asp)
can provide an independent cross-check; an empty result there does not turn the
matched RÚZ observation into a definitive negative or establish closure.

For investigation, RPO also documents historical-inclusive `onlyActive=false`
and `fullName` searches. A name search can locate a candidate but cannot replace
exact identifier binding. An empty successful search remains a data gap; a 503
or timeout is an access failure, including when RÚZ has already returned a record.

The EU's [BRIS company search](https://e-justice.europa.eu/topics/registers-business-insolvency-land/business-registers-search-company-eu_en)
can provide another manual official cross-check, subject to its interactive
antispam and access conditions. An empty BRIS search, like an empty historical
ORSR or RPO search, does not establish closure of an identity-matched RÚZ record.
Keep that source-coverage gap separate from HTTP, authentication and timeout failures.
