# Polish KRS current business extracts

Use the Ministry of Justice's anonymous [KRS open API](https://prs.ms.gov.pl/krs/openApi),
with `OdpisAktualny/<KRS>?rejestr=P&format=json`. The Ministry
[announces public API access](https://www.gov.pl/web/sprawiedliwosc/uruchomienie-otwartego-api-krajowego-rejestru-sadowego)
without requiring caller keys. This adapter targets the business register P;
association register S, CEIDG sole traders and NIP-only searches are outside scope.

Accept ten ASCII KRS digits, preserving zeroes and removing ASCII whitespace.
Syntax does not verify registration. NIP, REGON and VAT identifiers are distinct;
no prefix inference or truncation is performed. Require `rodzaj: Aktualny`, header
`rejestr: RejP`, an exact `numerKRS`, and a nonblank subject name. HTTP 404 and 204
remain source errors; neither proves absence from other Polish registers. No
fallback to other registers or implicit full-history query is performed.

Map source name and legal-form label directly. Keep NIP and the complete REGON in
typed subject details; neither establishes VAT registration. Common VAT fields,
lifecycle, aggregate native status, fiscal address, foreign name and registry link
remain null. Source header dates and entry-position codes remain native facts.
The adapter exposes the current extract, not caller-selected historical data.

The `siedzibaIAdres.adres` object supplies registered-office street/building,
separate unit, locality, postcode and explicit country. Normalize country display
using shared rules. Original component text remains in typed details. No complete
address string is invented, and seat-region codes/names are not substituted for
missing address fields. The native seat and address remain distinct. Expose only
selected first-section and header fields; no officer or ownership API is added.

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
