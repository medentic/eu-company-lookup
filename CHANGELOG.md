# Changelog

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
