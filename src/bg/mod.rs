//! Bulgarian business lookup: Commercial Register first, BULSTAT only after a definitive miss.
//! Provider field mappings and maintenance boundaries are documented in the adjacent README.md.
mod address;
mod bulstat;
mod client;
mod commercial;
mod html;
mod identifiers;
mod legal_form;

pub use client::{BgRegistryClient, BgRegistryConfig};
pub use identifiers::{BgBusinessId, validate_bg_business_id_syntax};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod probe;

#[cfg(test)]
mod bulstat_probe;
