#![doc = include_str!("../README.md")]
mod bg;
pub mod locations;
mod model;
mod vies;

pub use bg::{BgBusinessId, BgRegistryClient, BgRegistryConfig, validate_bg_business_id_syntax};
pub use model::{
    BulstatDetails, ChecksumSupport, CommercialRegisterDetails, CompanyAddress, CompanyRecord,
    CompanyStatus, IdentifierScheme, JurisdictionCapabilities, LanguageSupport, LegalForm,
    LimitScope, LookupError, Operation, Provider, ProviderCapabilities, RegistryCapital,
    RegistryDetails, RegistryDocument, RegistryDocumentType, RegistryError, RegistryMoney,
    RegistryPerson, RegistryRole, RetryAfter, UnavailabilityReason, VatAddress, VatCheck,
};
pub use vies::{Client, ClientConfig, VatId, validate_vat_syntax};
