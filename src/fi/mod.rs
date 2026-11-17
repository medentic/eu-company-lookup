//! Anonymous Finnish PRH YTJ company observations.
mod client;
mod decode;
mod identifiers;
mod mapping;
mod model;
pub use client::{FiRegistryClient, FiRegistryConfig};
pub use identifiers::{FiBusinessId, validate_fi_business_id_syntax};
pub use model::{
    FiBusinessLine, FiCompanyForm, FiCompanyLookupResult, FiCompanySituation, FiDescription,
    FiPostOffice, FiPrhDetails, FiRegisterEntry, FiRegisterName, FiRegistryAddress,
    FiSourceIdentifier, FiWebsite,
};
#[cfg(test)]
mod probe;
#[cfg(test)]
mod tests;
