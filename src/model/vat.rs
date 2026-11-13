use super::Provider;
use crate::VatId;
use chrono::{DateTime, Utc};
use serde::Serialize;

/// Source-provided identity on an affirmative check. Neither field is guaranteed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VatOwner {
    /// Original source name. Blank/withheld values become None.
    pub name: Option<String>,
    /// Original address with line breaks; not parsed, translated or validated.
    pub address: Option<String>,
}

/// A successful, identifier-matched VIES observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VatCheck {
    /// Normalized identifier actually checked.
    pub vat_id: VatId,
    /// VIES validity at the time of this observation, not national company status.
    pub valid: bool,
    /// Validated XML Schema date, preserving any source timezone suffix.
    pub request_date: String,
    /// Response completion time in UTC, from this machine's clock.
    pub checked_at: DateTime<Utc>,
    /// Optional owner data; always None on an invalid VAT response.
    pub owner: Option<VatOwner>,
    /// Provider that returned the observation.
    pub source: Provider,
}
