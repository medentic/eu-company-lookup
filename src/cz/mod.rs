//! Czech ARES economic subject lookup.
mod client;
mod decode;
mod identifiers;
mod legal_forms;
mod mapping;
mod model;
pub use client::{CzRegistryClient, CzRegistryConfig};
pub use identifiers::{CzBusinessId, validate_cz_business_id_syntax};
pub use model::{
    CzAresDetails, CzCompanyLookupResult, CzRegistryAddress, CzRosClosure, CzSourceRegistrations,
};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod probe;
