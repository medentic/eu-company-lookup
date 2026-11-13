#![doc = include_str!("../README.md")]
mod bg;
mod capabilities;
pub mod locations;
mod model;
mod ro;
mod vies;

pub use bg::{BgBusinessId, BgRegistryClient, BgRegistryConfig, validate_bg_business_id_syntax};
pub use model::{
    AnafDetails, BgCompanyLookupResult, BulstatDetails, BusinessId, ChecksumSupport,
    CommercialRegisterDetails, CompanyAddress, CompanyDetails, CompanyLookupResult, CompanyRecord,
    CompanyStatus, IdentifierScheme, JurisdictionCapabilities, LanguageSupport, LegalForm,
    LimitScope, LookupError, Operation, Provider, ProviderCapabilities, RegistryCapital,
    RegistryDetails, RegistryDocument, RegistryDocumentType, RegistryError, RegistryMoney,
    RegistryPerson, RegistryRole, RetryAfter, UnavailabilityReason, VatAddress, VatCheck,
    VatLookupResult,
};
pub use vies::{Client, ClientConfig, VatId, validate_vat_syntax};

pub use ro::{
    RoBusinessId, RoCompanyLookupResult, RoRegistryAddress, RoRegistryClient, RoRegistryConfig,
    RoTaxStatus, validate_ro_business_id_syntax,
};
