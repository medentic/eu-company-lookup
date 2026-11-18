# Romanian ANAF adapter

This is a taxpayer/fiscal adapter, not an ONRC extract. Default fiscal dates use
Europe/Bucharest with Jiff and bundled IANA timezone rules, independently of the
host clock timezone. Explicit dates use the exported `Date` type. The
[consumer reference](../../docs/reference/romanian-registry.md) defines dates and
limitations. [client.rs](client.rs) uses the private shared registry transport
for bounds, admission, cooldowns and HTTP/body handling, and owns the configured
`webservicesp.anaf.ro/api/PlatitorTvaRest/v9/tva` endpoint;
[identifiers.rs](identifiers.rs) accepts two to ten ASCII CUI digits, preserving
leading zeroes. No checksum or registration claim follows from syntax.

## Decoder map

[decode.rs](decode.rs) requires one matched `found` record and no `notFound`
records, or exactly one matching `notFound` value and no found records. Other
shapes are errors. Present envelope status must be `cod: 200` and
`message: SUCCESS`; omitted metadata is accepted, explicit null is not.
ANAF transports CUI numerically, so identity compares decimal value while the
result retains the caller's normalized spelling. Found records must also match
the requested fiscal date. A matched miss becomes `RegistryNotFound` for ANAF.

| Source | Native result |
| --- | --- |
| `date_generale.cui`, `.data` | Identity/date checks; `taxId` and `requestDate`. |
| `.denumire`, `.forma_juridica`, `.cod_CAEN` | Company name, legal form, activity code. |
| `.nrRegCom`, `.data_inregistrare` | Source trade-register reference and registration-date text. |
| `.stare_inregistrare` | Exact registration/resumption maps to active, suspension to inactive, struck-off or dissolution without liquidation by merger to closed; transfer-arrival and other unsupported text remain unknown. |
| `.adresa` | Original full fiscal address. |
| `adresa_sediu_social` / `adresa_domiciliu_fiscal` | Separate registered-office / fiscal structured addresses. |
| `inregistrare_scop_Tva.scpTVA` | Domestic VAT registration. |
| `inregistrare_RTVAI.statusTvaIncasare` | VAT on collection. |
| `stare_inactiv.statusInactivi` | Fiscal inactivity; never lifecycle closure. |
| `inregistrare_SplitTVA.statusSplitTVA` | Split VAT. |
| `date_generale.statusRO_e_Factura` | E-invoice registration. |

Address fields use `s`/`d` prefixes followed by `denumire_Strada`, `numar_Strada`,
`denumire_Localitate`, `denumire_Judet`, `tara`, `cod_Postal`, `detalii_Adresa`.
Missing optional text/flags stay null; unexpected types fail decoding. Observation
time is UTC decoding time; the fiscal request date does not establish historical
identity/address coverage. No tax flag substitutes for VIES validity.

[model.rs](model.rs) owns the native result; [mapping.rs](mapping.rs)
owns conversion into the common company view. Test both [tests.rs](tests.rs) and
[consumer contracts](../../tests/romanian.rs) when adjusting decoder behavior.
[probe.rs](probe.rs) and the collector are test-only/local evidence workflows.

Address country names use shared title-case display normalization, preserving
language and diacritics. Missing, null or blank countries default to `Romania` /
`RO` for existing addresses, including a fiscal address built from raw text only.
Explicit source countries take precedence; unrecognized names retain a null ISO
code. Missing address objects remain absent. Original full address text is unchanged.

The shared VAT fields map ANAF's article-316 `scpTVA` observation. Only an explicit
true flag allows `registered_vat_id` to derive `RO` plus the matched numeric fiscal
ID, subject to VAT syntax validation. The original requested CUI spelling stays
in `taxId`. Missing/false flags yield no VAT ID; false does not exclude special
article-317 registration or a different identifier for another fiscal activity.
This conversion adds no VIES request. See the consumer reference's shared VAT fields.

Company status uses reviewed exact registration wording, independently of fiscal
inactivity. See the [mapping audit](../../docs/development/company-mapping-audit.md).

Offline replay supports source-bound positive and negative reviews and compares
all replay identities with the original manifest; see the
[validation contract](../../docs/development/testing.md#registry-validation-outcomes).

After successful decoding, public lookup requires resolved lifecycle and returns
`unresolved_data` for missing, unsupported or ambiguous lifecycle evidence.
Missing required envelope/identity fields remain decoding errors. Pure decoding/projection
retains the original observation. See the
[runtime mapping audit](../../docs/development/company-mapping-audit.md#runtime-enforcement).

Activity-status meanings follow [ONRC suspension/resumption guidance](https://www.onrc.ro/index.php/ro/mentiuni/persoane-juridice/suspendarea-sau-reluarea-activitatii).
Merger dissolution follows [Companies Act article 238](https://legislatie.just.ro/Public/DetaliiDocument/800).
Only exact phrases, optionally followed by `din data` and a strict calendar date,
map. Fiscal inactivity stays independent from company lifecycle.

## Access, pacing and recovery

The [ANAF v9 specification](https://static.anaf.ro/static/10/Anaf/Informatii_R/Servicii_web/doc_WS_V9.txt)
permits at most one request per second per client and up to 100 CUIs per request.
This adapter sends one CUI/date per request, defaults to two-second start spacing
and rejects intervals below one second. Defaults also include a 30-second total
deadline, 3-second connect timeout, 60-second failure cooldown and 1 MiB body cap.
The provider's batch allowance does not create a batch lookup operation here.

`TRANSFER(SOSIRE)` records tax-file arrival under
[ANAF transfer procedures](https://static.anaf.ro/static/10/Anaf/legislatie/OPANAF_2853_2017.pdf);
it does not resolve company lifecycle. This is a data gap, not an access failure
or a matched ANAF miss. When correcting retained expectations, preserve the prior
review, source hashes and revision rationale; never replace expectations with
current parser output merely to make replay pass.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.

### Cross-checking tax transfers

`TRANSFER(SOSIRE)` describes a fiscal transfer and does not by itself establish
legal lifecycle. ONRC publishes separate [company and status datasets](https://data.gov.ro/organization/onrc)
with a status codebook (`N_STARE_FIRMA`) and registration-number keyed status rows
(`OD_STARE_FIRMA`). These are dated bulk snapshots, not ANAF response fields or a
current per-CUI endpoint. A snapshot can independently verify a company's status
at its reference date; it must not silently rewrite a different-date ANAF response
or make every fiscal transfer active. The ANAF adapter does not download ONRC bulk
data during lookup.
