//! Anonymous Irish registry observations.
mod client;
mod decode;
mod identifiers;
mod mapping;
mod model;
pub use client::{IeRegistryClient, IeRegistryConfig};
pub use identifiers::{IeBusinessId, validate_ie_business_id_syntax};
pub use model::*;
#[cfg(test)]
mod probe;
#[cfg(test)]
mod tests;
