use super::Provider;
use crate::VatId;
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Original VIES address and conservative extracted components.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct VatAddress {
    /// Unchanged source text, including placeholders; None only when absent or nil.
    pub full_address_by_vies: Option<String>,
    /// Shared normalized address components.
    #[serde(flatten, serialize_with = "serialize_vat_fields")]
    pub fields: crate::locations::Address,
}

/// Identifier-matched VAT response. Serialized output contains only the public
/// billing fields; observation timestamps are retained privately for future reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct VatCheck {
    /// Normalized identifier actually checked.
    pub vat_id: VatId,
    /// VIES validity, independent of address parsing success.
    pub valid: bool,
    /// Provider that returned the observation.
    pub source: Provider,
    /// Reported company name; blank/withheld values become None.
    pub company: Option<String>,
    /// Original address and available structured components.
    pub address: VatAddress,
    #[serde(skip)]
    pub(crate) request_date: String,
    #[serde(skip)]
    pub(crate) checked_at: DateTime<Utc>,
}

impl std::ops::Deref for VatAddress {
    type Target = crate::locations::Address;
    fn deref(&self) -> &Self::Target {
        &self.fields
    }
}
impl std::ops::DerefMut for VatAddress {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fields
    }
}
fn serialize_vat_fields<S: serde::Serializer>(
    a: &crate::locations::Address,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeMap;
    let mut map = serializer.serialize_map(Some(7))?;
    map.serialize_entry("address", &a.address)?;
    map.serialize_entry("address2", &a.address2)?;
    map.serialize_entry("city", &a.city)?;
    map.serialize_entry("postcode", &a.postcode)?;
    map.serialize_entry("region", &a.region)?;
    map.serialize_entry("country", &a.country)?;
    map.serialize_entry("countryISO2", &a.country_iso2)?;
    map.end()
}
