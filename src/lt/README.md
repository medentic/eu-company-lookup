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

Common output maps identifier and published name. Native full-address text,
address/form/status references and registration/removal/status dates are retained
without resolving referenced records or guessing common field meanings. Office,
fiscal address, lifecycle, legal form, VAT fields, foreign name and company link
remain null. This limited source projection is explicit; it does not invent
missing address components or current status from source identifiers or dates.

Transport uses the shared bounded defaults: thirty-second deadline, three-second
connect timeout, two-second start spacing, sixty-second cooldown and one-MiB
response cap. No automatic retries, redirects or ambient proxies. Source captures
and reviewed expectations stay local-only. Synthetic tests cover dataset scope,
unique identity, preserved references and anonymous exact-code requests.
