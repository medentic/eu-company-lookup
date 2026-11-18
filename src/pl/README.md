# Polish KRS business and association extracts

Use the Ministry of Justice's anonymous [KRS open API](https://prs.ms.gov.pl/krs/openApi),
with `OdpisAktualny/<KRS>?rejestr=P&format=json`. The Ministry
[announces public API access](https://www.gov.pl/web/sprawiedliwosc/uruchomienie-otwartego-api-krajowego-rejestru-sadowego)
without requiring caller keys. This adapter queries business register P first, then association register S only
after P returns HTTP 404. S includes associations, foundations, other organizations
and public healthcare institutions. CEIDG sole traders and NIP-only searches are
outside scope. A successful P result takes precedence when an entity appears in both.

Accept ten ASCII KRS digits, preserving zeroes and removing ASCII whitespace.
Syntax does not verify registration. NIP, REGON and VAT identifiers are distinct;
no prefix inference or truncation is performed. Require `rodzaj: Aktualny`, header
`rejestr: RejP` or `RejS` matching the requested route, an exact `numerKRS`, and a
nonblank subject name. A second 404 from S remains a source error. HTTP 204 triggers one bounded full-extract
request for the same KRS and register; neither status proves absence. Full-extract recovery
requires an exact `Pełny` identity and matching register, ordered entry numbers, a terminal
`WYKREŚLENIE Z KRAJOWEGO REJESTRU SĄDOWEGO` entry and valid removal/finality dates
no later than the source snapshot. No successor lookup is performed. The P-to-S fallback shares admission and the
total deadline, and is never triggered by an outage, malformed body or unresolved status. An explicit non-final removal maps to `closed`, retaining the native
non-finality marker. This is the consumer policy for `closed`, not a claim of
final legal removal; contradictory final/non-final evidence remains unresolved. Incomplete or unsupported full history remains a source error.

Map source name and legal-form label directly. Keep NIP and the complete REGON in
typed subject details; neither establishes VAT registration. Common VAT fields,
aggregate native status, fiscal address, foreign name and registry link
remain null. Source header dates and entry-position codes remain native facts.
The adapter exposes the current extract or confirmed final-removal recovery, not
caller-selected historical data. Recovery retains `fullExtractRemoval` and selected
identity fields valid at removal; it does not rewrite the requested company as its successor.

The `siedzibaIAdres.adres` object supplies registered-office street/building,
separate unit, locality, postcode and explicit country. Normalize country display
using shared rules. Original component text remains in typed details. No complete
address string is invented, and seat-region codes/names are not substituted for
missing address fields. The native seat and address remain distinct. Expose only
selected first-section, header, sixth-section proceeding, suspension and reorganization fields; no officer or ownership API is added.

Transport uses shared private bounds: thirty-second deadline, three-second connect
timeout, two-second spacing, sixty-second cooldown and one-MiB body cap. No keys,
redirects, retries or ambient proxy. Synthetic tests cover header/identity and
field projection. Live IDs, full source captures and reviewed expectations remain
local-only; request counts alone never establish verified coverage.

Foreign branches use the source's distinct `danePodmiotuZagranicznego` and
`siedzibaIAdresPodmiotuZagranicznego` sections. Accept exactly one consistent
subject/address family; retain `foreignBranch` and foreign undertaking fields in
typed details. The Polish branch keeps its full KRS identity. Its foreign parent
name is not the common foreign-script name, and the foreign head-office address
never replaces the Polish branch address.

After successful decoding, public lookup requires resolved lifecycle and returns
`unresolved_data` for missing, unsupported or ambiguous lifecycle evidence.
Missing required envelope/identity fields remain decoding errors. Pure decoding/projection
retains the original observation. See the
[runtime mapping audit](../../docs/development/company-mapping-audit.md#runtime-enforcement).

The current extract's sixth section maps a single explicit ongoing liquidation,
bankruptcy or restructuring proceeding. Competing proceedings, missing opening facts and unsupported ending/proceeding
types remain unresolved. A dated `UMORZENIE POSTĘPOWANIA UPADŁOŚCIOWEGO`
ending, or a district-court order with a `GUP` docket in the structured ending
field, ends the bankruptcy restriction only with valid opening ≤ ending ≤ snapshot
dates. Complete current registration and no conflicting bankruptcy name are still
required for `active`; the ending does not
itself establish active status or closure. See [Bankruptcy Act Articles 363–364](https://eli.gov.pl/api/acts/DU/2025/614/text/U/D20250614Lj.pdf). Ending
liquidation is not automatically deletion. Foreign-branch proceedings remain
unresolved because the section can describe the parent or branch. Neither an empty
sixth section nor `stanPozycji` alone proves active status. Field meanings follow the
[official register specification](https://eli.gov.pl/api/acts/DU/2026/649/text/I/D20260649.pdf).

Suspension/resumption dates in `zawieszenieWznowienie` establish `inactive` only
when a valid, nonoverlapping suspension interval includes the header `stanZDnia`.
Use the source snapshot date, not the replay date. Future suspensions and completed
intervals alone do not prove active status. Missing/malformed dates, overlapping
intervals and competing proceeding evidence remain unresolved.

## Access, pacing and recovery

Current and supplemental full-extract requests share a deadline/admission lock.
The default gap after completion and between exchanges is two seconds;
no numerical API quota has been established here. The public search frontend and
open API are separate surfaces: a frontend HTTP 403 does not establish that the
extract API is unavailable. Retain the failing endpoint with access diagnostics.

A complete identity-matched current extract maps to `active` when its registration,
last-entry and snapshot dates are valid and ordered, its section six is explicitly
present without unresolved restrictions, and no outgoing/conflicting merger or
liquidation/bankruptcy name contradicts that inference. The implementation also
accepts the observed current-register position values `1`/`4` for P and `3`/`4` for S as a
conservative guard, not as independent active-status codes. Association records
returned under `RejP` follow the same current-registration rule. Unrecognized section-six field names are retained in
`unmappedLifecycleFields` and prevent this inference; their contents are not exposed.

This combines current-registration evidence with absence of reported restrictions,
following [the current-extract definition, §7(2)](https://eli.gov.pl/api/acts/DU/2025/201/text/I/D20250201.pdf).
It means registered and unrestricted in KRS, not proof of trading, solvency or VAT
registration. For foreign branches it describes the Polish branch registration only.
A historical acquisition or transformation does not require another lookup. An
unresolved current response returns `unresolved_data`; full history is requested
only after HTTP 204 in the selected register, where it can establish removal. Unsupported contradictory liquidation fields require source clarification, not retries.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.

## Mergers and successor identities

The [official register specification, section-six reorganization fields](https://eli.gov.pl/api/acts/DU/2026/649/text/I/D20260649.pdf)
distinguishes acquisition of another company (`PRZEJĘCIE INNEJ SPÓŁKI`) from
acquisition by another company (`PRZEJĘCIE PRZEZ INNĄ SPÓŁKĘ`). Preserve the
structured source description and counterparties in `corporateEvents`, with
separate roles for partial divisions, asset recipients and transformations.
A generic `POŁĄCZENIE` entry identifies the resulting company only when its
structured receiver KRS equals the requested identity. An old acquisition or
proposal does not establish present activity or completed closure.

For absorbed-party events, `successorKrs` retains distinct, exact ten-digit
receiver KRS values. All-zero placeholders, malformed identifiers and self-links
are not promoted to successors; native fields remain available. Partial asset
receivers are not presented as merger successors. Missing counterpart identifiers
are not inferred from names, REGON or free-text descriptions. A receiver may itself
later close or merge; no automatic successor-chain traversal is performed.

Under [Commercial Companies Code Articles 492–494](https://eli.gov.pl/api/acts/DU/2024/18/text/U/D20240018Lj.pdf),
the absorbed company or companies forming a new company are dissolved through
registered merger/removal. A final removal entry establishes `closed`, even when
the history also contains old acquisitions or liquidation proceedings. The full
extract decoder selects records present at terminal removal, excluding earlier
withdrawn entries; historical role is not a reason to override confirmed removal.


## Reconciled liquidation histories

[Commercial Companies Code Articles 272–276](https://eli.gov.pl/api/acts/DU/2024/18/text/U/D20240018Lj.pdf)
distinguish removal, a resolution for continued existence, and ongoing liquidation.
For limited-liability companies, a registered affirmative continued-existence
resolution can end a liquidation period. Require valid ordered opening/ending dates,
a dated ending no later than the snapshot, and a current name without the liquidation
designation. When a dissolution description remains, it must repeat one of the
openings and every liquidation period must have such an ending. The ordinary
complete-current-registration checks still apply before returning `active`.
When the opening has been withdrawn, an affirmative resolution explicitly for
continued existence, cancellation of liquidation and dismissal of the liquidator
can also reconcile the ending. Require a dated resolution no later than the
snapshot, consistent repeated dates, no current liquidators and no liquidation
name. Negated, proposed, incomplete or future resolutions do not qualify.
An ending, a restored name or a retained liquidator entry alone is insufficient.

An identical court order in the opening and ending fields can be a duplicated
opening, rather than an actual ending. Resolve it as `liquidation` only when the
order explicitly dissolves the company and appoints a liquidator, its date is no
later than the snapshot, a current liquidator entry exists and the registered name
ends in `W LIKWIDACJI`. Other conflicting descriptions remain unresolved.

Preserve selected `rozwiazanieUniewaznienie` text and `liquidatorCount` in typed
details. The count is derived from source entries; identities are not exposed.
These rules use the current response. Full histories were used to review their
meaning, not added as routine runtime calls. See the executable
[liquidation examples](../../docs/examples/pl.md#continued-existence-after-liquidation).


## Register routing and HTTP 404

[P and S are separate KRS registers](https://www.gov.pl/web/gov/uzyskaj-informacje-z-krajowego-rejestru-sadowego).
An organization conducting business can be registered in both. The client prefers
P and tries S only after a P-route 404. It never tries S after a successful P
response, unsupported lifecycle, parse error, timeout, throttling or other HTTP
failure. A P 204 goes directly to full P, not S; an S 204 goes to full S.
At most three requests are needed: current P, current S, full S.
All exchanges share the deadline, admission lock and spacing policy.

A 404 from both routes remains an error, not proof of company closure or absence.
Each parser requires the exact register it requested: P data cannot satisfy an S
request or vice versa. Native `naglowek.rejestr` preserves the source register.
Historical error observations remain in local journals; recovery creates new
source-bound observations rather than rewriting them.

S uses `opisPodzialuPolaczeniaPrzeksztalcenia` for some merger descriptions and
`identyfikator.regon` on counterparties. Preserve these fields without converting
REGON to KRS or extracting IDs from prose. A generic S `POŁĄCZENIE` history with
named receiver entries but no structured KRS does not invalidate complete current
registration evidence merely because transaction direction is unresolved. Explicit
outgoing merger roles, unknown event types and unsupported restrictions still
block an active inference. Company lifecycle and historical transaction role are
separate facts. S registration does not imply commercial activity or VAT status.


## Historical transformations and acquisitions

The exact `PRZEKSZTAŁCENIE` label denotes a transformation, including public
healthcare entities. A generic `POŁĄCZENIE` with acquired parties and no receiving
parties identifies the acquirer unless a structured KRS contradicts that role.
[Healthcare Act Articles 66–67](https://eli.gov.pl/api/acts/DU/2025/450/text/T/D20250450L.pdf)
distinguish transfer into an existing entity from formation of a new entity.
Neither historical event alone proves closure. Conversely, an absorbed label
combined with acquired-party entries is conflicting transaction evidence; retain
an unresolved event role and do not invent successors. An independently supported
current restriction, such as restructuring, can still resolve company lifecycle.

## Verification milestone

KRS P/S national lookup meets the repository's reviewed 200-distinct-ID gate:
live source observations, source-bound expectations and complete offline replay.
Coverage includes active, closed, liquidation, restructuring, suspension and
bankruptcy states. Original failed or unresolved observations remain in local
audit journals alongside subsequent resolution evidence. This milestone applies
to KRS P/S; it does not imply CEIDG coverage, provider availability or publication.

## TODO: additional Polish registries

- [ ] **CEIDG:** add sole-trader lookup after reviewing official access, supported
  identifiers, authentication and rate limits. Map registration, suspension,
  resumption and cessation from authoritative fields; add synthetic tests and
  source-bound positive/negative replay cases.
- [ ] **KRZ / legacy RDN:** investigate debtor and insolvency coverage, using KRZ
  for modern records and assessing whether legacy RDN access is needed. Confirm
  official access and identifier matching, retain proceedings separately from
  company lifecycle, and test missing records and access failures without
  treating either as company closure. See the
  [EU insolvency-register guide](https://e-justice.europa.eu/topics/registers-business-insolvency-land/bankruptcy-and-insolvency-registers/pl_pl).

These are backlog items; the implemented lookup remains KRS P/S.
