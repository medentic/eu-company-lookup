//! Anonymous Polish registry observations.
mod client;
mod decode;
mod full;
mod identifiers;
mod liquidation;
mod mapping;
mod mergers;
mod model;
pub use client::{PlRegistryClient, PlRegistryConfig};
pub use identifiers::{PlBusinessId, validate_pl_business_id_syntax};
pub use model::*;
#[cfg(test)]
mod probe;
#[cfg(test)]
mod tests;
