use serde::Serialize;

/// Source of an observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum Provider {
    /// European Commission VAT Information Exchange System.
    Vies,
}

/// An implemented operation, not a roadmap promise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum Operation {
    /// Offline syntax checking and normalization.
    ValidateVatSyntax,
    /// VIES validity and optional owner information.
    LookupVat,
    /// Boolean convenience wrapper around the same check.
    IsVatValid,
}

/// Identifier scheme accepted by an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum IdentifierScheme {
    /// Country-prefixed VAT identifier; not a national company identifier.
    Vat,
}

/// Checksum coverage, independent of syntax and registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum ChecksumSupport {
    /// No checksum verification is performed.
    NotImplemented,
}

/// Source language behavior, not application UI language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum LanguageSupport {
    /// The source operation accepts no language selection parameter.
    NotSelectable,
}

/// Operations and language behavior of an enabled provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderCapabilities {
    /// Provider implementing these operations.
    pub provider: Provider,
    /// Implemented operations for this jurisdiction.
    pub operations: Vec<Operation>,
    /// Accepted identifier scheme.
    pub identifier_scheme: IdentifierScheme,
    /// Whether local checksum validation is implemented.
    pub checksum: ChecksumSupport,
    /// Language behavior shared by these operations.
    pub languages: LanguageSupport,
    /// Whether callers must supply credentials.
    pub requires_credentials: bool,
    /// The provider may supply a name and original address, but can withhold both.
    pub optional_owner_details: bool,
}

/// Capability grouping with an explicit VAT-prefix-to-jurisdiction mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JurisdictionCapabilities {
    /// ISO 3166-1 country code, e.g. GR for Greece or GB for Northern Ireland.
    pub country_code: &'static str,
    /// ISO subdivision when the capability is narrower than the country (GB-NIR).
    pub subdivision_code: Option<&'static str>,
    /// VIES prefix: EL for Greece and XI for Northern Ireland.
    pub vat_prefix: &'static str,
    /// Enabled providers only. This is not a live health check.
    pub providers: Vec<ProviderCapabilities>,
}
