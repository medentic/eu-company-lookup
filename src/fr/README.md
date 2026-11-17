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

Legal-form codes remain native; common legal form is null until label mapping is
supported. Administrative status remains native and maps to unknown lifecycle,
not inferred active/closed. VAT registration, VAT ID, fiscal address, foreign name
and registry link remain null. Selected native identity, name, legal-form,
activity, diffusion and source timestamp fields are retained without translation.
No historical-date or language-selectable operation is implemented.

## Transport and evidence

The private shared transport supplies a thirty-second deadline, three-second
connect timeout, two-second request spacing, sixty-second cooldown and one-MiB
body cap. This is below the documented seven requests per second per IP limit.
Retry-After is retained. No automatic retries, redirects or ambient proxies;
errors never prove absence. Offline synthetic tests cover exact query, identity,
missing fields and source preservation. Live evidence stays local-only and does
not imply a release or exhaustive provider coverage.
