# Finnish PRH YTJ open data

This adapter targets the official [PRH YTJ v3 API](https://avoindata.prh.fi/en/ytj/swagger-ui)
using an anonymous exact `businessId` query. PRH publishes Trade Register company
information and tax-register entries as [open data](https://www.prh.fi/en/companiesandorganisations/tietopalvelut/prhopendata.html),
updated daily. No keys, accounts, cookies or full-register download are required.
Attribute the original data to the Finnish Patent and Registration Office (PRH).
Implementation and live verification are separate; no release is claimed here.

## Identity and source boundary

Accept seven ASCII digits, a hyphen and one check digit, removing ASCII whitespace
only. Keep leading zeroes and the hyphen. Syntax does not verify the checksum or
registration; FI-prefixed VAT numbers are a separate scheme. Require exactly one
returned company with the exact requested business ID. Zero or multiple results,
wrong identity and malformed envelopes remain errors; this adapter makes no
broader absence claim about businesses outside PRH's open-data coverage.

Select one nonblank company name of type `1`, version `1`, using the official
[name-type list](https://avoindata.prh.fi/opendata-ytj-api/v3/description?code=TLAJI&lang=en).
Version 1 is the latest name even for dissolved companies; an end date must not
cause a false miss. Parallel and auxiliary names stay in typed details. Select
only a unique version-1 legal form's Finnish (`languageCode: "1"`) source label;
missing labels stay null. No caller-selected language or historical-date lookup.

## Common fields and native evidence

The common result supplies FI, the exact business ID, source company name and
available Finnish legal-form label. PRH's Trade Register status remains the native
status code. PRH's [REK_KDI classification](https://avoindata.prh.fi/opendata-ytj-api/v3/description?code=REK_KDI&lang=en)
distinguishes registered, removed and ceased entries. Trade Register code 1 maps to active and 4 to closed; code 2 (removed)
does not itself prove dissolution. Ongoing situations or a dissolution date
prevent a registered code from asserting active.
VAT registration, VAT ID, fiscal address, foreign name and registry link remain null; tax-entry types include special notification and group regimes; no blanket VAT inference is made.

Street and postal contact-address roles do not establish the shared legal
registered-office role. Both remain fully available in typed native addresses;
the common registered office stays null until a supported role mapping is
established. Never substitute the postal address or reconstruct original full text.
Native details preserve source identifiers, names and versions, legal forms,
business line, website, situations, register entries, address components,
language codes, status codes and dates. Source strings are not translated.
`lastModified` is optional: actual official records omit it despite the schema's
required annotation. This is not an incorporation date or freshness guarantee.

## Transport and verification

Use the private [shared transport helpers](../registry_transport/mod.rs), with
independent clone-shared admission, thirty-second total deadline, three-second
connection timeout, two-second start spacing, sixty-second failure cooldown and
one-MiB body limit. No automatic retries, redirects or ambient proxies. HTTP and
schema failures never become negative registration results. Tests use synthetic
records. Collected identifiers, observations and reviewed expectations stay local.

See the [mapping audit](../../docs/development/company-mapping-audit.md) for source definitions and remaining null fields.

After successful decoding, public lookup requires resolved lifecycle and returns
`unresolved_data` for missing, unsupported or ambiguous lifecycle evidence.
Missing required envelope/identity fields remain decoding errors. Pure decoding/projection
retains the original observation. See the
[runtime mapping audit](../../docs/development/company-mapping-audit.md#runtime-enforcement).

Ongoing SANE/SELTILA/KONK situations map to restructuring/liquidation/bankruptcy
when the record is registered and has no conflicting dissolution evidence. Ended
situations do not override current registration; conflicting situations are unresolved.

## Access, pacing and recovery

One PRH query uses the default two-second start interval. This is client policy,
not an established PRH quota. Daily publication and a latest-version name do not
mean the company is active: current register status, ongoing situations and
cessation evidence must be reviewed together. A source-schema discrepancy or
record outside open-data coverage is distinct from access failure or dissolution.

See [shared pacing and evidence rules](../README.md#registry-pacing-and-evidence)
for clone scope, cooldowns, error classification and complete-batch replay.
