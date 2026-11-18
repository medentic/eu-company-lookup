/// Selected decoded company page fields; no officers or ownership data.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct IsCompanyDetails {
    /// Explicit lifecycle from the identity-matched free registry overview, when needed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration_overview: Option<IsRegistrationOverview>,
    /// Exact company identifier from the heading.
    pub kennitala: String,
    /// Original decoded registration subtitle.
    pub registration_text: Option<String>,
    /// Original decoded postal-address cell.
    pub postal_address: String,
    /// Original decoded legal-address cell.
    pub legal_address: String,
    /// Original decoded municipality cell.
    pub municipality_text: String,
    /// Original decoded legal-form cell.
    pub legal_form_text: String,
    /// Explicit first legal-form text segment, when the source has exactly two.
    pub legal_form_code: Option<String>,
    /// Explicit second legal-form text segment, when the source has exactly two.
    pub legal_form_description: Option<String>,
}
/// Matched public Icelandic legal-entity observation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct IsCompanyLookupResult {
    /// Full requested legal-entity kennitala.
    pub tax_id: super::IsBusinessId,
    /// Company name decoded from the identity-bound heading.
    pub company_name: String,
    /// Typed source page fields.
    pub details: IsCompanyDetails,
}

/// Selected lifecycle evidence from a free Skatturinn registration overview.
/// Officers, personal identifiers and document contents are not exposed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct IsRegistrationOverview {
    /// Exact legal-entity identifier in the document.
    pub kennitala: String,
    /// Date printed on the official overview.
    pub issued_on: crate::Date,
    /// Decoded Icelandic status text; PDF positioning may omit visual spaces.
    pub native_status: String,
}
