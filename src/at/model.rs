use serde::{Deserialize, Serialize};
/// Matched public Firmenbuch detail observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AtCompanyLookupResult {
    /// Full matched Firmenbuch number.
    pub tax_id: crate::AtBusinessId,
    /// Matched source route identifier, not a national ID.
    pub record_id: String,
    /// Source registered company name.
    pub company_name: String,
    /// Original registry status code.
    pub native_registry_status: String,
    /// Source domicile text.
    pub domicile: Option<String>,
    /// Date returned by the source; not an incorporation date.
    pub source_date: Option<String>,
    /// Source legal form.
    pub legal_form: Option<AtLegalForm>,
    /// Structured source address; no original full-address text is supplied.
    pub registered_office: Option<AtRegistryAddress>,
}
/// Source legal-form label and abbreviation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AtLegalForm {
    /// Native abbreviation.
    pub abbr: Option<String>,
    /// Source legal-form label.
    pub name: Option<String>,
}
/// Structured address returned by public Firmenbuch details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AtRegistryAddress {
    /// Original source street line.
    pub street: Option<String>,
    /// Original postal code, including leading zeroes.
    pub zip_code: Option<String>,
    /// Source locality.
    pub city: Option<String>,
}

/// Native facts retained alongside the common company view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AtFirmenbuchDetails {
    /// Source-returned route identifier.
    pub record_id: String,
    /// Source domicile, distinct from the address locality.
    pub domicile: Option<String>,
    /// Source date, not interpreted as incorporation or cessation.
    pub source_date: Option<String>,
    /// Native legal-form label and abbreviation.
    pub legal_form: Option<AtLegalForm>,
    /// Original structured address.
    pub registered_office: Option<AtRegistryAddress>,
}
