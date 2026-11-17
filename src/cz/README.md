# Czech ARES adapter work

[IČO syntax validation](identifiers.rs), the bounded [GET client](client.rs),
[typed source models](model.rs) and [identity decoder](decode.rs) are implemented.
Czech company lookup is enabled in the facade and capability catalog.
The [shared projection](mapping.rs) keeps unsupported VAT and lifecycle facts null.
Broader live verification remains in progress.

ARES is the Czech Ministry of Finance's aggregation service for economic subjects,
not a certified commercial-register extract. The implementation source contract is
its [OpenAPI specification](https://ares.gov.cz/ekonomicke-subjekty-v-be/rest/v3/api-docs).
The native lookup is `GET /ekonomicke-subjekty/{ico}` under
`https://ares.gov.cz/ekonomicke-subjekty-v-be/rest`.

## Identity boundary

Accept exactly eight ASCII digits after removing ASCII whitespace; preserve all
leading zeroes. Syntax validation does not verify checksum or registration.
Require the response `ico` to equal the requested identifier. The source's
`icoId` also admits `ARES_` synthetic identities for subjects without assigned IČO;
these are outside this adapter's supported scheme. Do not accept VAT-prefixed DIČ
or personal tax numbers as IČO.

## Mapping constraints

- `obchodniJmeno` supplies the company name; `sidlo` is the registered-office
  address. Preserve `sidlo.textovaAdresa` verbatim.
- `adresaDorucovaci` is a delivery address, not evidence of a fiscal address.
- `pravniForma` and `pravniFormaRos` are native codes, not legal-form labels.
  Keep codes in typed details until an official code-list mapping is implemented.
- `dic` is a tax identifier. Its presence alone does not establish VAT registration.
  `dicSkDph` describes a VAT group identifier and must not silently replace the
  subject's own VAT identity.
- `seznamRegistraci` describes source registrations. Do not interpret a source's
  `AKTIVNI` value as proof of company lifecycle or standard domestic VAT status
  without a documented source-specific rule.
- Preserve dates and source identifiers in typed details. Do not infer incorporation
  or closure merely from response freshness.
- Define definitive miss handling from documented error codes and observed response
  envelopes before exposing a negative registry outcome; an HTTP failure alone
  does not establish absence.

Raw observations, identifier queues, source snapshots and review history remain
local-only. They are not synthetic fixtures or evidence of completed coverage.

## Native transport

Clones share one admission lock, a two-second start interval and a sixty-second
failure cooldown. Total deadline is thirty seconds, connection deadline three
seconds and body limit one MiB. Redirects and retries are disabled. HTTP 404 becomes a miss only with the documented `NENALEZENO` JSON error
and a nonblank textual description. Other HTTP failures remain errors. [Tests](tests.rs) cover
matched identity, original address preservation and the HTTP failure boundary.

## Structured delivery numbers

When ARES omits `cisloDoAdresy`, use explicit `cisloDomovni` plus optional
`cisloOrientacni` and its letter. Descriptive number type 1 uses `č.p.` when no
street is supplied; evidential type 2 uses `č.ev.`. Unknown types remain native.
See [ČÚZK number types](https://isui.cuzk.gov.cz/help/topics/postupy003.htm).
Source full-address text is never reconstructed or rewritten.

The ignored [probe](probe.rs) exercises the shared facade with bounded manifests,
retains raw bodies and hashes, and stops on failures. Offline replay requires
source-hash-bound expectations; prior outputs are observations, not the oracle.

When no street is supplied but the named settlement part differs from the municipality,
the delivery line uses that explicit settlement part plus house number. This preserves
the source delivery locality without replacing the common city (municipality).

Transport mechanics use the private [registry transport helpers](../registry_transport/mod.rs).
Each client instance owns its admission state, shared only with its clones;
country identifiers, accepted HTTP statuses, decoding and mapping stay here.
