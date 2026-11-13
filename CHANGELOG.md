# Changelog


## 1.0.1 — 2026-11-13

- Extract Bulgarian settlement-first locality-only addresses when the repeated region names agree, without inventing a street.
- Omit empty primary-address and housing-complex placeholders, and asterisk-only delivery values even without comma separators. Preserve real numbers, ranges, negative floors and meaningful starred values.
- Place explicit terminal `/партерен етаж/` annotations in the secondary address line.
- Preserve original provider address text and existing public Rust/JSON contracts.

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
