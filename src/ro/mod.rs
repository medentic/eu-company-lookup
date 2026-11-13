mod client;
mod decode;
mod identifiers;
mod model;

pub use client::{RoRegistryClient, RoRegistryConfig};
pub use identifiers::{RoBusinessId, validate_ro_business_id_syntax};
pub use model::{RoCompanyLookupResult, RoRegistryAddress, RoTaxStatus};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod probe;
