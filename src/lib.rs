#![doc = include_str!("../README.md")]
mod at;
mod be;
mod bg;
mod capabilities;
mod client;
mod cz;
mod ee;
mod fi;
mod fr;
mod gr;
mod ie;
mod is;
mod lt;
mod lv;
mod no;
mod pl;
mod sk;
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
pub use vies::{ClientConfig, VatChecksumResult, VatId, validate_vat_syntax};

pub use ro::{
    RoBusinessId, RoCompanyLookupResult, RoRegistryAddress, RoRegistryClient, RoRegistryConfig,
    RoTaxStatus, validate_ro_business_id_syntax,
};

pub use cz::{
    CzAresDetails, CzBusinessId, CzCompanyLookupResult, CzRegistryAddress, CzRegistryClient,
    CzRegistryConfig, CzRosClosure, CzSourceRegistrations, validate_cz_business_id_syntax,
};

pub use at::{
    AtBusinessId, AtCompanyLookupResult, AtFirmenbuchDetails, AtLegalForm, AtRegistryAddress,
    AtRegistryClient, AtRegistryConfig, validate_at_business_id_syntax,
};

pub use be::{
    BeBusinessId, BeCompanyLookupResult, BeNbbDetails, BeRegistryAddress, BeRegistryClient,
    BeRegistryConfig, validate_be_business_id_syntax,
};

pub use gr::{
    GrBusinessId, GrCompanyLookupResult, GrGemiDetails, GrRegistryClient, GrRegistryConfig,
    validate_gr_business_id_syntax,
};

pub use fi::{
    FiBusinessId, FiBusinessLine, FiCompanyForm, FiCompanyLookupResult, FiCompanySituation,
    FiDescription, FiPostOffice, FiPrhDetails, FiRegisterEntry, FiRegisterName, FiRegistryAddress,
    FiRegistryClient, FiRegistryConfig, FiSourceIdentifier, FiWebsite,
    validate_fi_business_id_syntax,
};

pub use fr::{
    FrBusinessId, FrCompanyDetails, FrCompanyLookupResult, FrHeadOffice, FrRegistryClient,
    FrRegistryConfig, validate_fr_business_id_syntax,
};

pub use ee::{
    EeBusinessId, EeCompanyDetails, EeCompanyLookupResult, EeRegistryClient, EeRegistryConfig,
    validate_ee_business_id_syntax,
};

pub use no::{
    NoAnnotation, NoBusinessId, NoCompanyDetails, NoCompanyLookupResult, NoRegistryClient,
    NoRegistryConfig, validate_no_business_id_syntax,
};

pub use no::{NoClassification, NoRegistryAddress};

pub use pl::{
    PlBusinessId, PlCompanyDetails, PlCompanyLookupResult, PlRegistryClient, PlRegistryConfig,
    validate_pl_business_id_syntax,
};

pub use pl::{
    PlAddress, PlBankruptcy, PlCorporateEvent, PlDissolution, PlExtractHeader, PlLifecycleSection,
    PlLiquidation, PlMergerKrs, PlMergerParty, PlProceedingEnd, PlProceedingOrder,
    PlRegisteredOffice, PlRemoval, PlReorganization, PlReorganizationRole, PlRestructuring, PlSeat,
    PlSourceIdentifiers, PlSubject, PlSuspension,
};

pub use sk::{
    SkBusinessId, SkCompanyDetails, SkCompanyLookupResult, SkRegistryClient, SkRegistryConfig,
    SkRpoCode, SkRpoLegalState, SkRpoLifecycle, SkRpoValue, validate_sk_business_id_syntax,
};

pub use lv::{
    LvBusinessId, LvCompanyDetails, LvCompanyLookupResult, LvInsolvency, LvLifecycleDetails,
    LvLiquidation, LvRegistryClient, LvRegistryConfig, LvSuspension,
    validate_lv_business_id_syntax,
};

pub use lt::{
    LtBusinessId, LtCompanyDetails, LtCompanyLookupResult, LtRegistryClient, LtRegistryConfig,
    validate_lt_business_id_syntax,
};

pub use lt::LtSourceReference;

pub use ie::{
    IeBusinessId, IeCompanyDetails, IeCompanyLookupResult, IeRegistryClient, IeRegistryConfig,
    validate_ie_business_id_syntax,
};

pub use is::{
    IsBusinessId, IsCompanyDetails, IsCompanyLookupResult, IsRegistryClient, IsRegistryConfig,
    validate_is_business_id_syntax,
};

pub(crate) use model::RegistryFailure;

pub use model::{
    CapturedBytes, CapturedHeader, CapturedRequest, CapturedResponse, ErrorDetails, ErrorId,
    HttpDiagnostics, HttpExchange,
};

#[cfg(test)]
mod test_support;
