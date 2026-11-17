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
codes remain typed details. A supplied status maps to unknown lifecycle; no
active/closed inference or generated legal-form label.

`legal_address` supplies the original registered-office text and `zip_code` the
postcode. No delivery, locality or country components are inferred from this
text. Missing address and postcode mean no common office. Fiscal address, foreign
name, common legal form, VAT registration and VAT ID remain null. The source URL
remains native; no common record link is asserted without a validated link policy.
No historical-date or selectable-language operation is supported; source names
remain original, with the English portal route selected.

Reuse private shared transport: thirty-second total deadline, three-second connect
timeout, two-second spacing, sixty-second cooldown, one-MiB body limit. No retries,
redirects, credentials or ambient proxy. HTTP, access and schema failures never
prove absence. Synthetic tests protect exact identity and field preservation;
source observations and review journals stay local-only.
