use serde::{Deserialize, Serialize};

/// Source of an observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum Provider {
    /// European Commission VAT Information Exchange System.
    Vies,
    /// Bulgarian Commercial Register and Register of Non-Profit Legal Entities.
    BulgarianCommercialRegister,
    /// Bulgarian BULSTAT register.
    Bulstat,
    /// Romanian National Agency for Fiscal Administration.
    RomanianAnaf,
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
    /// Country-independent lookup of an explicitly typed national identifier.
    LookupCompany,
    /// Offline Bulgarian business-identifier syntax check.
    ValidateBgBusinessIdSyntax,
    /// Bulgarian business lookup, with Commercial Register then BULSTAT fallback.
    LookupBgCompany,
    /// Offline Romanian CUI syntax check.
    ValidateRoBusinessIdSyntax,
    /// Romanian taxpayer lookup through ANAF.
    LookupRoCompany,
    /// Romanian fiscal lookup with an explicit requested date.
    LookupRoCompanyAt,
}

/// Whether an operation makes provider requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum Execution {
    /// Local syntax validation; no registration claim.
    Local,
    /// Remote provider lookup; operational failures remain errors.
    Remote,
}

/// Date accepted by a specific operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum DateInput {
    /// No caller-supplied date parameter.
    None,
    /// Requested fiscal date, not a historical company extract.
    FiscalDate,
}

/// Typed input contract for one implemented operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct OperationCapabilities {
    /// Callable operation.
    pub operation: Operation,
    /// Accepted schemes; national schemes remain country-specific.
    pub identifier_schemes: Vec<IdentifierScheme>,
    /// Whether provider requests are made.
    pub execution: Execution,
    /// Supported date parameter, if any.
    pub date_input: DateInput,
}
impl OperationCapabilities {
    pub(crate) fn new(operation: Operation, scheme: IdentifierScheme) -> Self {
        let execution = match operation {
            Operation::ValidateVatSyntax
            | Operation::ValidateBgBusinessIdSyntax
            | Operation::ValidateRoBusinessIdSyntax => Execution::Local,
            _ => Execution::Remote,
        };
        Self {
            operation,
            identifier_schemes: vec![scheme],
            execution,
            date_input: if operation == Operation::LookupRoCompanyAt {
                DateInput::FiscalDate
            } else {
                DateInput::None
            },
        }
    }
}

/// Identifier scheme accepted by an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum IdentifierScheme {
    /// Country-prefixed VAT identifier; not a national company identifier.
    Vat,
    /// Nine- or thirteen-digit Bulgarian ЕИК/BULSTAT business code, without BG.
    BgBusinessId,
    /// Romanian CUI without the RO VAT prefix.
    RoBusinessId,
}

/// Checksum coverage, independent of syntax and registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum ChecksumSupport {
    /// No checksum verification is performed.
    NotImplemented,
}

/// Source language behavior, independent of application UI language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct LanguageSupport {
    /// Known source language codes; empty means source-dependent or unspecified.
    pub available: Vec<&'static str>,
    /// Language requested by the adapter, if specified.
    pub default: Option<&'static str>,
    /// Whether this adapter accepts a caller-selected source language.
    pub selectable: bool,
}

/// Operations and language behavior of an enabled provider.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ProviderCapabilities {
    /// Provider implementing these operations.
    pub provider: Provider,
    /// Implemented operations for this jurisdiction.
    pub operations: Vec<OperationCapabilities>,
    /// Whether local checksum validation is implemented.
    pub checksum: ChecksumSupport,
    /// Language behavior shared by these operations.
    pub languages: LanguageSupport,
    /// Whether callers must supply credentials.
    pub requires_credentials: bool,
    /// Supplementary company identity/address fields may be missing; individual
    /// adapters can require a name for a successful record.
    pub optional_owner_details: bool,
    /// Conservative address rules exist for this jurisdiction; individual formats may remain unparsed.
    pub address_splitting: bool,
}

/// Capability grouping with an explicit VAT-prefix-to-jurisdiction mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct JurisdictionCapabilities {
    /// ISO 3166-1 country code, e.g. GR for Greece or GB for Northern Ireland.
    pub country_code: &'static str,
    /// ISO subdivision when the capability is narrower than the country (GB-NIR).
    pub subdivision_code: Option<&'static str>,
    /// VIES prefix: EL for Greece and XI for Northern Ireland.
    pub vat_prefix: Option<&'static str>,
    /// Enabled providers only. This is not a live health check.
    pub providers: Vec<ProviderCapabilities>,
}

impl ProviderCapabilities {
    /// Whether this provider advertises the operation for the supplied scheme.
    pub fn supports(&self, operation: Operation, scheme: IdentifierScheme) -> bool {
        self.operations
            .iter()
            .any(|c| c.operation == operation && c.identifier_schemes.contains(&scheme))
    }
}
