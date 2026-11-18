# Icelandic public company-register pages

Use the tax administration's anonymous [company register](https://www.skatturinn.is/fyrirtaekjaskra/)
HTML detail at `/fyrirtaekjaskra/leit/kennitala/<kennitala>`. Require one company
heading with the exact requested identifier and the labelled address/form table.
Parse only company identity, registration subtitle, postal/legal address,
municipality and legal form; officers, owners and document-ordering controls
are outside the returned data. No account, API key or payment is needed.

Accept canonical ten-digit ASCII legal-entity kennitala, removing ASCII whitespace.
The first two digits must be 41–71, excluding individual identifiers, following
Iceland's [published TIN structure](https://www.oecd.org/content/dam/oecd/en/topics/policy-issue-focus/aeoi/iceland-tin.pdf).
This is syntax only, without checksum or registration assurance; hyphenated input
is outside the supported canonical format. No VAT identifier is inferred.

`Lögheimili` supplies registered-office full text. Preserve the decoded cell text
without normalizing its whitespace; keep postal address and municipality separate
in native details. Structured office fields stay null. Extract legal-form code
and label only from the source's two explicit text segments; otherwise preserve
the complete native cell without guessing a label. Registration subtitle remains
native text. VAT fields, fiscal address, foreign name and company link
remain null. Iceland is outside VIES; absence of a displayed VAT table is not
interpreted as a negative domestic VAT registration.

Transport accepts bounded UTF-8 HTML, with the shared thirty-second deadline,
three-second connection timeout, two-second spacing and sixty-second cooldown.
Body cap defaults to one MiB. No retries, redirects or ambient proxies. Changed/missing
page structure, login/challenge pages and HTTP errors are operational failures,
not proof of no company. Synthetic tests enforce identity, table labels, legal
versus postal address and ignored personal/document content. Real pages and
source review evidence remain local-only.

After successful decoding, public lookup requires resolved lifecycle and returns
`unresolved_data` for missing, unsupported or ambiguous lifecycle evidence.
Missing required envelope/identity fields remain decoding errors. Pure decoding/projection
retains the original observation. See the
[runtime mapping audit](../../docs/development/company-mapping-audit.md#runtime-enforcement).

An exact `Félag afskráð DD.MM.YYYY` notice attached to the registration subtitle
maps to closed after strict date validation. Text elsewhere, including VAT removal,
is not company closure. An ordinary registration date does not establish active status.

## Free registration-overview fallback

When the matched public page has unresolved lifecycle and offers the explicit free
overview button for the same identifier, fetch Skatturinn's
[registration overview](https://www.skatturinn.is/fyrirtaekjaskra/thjonusta/upplysingar-ur-fyrirtaekjaskra/).
The document supplies a separate `Staða` field. Exact `Virk skráning` maps to active;
PDF text extraction can omit visual spaces, so comparison normalizes whitespace
only. Other status wording remains unresolved. The decoded status, printed date
and matched kennitala stay in typed `registrationOverview` details; document
contents, officers and personal identifiers are not returned. Missing overviews
are omitted from serialized details, preserving the page-only output shape.

The adapter requests only free item type `9`, checks a single exact-identifier
cart row with zero price, and accepts only the source's zero-amount confirmation
for that cart. It follows the observed form protocol without executing scripts.
Cart URLs are restricted to the official document-shop host/path; the follow-up
routes are constructed locally. Paid certificates, changed forms, multiple items,
nonzero prices, changed origins and mismatched document identities are errors.
This is a provider-specific document retrieval, not a payment integration.

The document shop requires session cookies and same-origin form headers. An
isolated cookie jar exists only for one lookup and is discarded afterwards;
it never imports browser cookies or shares sessions across client clones.
Only the first PDF page is decoded, with bounded stream decompression and text
extraction on a blocking worker. Raw diagnostics, when explicitly enabled, can
include the source document; ordinary outputs contain only selected fields.

## Access, pacing and recovery

A resolved page needs one HTTP request. The free-overview fallback adds five:
create the free cart, read it, submit its form, acknowledge the zero-cost document
request, then download the PDF. Every exchange uses the configured interval
(default two seconds), one shared deadline (default 30 seconds), and a separate
body cap (default one MiB). Completion spacing is shared across client clones.
These are client bounds, not published provider quotas.

A missing lifecycle field is a data gap. Login/challenge pages, HTTP errors,
timeouts and document-download failures remain operational failures; none proves
closure. There is no automatic paid or authenticated fallback. The separately
registered [official API](https://api.skatturinn.is/product) is not used.

Replay binds the primary page and every overview response to the review hash.
Older page-only captures remain unresolved unless they contain explicit closure;
a newer overview must never be silently attached to an older observation.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.
