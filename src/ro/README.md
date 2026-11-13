# Romanian ANAF adapter

This is a taxpayer/fiscal adapter, not an ONRC extract. The
[consumer reference](../../docs/reference/romanian-registry.md) defines dates and
limitations. [client.rs](client.rs) owns HTTP policy and the configured
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
| `.stare_inregistrare` | Preserved registration/native status; `Unknown` when present, null when absent. |
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

[model.rs](model.rs) owns the native result; [../model/business.rs](../model/business.rs)
owns conversion into the common company view. Test both [tests.rs](tests.rs) and
[consumer contracts](../../tests/romanian.rs) when adjusting decoder behavior.
[probe.rs](probe.rs) and the collector are test-only/local evidence workflows.
