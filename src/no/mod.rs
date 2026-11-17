//! Anonymous Norwegian registry observations.
mod client;
mod decode;
mod identifiers;
mod mapping;
mod model;
pub use client::{NoRegistryClient, NoRegistryConfig};
pub use identifiers::{NoBusinessId, validate_no_business_id_syntax};
pub use model::*;
#[cfg(test)]
mod probe;
#[cfg(test)]
mod tests;
