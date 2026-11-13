mod address;
pub(crate) mod capabilities;
mod client;
mod identifiers;
#[cfg(test)]
mod reporting;
mod soap;
mod transport;

#[cfg(test)]
pub(crate) use crate::Client;
pub use client::ClientConfig;
pub(crate) use client::ViesClient;
pub use identifiers::{VatId, validate_vat_syntax};

#[cfg(test)]
mod tests;
