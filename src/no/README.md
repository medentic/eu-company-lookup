# Norwegian Enhetsregisteret open data

Use Brønnøysundregistrene's anonymous entity endpoint, with the explicit version-2
media type. The [official API documentation](https://data.brreg.no/enhetsregisteret/api/dokumentasjon/en/index.html)
distinguishes entities from subentities and credentialed personal-role services.
This adapter reads only entity data, with no keys or account. Attribute the data
to Brønnøysundregistrene under its Norwegian Licence for Open Government Data.

Accept nine ASCII organisation-number digits, removing ASCII whitespace and
preserving zeroes. Do not accept VAT prefixes, MVA suffixes or personal IDs.
Syntax is not a checksum or registration assertion. Require an exact source
organisation number and a nonblank name. HTTP 404 remains a provider error in
this adapter; no cross-register absence claim is exposed. Malformed, access and
service failures also remain errors. No historical-date or selectable-language
query is implemented; source labels remain Norwegian.

The common result supplies the original name and organisation-form description.
`registrertIMvaregisteret` supplies current domestic VAT registration when present;
it is not a VIES assertion. Norway has no VIES mapping. The common VAT ID remains
null: no identifier is synthesized and the shared VatId type is VIES-specific.
Business and postal addresses remain separate typed native observations. They
are not substituted for the shared legal registered-office or fiscal-address
roles. No full address is reconstructed from components. Foreign name and registry link stay null. Explicit bankruptcy/liquidation flags
map to their proceeding states, and valid deletion dates to closed. Conflicting
evidence is unresolved; false flags alone never imply active; the complete entity evidence described below is required.
Typed details retain identifier, name, organisation form, address roles, selected
classification and registration codes, dates, VAT and insolvency flags.

Use the private shared transport with thirty-second total deadline, three-second
connect timeout, two-second spacing, sixty-second cooldown and one-MiB body cap.
No automatic retries, redirects, cookies or ambient proxies. Synthetic tests cover
identity, missing facts, domestic VAT flags and source preservation; live records
and source-bound reviewed expectations remain local-only.

After successful decoding, public lookup requires resolved lifecycle and returns
`unresolved_data` for missing, unsupported or ambiguous lifecycle evidence.
Missing required envelope/identity fields remain decoding errors. Pure decoding/projection
retains the original observation. See the
[runtime mapping audit](../../docs/development/company-mapping-audit.md#runtime-enforcement).

The official `Enhet` response discriminator plus a valid registration date,
three explicitly false proceeding flags and an explicitly empty annotations list
establish active status. `SlettetEnhet` requires a valid deletion date. A valid
restructuring date maps to restructuring; foreign insolvency or unclassified
annotations remain unresolved. Native response class and annotations are retained.

## Access, pacing and recovery

One entity request uses the default two-second start interval; no numerical
provider quota has been established here. The anonymous entity endpoint is
separate from credentialed role/person services. Active mapping requires the
complete documented `Enhet` evidence, not merely three false proceeding flags.
Unclassified annotations or foreign insolvency are data gaps; HTTP 404 or access
failure remains a transport/source error under this adapter's current contract.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.
