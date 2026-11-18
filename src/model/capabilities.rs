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
    /// Czech Ministry of Finance ARES economic subjects.
    CzechAres,
    /// Austrian public Firmenbuch via JustizOnline.
    AustrianFirmenbuch,
    /// National Bank of Belgium Consult, with CBE-derived company identity.
    BelgianNbb,
    /// Greek General Commercial Registry public company details.
    GreekGemi,
    /// Finnish Patent and Registration Office YTJ open data.
    FinnishPrh,
    /// French DINUM public company search.
    FrenchRechercheEntreprises,
    /// Icelandic anonymous public registry.
    IcelandicSkatturinn,
    /// Irish anonymous public registry.
    IrishCro,
    /// Lithuanian anonymous public registry.
    LithuanianJar,
    /// Latvian anonymous public registry.
    LatvianOpenData,
    /// Slovak anonymous public registry.
    SlovakRuz,
    /// Polish anonymous public registry.
    PolishKrs,
    /// Norwegian anonymous public registry.
    NorwegianBrreg,
    /// Estonian anonymous public registry.
    EstonianRik,
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
    /// Offline Czech IČO syntax validation.
    ValidateCzBusinessIdSyntax,
    /// Czech ARES economic-subject lookup.
    LookupCzCompany,
    /// Offline Austrian Firmenbuch number syntax validation.
    ValidateAtBusinessIdSyntax,
    /// Austrian public Firmenbuch lookup.
    LookupAtCompany,
    /// Offline Belgian enterprise-number syntax validation.
    ValidateBeBusinessIdSyntax,
    /// Belgian NBB Consult company lookup.
    LookupBeCompany,
    /// Offline Greek GEMI-number syntax validation.
    ValidateGrBusinessIdSyntax,
    /// Greek GEMI public company lookup.
    LookupGrCompany,
    /// Offline Finnish business-ID syntax validation.
    ValidateFiBusinessIdSyntax,
    /// Finnish PRH YTJ company lookup.
    LookupFiCompany,
    /// Offline French SIREN syntax validation.
    ValidateFrBusinessIdSyntax,
    /// French public company lookup.
    LookupFrCompany,
    /// Offline Icelandic identifier syntax.
    ValidateIsBusinessIdSyntax,
    /// Icelandic national lookup.
    LookupIsCompany,
    /// Offline Irish identifier syntax.
    ValidateIeBusinessIdSyntax,
    /// Irish national lookup.
    LookupIeCompany,
    /// Offline Lithuanian identifier syntax.
    ValidateLtBusinessIdSyntax,
    /// Lithuanian national lookup.
    LookupLtCompany,
    /// Offline Latvian identifier syntax.
    ValidateLvBusinessIdSyntax,
    /// Latvian national lookup.
    LookupLvCompany,
    /// Offline Slovak identifier syntax.
    ValidateSkBusinessIdSyntax,
    /// Slovak national lookup.
    LookupSkCompany,
    /// Offline Polish identifier syntax.
    ValidatePlBusinessIdSyntax,
    /// Polish national lookup.
    LookupPlCompany,
    /// Offline Norwegian identifier syntax.
    ValidateNoBusinessIdSyntax,
    /// Norwegian national lookup.
    LookupNoCompany,
    /// Offline Estonian identifier syntax.
    ValidateEeBusinessIdSyntax,
    /// Estonian national lookup.
    LookupEeCompany,
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
            | Operation::ValidateRoBusinessIdSyntax
            | Operation::ValidateCzBusinessIdSyntax
            | Operation::ValidateAtBusinessIdSyntax
            | Operation::ValidateBeBusinessIdSyntax
            | Operation::ValidateEeBusinessIdSyntax
            | Operation::ValidateNoBusinessIdSyntax
            | Operation::ValidatePlBusinessIdSyntax
            | Operation::ValidateSkBusinessIdSyntax
            | Operation::ValidateLvBusinessIdSyntax
            | Operation::ValidateLtBusinessIdSyntax
            | Operation::ValidateIeBusinessIdSyntax
            | Operation::ValidateIsBusinessIdSyntax
            | Operation::ValidateFrBusinessIdSyntax
            | Operation::ValidateFiBusinessIdSyntax
            | Operation::ValidateGrBusinessIdSyntax => Execution::Local,
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
    /// Eight-digit Czech IČO, without VAT prefix.
    CzBusinessId,
    /// Austrian Firmenbuch number without FN label or VAT prefix.
    AtBusinessId,
    /// Ten-digit Belgian enterprise number without VAT prefix.
    BeBusinessId,
    /// Twelve-digit Greek GEMI registration number, distinct from AFM.
    GrBusinessId,
    /// Finnish business ID with its hyphen; distinct from a VAT number.
    FiBusinessId,
    /// French nine-digit SIREN.
    FrBusinessId,
    /// Icelandic national identifier.
    IsBusinessId,
    /// Irish national identifier.
    IeBusinessId,
    /// Lithuanian national identifier.
    LtBusinessId,
    /// Latvian national identifier.
    LvBusinessId,
    /// Slovak national identifier.
    SkBusinessId,
    /// Polish national identifier.
    PlBusinessId,
    /// Norwegian national identifier.
    NoBusinessId,
    /// Estonian national identifier.
    EeBusinessId,
}

/// Checksum coverage, independent of syntax and registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum ChecksumSupport {
    /// Lookup checks supported national checksum variants before sending HTTP.
    /// Other variants remain syntax-only; this never proves registration.
    SupportedFormats,
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
