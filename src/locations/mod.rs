//! Provider-independent address parsing, normalization and country metadata.
mod address;
mod country;
mod delivery;
pub(crate) mod normalize;
mod parser;
mod rules;
pub use address::Address;
pub use country::Country;

/// Structure plain address text using an ISO country-format hint (for example `bg`).
/// The hint chooses rules; it never proves the physical address country.
/// Unsupported formats retain no guessed components; callers retain source text.
pub fn parse_address(raw: &str, localization: &str) -> Address {
    let code = localization.to_uppercase();
    let code = if code == "GR" { "EL" } else { &code };
    parser::split_fields(code, raw)
}
/// Whether extraction rules exist for an ISO country-format hint.
pub fn supports_address(localization: &str) -> bool {
    let code = localization.to_uppercase();
    rules::rule(if code == "GR" { "EL" } else { &code }).is_some()
}
/// Normalize and split a Bulgarian delivery line; does not interpret locality labels.
pub fn split_delivery(raw: &str, localization: &str) -> (Option<String>, Option<String>) {
    if localization.eq_ignore_ascii_case("bg") {
        delivery::bulgarian_delivery(&normalize::address("BG", raw))
    } else {
        (Some(raw.to_owned()), None)
    }
}
