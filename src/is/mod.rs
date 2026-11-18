//! Anonymous Icelandic registry observations.
mod client;
mod decode;
mod identifiers;
mod mapping;
mod model;
mod overview;
pub use client::{IsRegistryClient, IsRegistryConfig};
pub use identifiers::{IsBusinessId, validate_is_business_id_syntax};
pub use model::*;
#[cfg(test)]
mod probe;
#[cfg(test)]
mod tests;
