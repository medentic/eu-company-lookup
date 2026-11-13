# Public API compatibility

The 1.x contract covers exported Rust types and methods and documented JSON field
names, value types and null semantics. Provider data remains an observation that
can change between requests.

## Rust API

- Provider modules are private. The crate root explicitly exports identifiers,
  clients, configs, result models, capabilities and errors; `locations` is public.
- Result/capability structs, configuration structs and extensible enums are
  `#[non_exhaustive]`. Read fields, destructure with `..` and include a fallback
  match arm. Minor releases may add fields or variants.
- Construct configs using `Default` and then assign public fields. Client construction
  validates bounds. Endpoint overrides and parser internals are not public APIs.
- `VatId` and `BgBusinessId` keep their contents private. Parsing, `TryFrom` and
  Serde input validate syntax. `AsRef<str>` borrows the normalized identifier;
  conversion to `String` consumes it. Identifiers support Serde input and output;
  result models provide serialization, not general-purpose deserialization.
- Both clients are cloneable, `Send + Sync`; lookup futures are `Send`. Clones share
  admission state. Debug output exposes configuration, not requests or provider bodies.
- `RetryAfter` is exhaustive: its supported representations are seconds and an
  absolute date. Error and provider enums are extensible.
- Chrono types appearing in public fields are part of the Rust contract;
  incompatible dependency-type changes require compatibility review.

The minimum Rust version is 1.98.1. Increases require a documented minor release,
never a patch release. Network lookups require a Tokio runtime with I/O and time
enabled. CI covers Linux; no broader platform verification is claimed.

These choices follow [Cargo's SemVer guidance](https://doc.rust-lang.org/cargo/reference/semver.html).

## JSON and business meaning

Existing field names, types, null behavior and provider spellings are stable.
Minor releases can add fields and extensible enum values; consumers should tolerate
unknown fields and values. Missing optional information stays null, rather than
becoming an invented fact or an assertion of absence.

VAT results and company records are separate contracts. `valid: false` is a negative
VIES observation. `NotFound` is a definitive miss from both Bulgarian registers.
Operational failures cannot be converted into either result.

For Bulgarian records, `taxId` is a business code, `companyEnglishName` is the
registered foreign-script name, and `identifierEntryDate` is the UIC field entry
date. These names do not imply VAT registration, translation or incorporation.
`source` discriminates the untagged `registryDetails` payload.

## Address policy

Address parsing is conservative and independent of validity. Parser corrections
may change structured components in patch releases while retaining original
provider address text. VIES preserves decoded XML text; registry adapters preserve
decoded visible HTML text, not the original HTML markup.

Shared Bulgarian presentation rules operate on a working copy: normalize recognized
marker spacing, remove double quotation marks, use `жк.`, uppercase single entrance
letters, and title-case all-uppercase place names while retaining abbreviations
and Roman numerals. Apostrophes and meaningful mixed-case names are preserved.
Recognized empty or asterisk-only delivery components and explicit post-box
components are omitted. Short-number `П.К` delivery components are treated as
post boxes; four-digit postcode-like values are retained. Unlabelled numbers
are not guessed into telephone, entrance, floor or apartment fields.

A `locations` country hint chooses extraction rules; it is not evidence of physical
country. VIES country metadata follows VAT jurisdiction. Registry country metadata
requires explicit recognized source country text and uses the shared native name.
Unsupported formats retain unknown components. Provider envelopes preserve source
text so applications can inspect what could not be structured.
