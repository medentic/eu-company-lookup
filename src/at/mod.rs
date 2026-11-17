//! Anonymous JustizOnline public Firmenbuch search and details.
mod client;
mod decode;
mod identifiers;
mod mapping;
mod model;
pub use client::{AtRegistryClient, AtRegistryConfig};
pub use identifiers::{AtBusinessId, validate_at_business_id_syntax};
pub use model::{AtCompanyLookupResult, AtFirmenbuchDetails, AtLegalForm, AtRegistryAddress};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod probe;
