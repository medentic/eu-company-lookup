//! Public result contracts; provider adapters own source-to-common projections.
//! Keep native evidence separate from normalized lifecycle and address presentation.
mod capabilities;
mod company;
mod company_types;
mod diagnostics;
mod error;
mod error_catalog;
mod failure;
mod registry_failure;
mod vat;

pub use capabilities::{
    ChecksumSupport, DateInput, Execution, IdentifierScheme, JurisdictionCapabilities,
    LanguageSupport, Operation, OperationCapabilities, Provider, ProviderCapabilities,
};
pub use company::{
    BgCompanyLookupResult, BulstatDetails, CommercialRegisterDetails, CompanyAddress,
    CompanyRecord, RegistryCapital, RegistryDetails, RegistryDocument, RegistryMoney,
    RegistryPerson,
};
pub use company_types::{CompanyStatus, LegalForm, RegistryDocumentType, RegistryRole};
pub use diagnostics::{
    CapturedBytes, CapturedHeader, CapturedRequest, CapturedResponse, HttpDiagnostics, HttpExchange,
};
pub use error::{ErrorDetails, LimitScope, LookupError, RetryAfter};
pub use error_catalog::ErrorId;
pub(crate) use failure::LookupFailure;
pub(crate) use registry_failure::RegistryFailure;
pub use registry_failure::UnavailabilityReason;
pub use vat::{VatAddress, VatCheck, VatLookupResult};

mod business;
pub use business::{AnafDetails, BusinessId, CompanyDetails, CompanyLookupResult};
