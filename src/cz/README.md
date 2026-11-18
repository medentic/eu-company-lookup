# Czech ARES adapter work

[IČO syntax validation](identifiers.rs), the bounded [GET client](client.rs),
[typed source models](model.rs) and [identity decoder](decode.rs) are implemented.
Czech company lookup is enabled in the facade and capability catalog.
The [shared projection](mapping.rs) maps explicit cessation dates to closed and
unambiguous ARES legal-form codes to Czech labels. VAT facts remain null.
The reviewed sample meets the 200-distinct-ID national verification gate,
including active and closed companies, with source-bound expectations and
complete offline replay. This does not establish coverage of every proceeding
or entity type.

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
  Map `pravniForma` through the official ARES PravniForma/res list when one
  label applies across its validity periods. Retain ambiguous and ROS-only
  codes natively; do not use the RES list for ROS codes.
- `dic` is a tax identifier. Its presence alone does not establish VAT registration.
  `dicSkDph` describes a VAT group identifier and must not silently replace the
  subject's own VAT identity.
- `seznamRegistraci` describes source registrations. Do not interpret a source's
  `AKTIVNI` value as proof of company lifecycle or standard domestic VAT status
  without a documented source-specific rule. Primary ROS `AKTIVNI` plus explicit `NEEXISTUJICI` insolvency/execution registrations establishes active membership; primary ROS `ZANIKLY` establishes closure. Primary VR uses the same rules only when ROS explicitly reports `NEEXISTUJICI`: VR `AKTIVNI` requires absent IR/CEU records, and VR `ZANIKLY` establishes closure. Conflicting or missing ROS evidence does not permit this rule. The ARES `StavZdroje` codebook defines these values. For a non-ROS primary source with ROS `ZANIKLY` and no aggregate cessation date, query `/ekonomicke-subjekty-ros/{ico}`. Require an exact envelope and record IČO, exactly one record, `stavSubjektu: ZANIKLY` and a canonical cessation date. Preserve those facts separately in `rosClosure`; do not replace the aggregate date. Other cases remain unresolved.
- Preserve dates and source identifiers in typed details. Do not infer incorporation
  or closure merely from response freshness.
- A definitive miss requires HTTP 404 with the documented `NENALEZENO` envelope.
  Other HTTP failures do not establish absence.

Raw observations, identifier queues, source snapshots and review history remain
local-only. They are not synthetic fixtures or evidence of completed coverage.

## Native transport

Clones share one admission lock, a two-second completion gap and a sixty-second
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

See the [mapping audit](../../docs/development/company-mapping-audit.md) for source definitions and remaining null fields.

Offline replay also accepts a source-hash-bound `outcome: "notFound"` review,
requiring captured HTTP 404 and the documented `NENALEZENO` envelope. This is a
verified negative registry case; null/unknown lifecycle is not one.

After successful decoding, public lookup requires resolved lifecycle and returns
`unresolved_data` for missing, unsupported or ambiguous lifecycle evidence.
Missing required envelope/identity fields remain decoding errors. Pure decoding/projection
retains the original observation. See the
[runtime mapping audit](../../docs/development/company-mapping-audit.md#runtime-enforcement).

## Access, pacing and recovery

ARES uses a two-second gap after completion and between aggregate and supplemental
ROS exchanges; this is client policy, not a verified provider quota. Both exchanges
share the lookup deadline and admission lock. A supplemental failure remains an
operational error; an empty or ambiguous ROS result remains unresolved. Only the
documented HTTP 404 `NENALEZENO` envelope qualifies as an ARES negative result;
access denial, malformed JSON and other failures do not.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.
