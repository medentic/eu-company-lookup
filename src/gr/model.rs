use serde::Serialize;

/// Exact GEMI company observation, separate from AFM and VIES verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct GrCompanyLookupResult {
    /// Full twelve-digit GEMI number, including leading zeroes.
    pub tax_id: super::GrBusinessId,
    /// Original registered name.
    pub company_name: String,
    /// Source Latin-script registered name, when supplied.
    pub foreign_name: Option<String>,
    /// Source legal-form label.
    pub legal_form: Option<String>,
    /// Native status label.
    pub native_registry_status: Option<String>,
    /// Original full registered-office address; never substituted with map text.
    pub full_address_by_registry: Option<String>,
    /// Native dates, codes and tax number.
    pub details: GrGemiDetails,
}

/// Native GEMI facts; AFM does not establish VAT registration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct GrGemiDetails {
    /// Source tax number (AFM), distinct from the GEMI lookup identity.
    pub tax_identifier: Option<String>,
    /// Source legal-type code.
    pub legal_type_code: Option<String>,
    /// Source status code.
    pub status_code: Option<String>,
    /// Original formation date text.
    pub start_date: Option<String>,
    /// Original GEMI registration date text.
    pub gemi_registration_date: Option<String>,
    /// Source address prepared for map links; not the official full address.
    pub map_address: Option<String>,
    /// Source branch classification.
    pub is_branch: Option<bool>,
    /// Source foreign-branch classification.
    pub is_foreign_branch: Option<bool>,
}
