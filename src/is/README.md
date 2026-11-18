# Icelandic public company-register pages

Use the tax administration's anonymous [company register](https://www.skatturinn.is/fyrirtaekjaskra/)
HTML detail at `/fyrirtaekjaskra/leit/kennitala/<kennitala>`. Require one company
heading with the exact requested identifier and the labelled address/form table.
Parse only company identity, registration subtitle, postal/legal address,
municipality and legal form; officers, owners and document-ordering controls
are outside the returned data. No account, cookies, keys or purchases are needed.

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
Body cap defaults to one MiB. No retries, redirects or proxies. Changed/missing
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
is not company closure. A registration date alone remains unresolved. Synthetic
notice coverage does not establish live lifecycle coverage. The fuller
[official API](https://api.skatturinn.is/product) requires registered access;
it is not an anonymous fallback for this adapter.

## Access, pacing and recovery

One public HTML request uses the default two-second start interval; no numerical
quota has been established for that route. The fuller API has a separate
[registered-user access contract](https://api.skatturinn.is/product), including
credentials and service-defined quotas. This client does not implement that API.
A matched page without a lifecycle notice is an evidence gap (`unresolved_data`),
not an access blocker. A login/challenge page or HTTP 401/403 is a different failure.
The absence of a deregistration notice cannot by itself establish active status.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.

## Evidence needed to resolve an ordinary registration page

The government's [company-formation guidance](https://www.stjornarradid.is/verkefni/atvinnuvegir/vidskipti/felagarettur-og-skraning-i-fyrirtaekjaskra/stofnun-felaga/)
says deregistered businesses are absent from the website, but that page was last
updated in 2014 and discusses name availability. It does not establish a current
lifecycle guarantee for this direct-identifier endpoint. Do not use it to turn
an ordinary registration date into an active status.

Skatturinn also offers a [free registration overview](https://www.skatturinn.is/fyrirtaekjaskra/thjonusta/upplysingar-ur-fyrirtaekjaskra/)
through its document shop. That is a separate workflow, not a lifecycle field in
the public HTML response. An overview or API integration must establish exact
identity, status semantics and observation date before it can supplement this
adapter. An unsuccessful document retrieval does not establish company closure.
