// VIES source envelope and VAT-prefix country policy only.
#[cfg(test)]
use crate::locations::normalize;
use crate::{
    VatAddress,
    locations::{self, Country},
};
pub(super) fn supports(prefix: &str) -> bool {
    locations::supports_address(prefix)
}
fn country(prefix: &str) -> Option<(&'static str, &'static str)> {
    let country = Country::from_iso2(match prefix {
        "EL" => "GR",
        "XI" => "GB",
        other => other,
    })?;
    Some((country.iso2(), country.native_name()))
}
pub(super) fn split(prefix: &str, original: Option<String>) -> VatAddress {
    let mut fields = original
        .as_deref()
        .map(|raw| locations::parse_address(raw, prefix))
        .unwrap_or_default();
    if let Some((iso, name)) = country(prefix) {
        fields.country_iso2 = Some(iso.into());
        if fields.country.is_none() {
            fields.country = Some(name.into());
        }
    }
    VatAddress {
        full_address_by_vies: original,
        fields,
    }
}
#[cfg(test)]
mod corpus;
#[cfg(test)]
mod tests;
