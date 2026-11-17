//! Anonymous Slovak registry observations.
mod client;
mod decode;
mod identifiers;
mod mapping;
mod model;
pub use client::{SkRegistryClient, SkRegistryConfig};
pub use identifiers::{SkBusinessId, validate_sk_business_id_syntax};
pub use model::*;
#[cfg(test)]
mod probe;
#[cfg(test)]
mod tests;
