# French public company search

Use the anonymous DINUM [API Recherche Entreprises](https://recherche-entreprises.api.gouv.fr/docs/)
with `q=<SIREN>&minimal=true&include=siege`. No key, account or cookies are required.
The source aggregates public company data; it excludes non-diffusible entities and
rejected RCS registrations and is not a complete Sirene export or certified extract.
The minimal response excludes unnecessary officer details. Source documentation and
its [OpenAPI schema](https://recherche-entreprises.api.gouv.fr/openapi.json) define the query.

## Identity and fields

Accept nine ASCII SIREN digits, preserving zeroes and removing ASCII whitespace.
SIRET establishment numbers and VAT numbers are separate and rejected. No checksum
or registration claim follows from syntax. Require one result, an exact SIREN,
and a nonblank source `nom_complet`. A supplied headquarters must have `est_siege`
true; any supplied SIRET must be fourteen digits with the matched SIREN prefix.
Empty or ambiguous searches remain errors because public coverage is incomplete.

The common name is the original display name. Headquarters text `adresse` is
preserved verbatim; delivery components are joined in source order, and postcode
and locality retain source spelling. An explicit foreign-country code selects
foreign locality. Foreign country labels use shared display normalization; INSEE
country codes are not ISO codes. No domestic country, region label, geographical
address substitute or missing full text is invented. Absent headquarters stay null.
Native headquarters codes, CEDEX, source components and dates remain typed details.

Legal-form codes retain their native values and map through the INSEE level-III
September 2022 code list to French labels. Unlisted codes remain null. Company
administrative status A maps to active and C to closed; establishment status
is not substituted. VAT registration, VAT ID, fiscal address, foreign name
and registry link remain null. Selected native identity, name, legal-form,
activity, diffusion and source timestamp fields are retained without translation.
No historical-date or language-selectable operation is implemented.

## Transport and evidence

The private shared transport supplies a thirty-second deadline, three-second
connect timeout, two-second request spacing, sixty-second cooldown and one-MiB
body cap. Provider rate guidance and its overload exception are described below.
Retry-After is retained. No automatic retries, redirects or ambient proxies;
errors never prove absence. Offline synthetic tests cover exact query, identity,
missing fields and source preservation. Live evidence stays local-only and does
not imply a release or exhaustive provider coverage.

See the [mapping audit](../../docs/development/company-mapping-audit.md) for source definitions and remaining null fields.

After successful decoding, public lookup requires resolved lifecycle and returns
`unresolved_data` for missing, unsupported or ambiguous lifecycle evidence.
Missing required envelope/identity fields remain decoding errors. Pure decoding/projection
retains the original observation. See the
[runtime mapping audit](../../docs/development/company-mapping-audit.md#runtime-enforcement).

## Access, pacing and recovery

One search request uses the default two-second start interval. The
[official service catalog](https://www.data.gouv.fr/dataservices/api-recherche-dentreprises)
publishes seven calls per second and explicitly allows a lower limit under load.
This is not a guaranteed throughput entitlement; account for other callers sharing
the service and respect 429/`Retry-After`. Public-data exclusions remain relevant
even when transport succeeds. These limits belong to Recherche Entreprises,
not to VIES or every French registry service.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.
