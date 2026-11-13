#![doc = include_str!("../README.md")]
mod model;
mod vies;

pub use model::*;
pub use vies::{Client, ClientConfig, VatId, validate_vat_syntax};
