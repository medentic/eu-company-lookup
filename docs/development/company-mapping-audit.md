# Company field mapping audit

This audit covers the common fields of all sixteen national company adapters.
Conversions use provider-specific, deterministic rules; no runtime language model,
translation service or VAT-prefix guess is involved. LV and SK use documented identity-matched supplemental registry queries.
Native observations remain in typed details. The linked source definitions explain
why a fact maps, or why a remaining null/unknown is intentional.

## Lifecycle

`active` describes an active source company/record, not trading, solvency or VAT
registration. `closed` includes explicit cessation, deletion or deregistration.
Liquidation, bankruptcy and pending procedures have distinct resolved states when source definitions support them; they do not automatically mean closed.
Exact text matching normalizes case and surrounding whitespace on a working copy;
the original text is preserved. Novel or ambiguous supplied statuses stay unknown in pure projections and fail public lookup with `unresolved_data`.
Absent status metadata stays null unless documented source membership or cessation evidence resolves lifecycle. Missing cessation dates never imply active.

| Adapter | Supported mapping | Remaining limits and source |
| --- | --- | --- |
| BG Commercial Register | Existing deed states 1/2 active; state 4, closed main subdeed or explicit company deletion closed. | Unrecognized deed codes remain unknown. See [BG reference](../reference/bulgarian-registry.md). |
| BG BULSTAT | `развиващ дейност` active; exact `заличен` closed. | Other descriptions remain unknown; no substring match against unrelated deleted fields. See [BULSTAT](https://reports.bulstat.bg/bulstat-ireports/). |
| RO | Exact `INREGISTRAT`/`RELUARE ACTIVITATE` active; `SUSPENDARE ACTIVITATE` inactive; `RADIERE`/`RADIAT` and `DIZOLVARE FARA LICHIDARE(FUZIUNE)` closed, optionally followed by `din data` and a strict day.month.year date. | Other phrases remain unknown. [ANAF v9](https://static.anaf.ro/static/10/Anaf/Informatii_R/Servicii_web/doc_WS_V9.txt) separates company status from fiscal inactivity; tax flags never determine lifecycle. |
| AT | `ACTIVE` active; `DELETED` closed. | `HISTORIC` and other values unknown. These are the [official portal](https://justizonline.gv.at/jop/web/firmenbuchabfrage) status enum values; current lookup searches active records only. |
| BE | Legal-situation code `000` active; `014`–`017`, `021`–`026`, `111` closed. English `Normal situation` maps to active when no code exists. | Codes 012/013 map to liquidation, 048/049/050 to bankruptcy and 040/041/091 to restructuring. Duplicate-number replacement and unresolved historic situations remain unknown. Codes take precedence over labels. [FPS Economy definitions](https://economie.fgov.be/sites/default/files/Files/Entreprises/BCE/BCE-Definitions-des-situations-juridiques.pdf) distinguish liquidation opening from completion. |
| CZ | Valid `datumZaniku` maps to closed. Primary ROS `AKTIVNI` with absent insolvency and execution registrations maps to active; primary ROS `ZANIKLY` maps to closed. Non-primary ROS cessation triggers an exact, unique ROS supplement; a confirmed `ZANIKLY` record with a valid cessation date maps to closed. | Other unsupported or conflicting registrations remain unresolved. The official ARES `StavZdroje` codebook defines these states; generic availability flags are not substitutes. [ARES schema](https://ares.gov.cz/ekonomicke-subjekty-v-be/rest/v3/api-docs). |
| FI | Trade Register `1` registered maps to active; `4` ceased maps to closed. | `2` removed does not establish dissolution; unregistered/pending states remain unknown. Ongoing SANE/SELTILA/KONK situations map to restructuring/liquidation/bankruptcy for registered records. Conflicting simultaneous situations or dissolution evidence prevent active success. [Register codes](https://avoindata.prh.fi/opendata-ytj-api/v3/description?code=REK_KDI&lang=en), [schema](https://avoindata.prh.fi/opendata-ytj-api/v3/schema?lang=en). |
| FR | Company administrative `A` active; `C` ceased maps to closed. | Headquarters status is never substituted. [INSEE definitions](https://www.insee.fr/fr/statistiques/fichier/3711695/Description-liste-sirene-fr.pdf). |
| EE | `R` registered active; `K` deleted closed. | `L` maps to liquidation and `N` to bankruptcy. [RIK statuses](https://avaandmed.ariregister.rik.ee/en/open-data-api/detailed-company-data-query). |
| NO | Explicit bankruptcy/liquidation flags map to the corresponding proceeding; a valid deletion date maps to closed. | Simultaneous conflicting flags/deletion evidence remain unknown; active requires the official `Enhet` response class, three explicit false proceeding flags, valid registration date and an explicitly empty annotations list. `SlettetEnhet` requires a valid deletion date. Restructuring dates map to restructuring; foreign insolvency stays unresolved. [Brreg API](https://data.brreg.no/enhetsregisteret/api/dokumentasjon/en/index.html). |
| PL | P is preferred; P HTTP 404 permits S fallback with exact register matching. A single ongoing sixth-section liquidation, bankruptcy or restructuring maps to its corresponding state. An unambiguous suspension interval covering the extract snapshot maps to inactive. A matched full extract with terminal final removal maps to closed; merger role and receiver KRS numbers remain separate from lifecycle. | Complete current registration with ordered dates and no unresolved restrictions maps to active; dated discontinued bankruptcy is historical. Non-final removal also maps to closed under the consumer policy, with non-finality retained. Dated affirmative continued-existence resolutions can reconcile old dissolution entries; a duplicated appointment order with current liquidation name and liquidator evidence remains liquidation. Other conflicts, unsupported position codes and foreign-branch proceedings remain unresolved. [Adapter scope](../../src/pl/README.md). |
| SK | Valid `datumZrusenia` maps to closed; matched current RPO membership and CL010108 states resolve active or proceedings. | Missing date does not imply active; malformed date is unresolved. API-record deletion is not entity dissolution. [RÚZ API](https://www.registeruz.sk/cruz-public/home/api). |
| LV | Valid `terminated` maps to closed; complete supplemental queries resolve active, liquidation or suspended activity. | `closed` markers lack a complete published codebook; a nonblank marker alone stays unknown. A blank marker is absent status; its exact text remains in native `closed`. Missing removal dates never imply active. [Dataset definition](https://data.gov.lv/dati/lv/dataset/uz) includes removal through reorganisation. |
| LT | Valid `isreg_data` deregistration date maps to closed. | The [official status reference list](https://data.gov.lt/datasets/1624/versions/1213/api/getall/Statusas/) resolves normal/no-proceedings, removal, liquidation, bankruptcy, restructuring and initiated liquidation. Transformation states and unrecognized references remain unresolved; missing dates alone do not imply active. [JAR schema](https://data.gov.lt/datasets/1484/versions/63/models/JuridinisAsmuo/). |
| IE | Normal/reinstated states active; dissolved, struck-off, ceased and reviewed terminal deletion/deregistration states closed. Official numeric codes also work without labels. | CRO proceedings map to explicit liquidation, receivership, restructuring, pendingClosure, liquidationStayed, administration and inactive states. Interim, conversion, unknown codes and conflicting code/label pairs remain unresolved. [CRO status list](https://services.cro.ie/companystatuses.aspx/1000). A past dissolution date does not override a restored current status. |
| IS | Exact dated company-deregistration notice in the registration subtitle maps to closed. | Registration date alone and VAT-table removal do not establish lifecycle; full API access requires registration. [Adapter scope](../../src/is/README.md). |
| GR | Exact `ΕΝΕΡΓΗ` active and `ΔΙΑΓΡΑΦΗ` closed, including accented spelling. | Exact dissolution/liquidation and bankruptcy labels map to liquidation/bankruptcy; other states remain unresolved. No substring inference. Code-only observations remain unknown. [GEMI guidance](https://gge.mindev.gov.gr/tomeas-emporiou/sixnes-erotiseis-gemh/). |

BE, GR and IE preserve a supplied status code as `nativeRegistryStatus` when no
label is supplied. CZ and date-only LT observations keep that field null even when a cessation date maps
to closed: a date is not rewritten into an invented status label.

## Legal forms

Source labels already populate BG, RO, AT, BE, FI, NO, PL, LV, IE, IS and GR.
Missing labels remain null; a company-name suffix is not a substitute.

- CZ: `PravniForma` from the ARES `res` code list maps 129 codes with one Czech
  label across source validity periods. Unspecified `000`, unknown codes and
  five codes with conflicting historical labels remain null. `pravniFormaRos`
  uses a different code list and is not silently interpreted as RES.
- FR: the [INSEE September 2022 level-III list](https://www.insee.fr/fr/information/2028129)
  supplies 260 French labels. Preserve all four code digits, including `0000`.
- SK: the [RÚZ/ŠÚSR 0056 list](https://www.registeruz.sk/cruz-public/api/pravne-formy)
  supplies 114 Slovak short labels. Unspecified `000`/`995` remain null.
- EE: [RIK's documented examples](https://avaandmed.ariregister.rik.ee/et/ariregistri-rest-api/ettevotte-uldandmed)
  establish `3` as Füüsilisest isikust ettevõtja and `5` as Osaühing. Other portal
  codes remain null until their matching classification is verified.
- LT: `forma` is an opaque referenced record, not a legal-form code or label.
  Resolving it requires a separate source operation; it remains native.

The static code lists were reviewed on 2026-11-18. They add no runtime requests.
To update them, review the linked authority's list, retain native codes, preserve
historical ambiguity, and add regression cases for changed meanings. Do not derive
labels from the company name or fill an unknown code with a likely translation.

## Addresses and remaining optional fields

| Adapter | Address decisions | Other remaining nulls |
| --- | --- | --- |
| BG | Existing registered-office mapping and documented jurisdiction default retained. VIES never replaces it. | Fiscal address absent; VAT enrichment retains its existing name-match rules. |
| RO | Separate registered/fiscal roles and original fiscal text retained. | No separate foreign name or record permalink supplied. |
| AT | Street/city/postcode map to office; no invented full text or country. | No VAT registration, foreign name, fiscal address or verified permalink. |
| BE | Source office components map; box stays separate. | No original full-address text, VAT evidence, foreign name, fiscal address or verified permalink. |
| CZ | Existing `sidlo` components/raw text map to office. Delivery address is not fiscal. | DIČ and VAT-group codes do not prove the subject's VAT registration; no foreign name or verified permalink. |
| FI | Schema defines street/postal contact addresses, not legal office/fiscal roles. Both remain native. | Parallel names may be multiple and are not necessarily foreign-script names. Register 6 has VAT-liability, notification-only and group-member types; no blanket true/false or VAT-ID derivation. No verified permalink. |
| FR | Matched `siege` maps to office. Municipality/region codes remain codes; explicit country labels only. | No VAT evidence, fiscal address, foreign name or verified permalink. |
| EE | Original legal address/postcode map; no guessed country/city split. | Supplied HTTPS record URL is exposed only after exact official host/path/identity validation. VAT, fiscal address and foreign name absent. |
| NO | Business and postal roles remain native; neither is substituted for a verified legal office/fiscal role. | Domestic VAT flag already maps. VIES-specific `VatId` cannot represent Norwegian VAT IDs. No foreign name or verified permalink. |
| PL | Source seat/address maps to office; foreign parent office never replaces branch office. | NIP/REGON do not prove VAT registration. No fiscal address, foreign name or verified permalink. |
| SK | Accounting address remains native; `sidlo` is a municipality code, not full address text. | DIČ does not prove VAT registration. No foreign name, fiscal role or verified permalink. |
| LV | Original legal address maps; numeric geographic/index fields do not supply verified display names or postal formatting. | SEPA identifier is not VAT. No fiscal address, foreign name or verified permalink. |
| LT | `pilnas_adresas` explicitly means registration address; map original text to office, leaving unknown components null. | Opaque address references stay native. No VAT, fiscal address, foreign name or verified permalink. |
| IE | Company lines map to the statutory registered office, with Eircode as postcode. This follows [CRO office guidance](https://cro.ie/post-registration/company-post/duties-of-directors-and-secretaries/); no city/county or original full text is synthesized. | No VAT evidence, fiscal address, foreign name or verified permalink. |
| IS | `Lögheimili` maps to office; postal address stays native. | No VAT, fiscal address, foreign name or verified permalink supplied by selected fields. |
| GR | Official full address maps; map-display address is not substituted. | AFM is not VAT evidence. Registered Latin name and matched GEMI permalink already map. No fiscal address. |

These are boundaries of the selected source projections, not claims that another
registry service could never supply the missing facts. VIES itself remains a
VAT-validation observation: a missing name/address and a provider failure cannot
be converted into a guessed company fact or a negative registration conclusion.

## Validation qualification

An unresolved lifecycle is not an acceptable positive or negative company
validation case. Keep the source fact and parser result, flag `unresolved`, and
exclude it from qualifying validation coverage. Do not invent active/closed or
not-found to remove unknown/null. See [validation outcomes](testing.md#registry-validation-outcomes)
for replay/report rules. This distinguishes nullable observation fields from a
claim that registry validation has completed successfully.

## Runtime enforcement

All sixteen native registry clients enforce resolved lifecycle after identity
decoding and provider-local projection, before returning success. Missing or
unmapped/ambiguous lifecycle returns `unresolved_data`, preserving the source
status in error details. Optional non-lifecycle facts remain nullable.
Pure projections and offline parser replay retain unresolved observations.

Additional proceeding definitions are verified against
[CRO statuses](https://services.cro.ie/companystatuses.aspx/1000),
[RIK status codes](https://avaandmed.ariregister.rik.ee/et/ariregistri-rest-x-tee/ettevote-ja-seosed),
[PRH situation schema](https://avoindata.prh.fi/opendata-ytj-api/v3/schema?lang=en),
[FPS Economy legal situations](https://economie.fgov.be/sites/default/files/Files/Entreprises/BCE/BCE-Definitions-des-situations-juridiques.pdf)
and [Brreg field definitions](https://data.brreg.no/enhetsregisteret/api/dokumentasjon/en/index.html).
No source guarantees that future codes are exhausted; unsupported evidence must
remain an explicit error until its meaning is verified.

## Supplemental lifecycle sources

Latvia queries [current liquidation](https://data.gov.lv/dati/dataset/liquidations), [suspension](https://data.gov.lv/dati/dataset/suspensions-prohibitions) and insolvency datasets alongside the base registry record. Complete exact-identity results distinguish active, liquidation and suspended activity. Ongoing insolvency with an unsupported process type remains unresolved.

Slovakia supplements matched RÚZ records lacking dissolution dates with [RPO](https://rpo.minv.sk/rpo-api-doc.html). The exact identifier search requests `onlyActive=true`; current detail must match both identifiers. Current CL010108 proceedings override ordinary active membership. Rate limits and unavailable supplemental sources remain operational errors.

Romanian `TRANSFER(SOSIRE)` describes tax-file arrival, not company lifecycle, under [ANAF transfer procedures](https://static.anaf.ro/static/10/Anaf/legislatie/OPANAF_2853_2017.pdf). It remains unresolved without independent lifecycle evidence. Reviewed corpus revisions preserve the original expectations and source hashes in local append-only evidence.
