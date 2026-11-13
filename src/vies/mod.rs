mod address;
pub(crate) mod capabilities;
mod client;
mod identifiers;
#[cfg(test)]
mod reporting;
mod soap;
mod transport;

pub use client::{Client, ClientConfig};
pub use identifiers::{VatId, validate_vat_syntax};

#[cfg(test)]
mod tests;
