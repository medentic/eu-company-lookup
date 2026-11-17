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
HTTP failures remain operational errors; there is no cross-source fallback.

Map name and identifier only to common facts. Preserve DIČ, source codes, dates,
legal-form code and source address components in typed native details. Accounting
entity address components do not establish the shared registered-office or fiscal
address role. Those fields stay null, as do legal form, VAT registration/identifier,
lifecycle, foreign name and company link. Native dates do not imply a historical
lookup. No financial reports, officers or owners are fetched.

A lookup has two bounded requests under one admission and thirty-second deadline.
Space their starts by the configured interval, at least one second; defaults are
two-second spacing, three-second connect timeout, sixty-second failure cooldown
and one-MiB response cap per response. No automatic retries, redirects or proxies.
Capture and replay both source responses locally; synthetic tests cover ambiguity,
identity, minimal fields and the two-request anonymous transport contract.
