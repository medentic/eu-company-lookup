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
native text. Lifecycle, VAT fields, fiscal address, foreign name and company link
remain null. Iceland is outside VIES; absence of a displayed VAT table is not
interpreted as a negative domestic VAT registration.

Transport accepts bounded UTF-8 HTML, with the shared thirty-second deadline,
three-second connection timeout, two-second spacing and sixty-second cooldown.
Body cap defaults to one MiB. No retries, redirects or proxies. Changed/missing
page structure, login/challenge pages and HTTP errors are operational failures,
not proof of no company. Synthetic tests enforce identity, table labels, legal
versus postal address and ignored personal/document content. Real pages and
source review evidence remain local-only.
