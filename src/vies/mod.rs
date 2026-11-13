mod address;
mod capabilities;
mod client;
mod identifiers;
mod soap;

pub use client::{Client, ClientConfig};
pub use identifiers::{VatId, validate_vat_syntax};

#[cfg(test)]
mod tests;
