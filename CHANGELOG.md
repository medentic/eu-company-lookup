# Changelog

## 4.0.0

- Replace public error enums with one serializable `LookupError` object: stable
  IDs, English titles and administrator descriptions, availability flag and
  structured details. See the [error contract and migration](docs/reference/errors.md).
- Add opt-in raw HTTP diagnostics with request/response header arrays, lossless
  bodies, cURL reproduction and explicit incomplete-capture reasons.
- Add keyless national company lookup for Ireland (CRO open data) and Norway
  (Enhetsregisteret). Native identifiers, typed provider details and explicit
  capability discovery remain separate from VIES VAT verification.
- Share bounded registry transport and CKAN wire handling while preserving
  provider-specific identity checks, absence rules, address roles and VAT scope.
- Extend conservative address parsing and retain original provider address text.
- Document VIES national-service availability and capacity failures separately
  from matched negative VAT responses.
- Add bounded, resumable multi-country verification tooling; collected identifiers
  and source observations remain excluded from the public source and package.

## 3.0.0

- Automatically enrich nine-digit BG company lookups through a required VIES
  dependency owned by the Bulgarian adapter.
  Require valid, name-matched responses for common VAT fields and retain full
  `vatVerification` in BG details. Skip branches; propagate VIES failures. Native
  BG registry clients remain registry-only; disabling standalone VIES operations
  does not disable verification inside shared BG company lookup.

- Default missing BG address countries to `България` / `BG` in Commercial
  Register and BULSTAT results; normalize supplied party country names in details.

- Default missing ANAF address countries to `Romania` / `RO`; preserve explicit
  source countries and keep absent address objects null.

- Move native-to-common projections into the owning registry adapters and derive
  source labels from typed details to prevent mismatched provider tags. Document
  the complete common object and the implementation contract for future adapters.

- Add common nullable `vatRegistered` and `vatId` company fields. RO maps ANAF's
  standard domestic registration flag and derives a VAT ID only for a positive
  matched fiscal registration; preserve the requested `taxId` spelling. BG uses the supplementary VIES enrichment described above. No generic
  cross-country prefix inference is introduced.

### Breaking changes

- Move the shared result's `nativeRegistryStatus` into `registryDetails.data`.
  Keep only normalized lifecycle `status` at the top level and document the
  common-model versus provider-specific extension boundary. `CompanyDetails`
  variants now have named `native_registry_status` and `details` fields.

- Normalize country names to title case across structured addresses and expose
  uppercase `countryISO2` on native ANAF addresses as well as common addresses.
  Preserve original full address text and keep unrecognized country codes null.

- Replace the shared company result's nested `identifier` with top-level `country`
  and string `taxId` (`tax_id` in Rust), preserving leading zeroes.

- Unify company fields with nullable top-level `registryUrl` and `foreignName`.
  Rename shared `details` to `registryDetails` (`registry_details` in Rust), with
  direct Commercial Register, BULSTAT and ANAF variants for provider-only facts.

- Consolidate company lookup on `Client::lookup_company(value, country)`, returning
  `CompanyLookupResult`. Remove country-specific facade methods; explicit Romanian
  fiscal dates remain available through `RoRegistryClient::lookup_at`.

## 2.0.1

- Extend synthetic VIES identity and validity tests across every supported prefix,
  and replay saved SOAP outcomes and reviewed country address corpora offline.
- Add conservative Croatian, Hungarian and Maltese address formats; refine
  Estonian locality labels, Finnish corporate postal labels, Swedish postal-only
  addresses and Portuguese delivery lines without postcodes. Preserve original
  source text and leave uncertain components unknown.
- Document Maltese postcode discrepancies and incomplete Portuguese source
  addresses as known limitations, separate from VAT validity.
- Extend bounded VIES collection to all supported prefixes with immutable six-
  and twenty-five-second test profiles. Production timeout and retry behavior
  remain unchanged.

## 2.0.0

### Breaking changes

- Replace `CompanyStatus::Other(String)` with `Unknown`. Status serializes as
  `active`, `closed`, `unknown`, or null when absent. Unknown Commercial Register
  codes now produce `unknown`; BULSTAT `развиващ дейност` produces `active`.
  Native values remain available separately.
- Make capability VAT prefixes optional and replace the `LanguageSupport` enum
  with explicit available/default/selectable metadata. Metadata-only EEA entries
  have no enabled providers and do not imply lookup support.

- Replace flat provider operation lists and the provider-level identifier scheme
  with typed per-operation input, execution and date contracts. Use
  `ProviderCapabilities::supports` to check operation/scheme pairings.

See [migration guidance](docs/reference/api-stability.md#migrating-from-1x).

### Added

- Provider-independent `Client` facade and named `ClientBuilder` configuration
  with explicit adapter enablement, retaining the convenience constructors.
- Free `capabilities()` discovery without transport construction; client discovery
  reports only enabled adapters. Typed errors distinguish disabled operations and
  unsupported country/scheme pairs from malformed input and provider failures.

- Shared `BusinessId` validates country, identifier scheme and value together.
  `Client::lookup_company` returns `CompanyLookupResult` for BG/RO, with separate
  registered/fiscal addresses and typed provider details.
- Romanian ANAF taxpayer lookup, dated domestic tax flags, bounded transport,
  explicit source-specific misses and conservative Romanian VIES address parsing.
  Tax flags remain distinct from VIES validity and company lifecycle.
- `nativeRegistryStatus` preserves source status text or numeric codes as strings
  for all company results. Romanian lifecycle stays unknown until source mappings
  are verified; fiscal inactivity never implies closure.
- Preferred native result names `VatLookupResult` and `BgCompanyLookupResult`.
  `VatCheck` and `CompanyRecord` remain aliases without deprecation warnings.

### Fixed

- Preserve Bulgarian locality-only addresses, remove empty delivery placeholders,
  separate explicit ground-floor/office details and normalize duplicate portal
  street labels without discarding original provider address text.
- Extract corporate holder names with explicit business-code metadata and map
  Bulgarian partnership legal form `СД`.
- Remove artificial default spacing between Bulgarian registry requests while
  retaining configurable pacing, shared admission and failure cooldowns.

## 1.0.0

Initial release, scoped to VIES and Bulgarian registry lookup:

- Asynchronous VAT verification for the EU VIES prefixes and Northern Ireland's XI,
  with typed syntax validation and explicit EL/GR and XI/GB-NIR mappings.
- Bulgarian business lookup through the Commercial Register, followed by BULSTAT
  only after a definitive miss, with exact nine- or thirteen-digit identity checks.
- Typed company details, official record links, source-specific status, legal forms,
  role holders, exact capital strings and constitutive-document references.
- Source-preserving address envelopes and shared conservative address utilities.
- Explicit errors separating invalid input, negative observations, rate limits,
  availability failures and unexpected provider responses.
- Bounded deadlines and response bodies, shared admission limits and cooldowns;
  no automatic retries, redirects or caching.
- Local capability discovery, extensible public models and validated identifier
  conversions. Serde serialization preserves the documented JSON contracts.

Other national registries, checksums, D-U-N-S and verification reports are outside
this release. See [README.md](README.md) for usage and limitations.
