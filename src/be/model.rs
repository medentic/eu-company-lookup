use serde::{Deserialize, Serialize};

/// Matched NBB Consult company observation. Coverage is limited to NBB's data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct BeCompanyLookupResult {
    /// Matched enterprise number, including its leading zero.
    pub tax_id: super::BeBusinessId,
    /// Source name, without translation by this library.
    pub company_name: String,
    /// Source legal-form label requested in English.
    pub legal_form: Option<String>,
    /// Original source legal-situation label.
    pub native_registry_status: Option<String>,
    /// Source structured registered-office components.
    pub registered_office: BeRegistryAddress,
    /// Source metadata and codes.
    pub details: BeNbbDetails,
}

/// NBB's structured address components; no synthesized original address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct BeRegistryAddress {
    /// Source street name.
    pub street_name: Option<String>,
    /// Source building number.
    pub street_number: Option<String>,
    /// Source box or unit number.
    pub box_number: Option<String>,
    /// Source postcode, including leading zeroes.
    pub postal_code: Option<String>,
    /// Source municipality label.
    pub town: Option<String>,
    /// Source country label.
    pub country: Option<String>,
    /// Source country code.
    pub country_code: Option<String>,
}

/// Native NBB metadata; legal-situation codes are not normalized lifecycle claims.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct BeNbbDetails {
    /// Source record language.
    pub language: Option<String>,
    /// Source language-regime code.
    pub language_regime: Option<String>,
    /// Source municipality code.
    pub nis_code: Option<String>,
    /// Native legal-form code.
    pub legal_form_code: Option<String>,
    /// Native legal-situation code.
    pub legal_situation_code: Option<String>,
    /// Original legal-situation date; not an incorporation date.
    pub legal_situation_date: Option<String>,
}
