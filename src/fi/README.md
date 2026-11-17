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
status code. Lifecycle, VAT registration, VAT ID, fiscal address, foreign name
and registry link remain null; register-entry codes are not VIES evidence.

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
