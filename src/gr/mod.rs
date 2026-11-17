//! Anonymous GEMI public company details by complete GEMI number.
mod client;
mod decode;
mod identifiers;
mod model;
pub use client::{GrRegistryClient, GrRegistryConfig};
pub use identifiers::{GrBusinessId, validate_gr_business_id_syntax};
pub use model::{GrCompanyLookupResult, GrGemiDetails};
#[cfg(test)]
mod tests;

mod mapping;
#[cfg(test)]
mod probe;
