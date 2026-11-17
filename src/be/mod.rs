//! Anonymous NBB Consult identity observations, sourced by NBB from CBE.
mod client;
mod decode;
mod identifiers;
mod model;
pub use client::{BeRegistryClient, BeRegistryConfig};
pub use identifiers::{BeBusinessId, validate_be_business_id_syntax};
pub use model::{BeCompanyLookupResult, BeNbbDetails, BeRegistryAddress};
#[cfg(test)]
mod tests;

mod mapping;
#[cfg(test)]
mod probe;
