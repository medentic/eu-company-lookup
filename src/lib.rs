#![doc = include_str!("../README.md")]

mod bg;
mod capabilities;
mod client;

mod ie;

mod no;

pub use capabilities::capabilities;
pub use client::{Client, ClientBuilder};
mod http_capture;
pub mod locations;
mod model;
mod registry_transport;
mod ro;
mod vies;

pub use bg::{BgBusinessId, BgRegistryClient, BgRegistryConfig, validate_bg_business_id_syntax};
pub use model::{
    AnafDetails, BgCompanyLookupResult, BulstatDetails, BusinessId, ChecksumSupport,
    CommercialRegisterDetails, CompanyAddress, CompanyDetails, CompanyLookupResult, CompanyRecord,
    CompanyStatus, DateInput, Execution, IdentifierScheme, JurisdictionCapabilities,
    LanguageSupport, LegalForm, LimitScope, LookupError, Operation, OperationCapabilities,
    Provider, ProviderCapabilities, RegistryCapital, RegistryDetails, RegistryDocument,
    RegistryDocumentType, RegistryMoney, RegistryPerson, RegistryRole, RetryAfter,
    UnavailabilityReason, VatAddress, VatCheck, VatLookupResult,
};
pub use vies::{ClientConfig, VatId, validate_vat_syntax};

pub use ro::{
    RoBusinessId, RoCompanyLookupResult, RoRegistryAddress, RoRegistryClient, RoRegistryConfig,
    RoTaxStatus, validate_ro_business_id_syntax,
};

pub use no::{
    NoBusinessId, NoCompanyDetails, NoCompanyLookupResult, NoRegistryClient, NoRegistryConfig,
    validate_no_business_id_syntax,
};

pub use no::{NoClassification, NoRegistryAddress};

pub use ie::{
    IeBusinessId, IeCompanyDetails, IeCompanyLookupResult, IeRegistryClient, IeRegistryConfig,
    validate_ie_business_id_syntax,
};

pub(crate) use model::RegistryFailure;

pub use model::{
    CapturedBytes, CapturedHeader, CapturedRequest, CapturedResponse, ErrorDetails, ErrorId,
    HttpDiagnostics, HttpExchange,
};
