//! Anonymous French API Recherche Entreprises observations.
mod client;
mod decode;
mod identifiers;
mod legal_forms;
mod mapping;
mod model;
pub use client::{FrRegistryClient, FrRegistryConfig};
pub use identifiers::{FrBusinessId, validate_fr_business_id_syntax};
pub use model::{FrCompanyDetails, FrCompanyLookupResult, FrHeadOffice};
#[cfg(test)]
mod probe;
#[cfg(test)]
mod tests;
