use super::Provider;
use crate::VatId;
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Original VIES address and conservative extracted components.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VatAddress {
    /// Unchanged source text, including placeholders; None only when absent or nil.
    pub full_address_by_vies: Option<String>,
    /// Normalized primary delivery line, including the street number.
    pub address: Option<String>,
    /// Additional delivery lines/locality and Bulgarian block, entrance, floor or apartment details.
    pub address2: Option<String>,
    /// City or settlement explicitly identified by the source format.
    pub city: Option<String>,
    /// Postcode as written, retaining prefixes and spaces.
    pub postcode: Option<String>,
    /// Region explicitly identified by the source format.
    pub region: Option<String>,
    /// Explicit country text only; never inferred from the VAT prefix.
    pub country: Option<String>,
}

/// Identifier-matched VAT response. Serialized output contains only the public
/// billing fields; observation timestamps are retained privately for future reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
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
