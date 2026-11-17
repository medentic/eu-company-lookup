use serde::{Deserialize, Serialize};

/// Native identifier and attribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FiSourceIdentifier {
    /// Exact source identifier.
    pub value: String,
    /// Source-system code.
    pub source: String,
    /// Date assigned by the source, if supplied.
    pub registration_date: Option<String>,
}

/// Source label in its supplied language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FiDescription {
    /// Native language code.
    pub language_code: String,
    /// Original source label.
    pub description: String,
}

/// Primary, parallel or auxiliary name and version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FiRegisterName {
    /// Original name.
    pub name: String,
    /// Native name-type code.
    pub r#type: String,
    /// Source version; 1 is the latest.
    pub version: u32,
    /// Native source code.
    pub source: String,
    /// Source start date.
    pub registration_date: Option<String>,
    /// Source end date.
    pub end_date: Option<String>,
}

/// Native legal-form code, labels and version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FiCompanyForm {
    /// Native legal-form code.
    pub r#type: String,
    /// Source labels, without generated translations.
    #[serde(default)]
    pub descriptions: Vec<FiDescription>,
    /// Source version; 1 is the latest.
    pub version: u32,
    /// Native source code.
    pub source: String,
    /// Source start date.
    pub registration_date: Option<String>,
    /// Source end date.
    pub end_date: Option<String>,
}

/// Source restructuring, liquidation or bankruptcy observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FiCompanySituation {
    /// Native situation code.
    pub r#type: String,
    /// Source start date.
    pub registration_date: Option<String>,
    /// Source end date.
    pub end_date: Option<String>,
}

/// Native tax or trade-register entry; not a VIES assertion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FiRegisterEntry {
    /// Native entry-type code.
    pub r#type: String,
    /// Native register code.
    pub register: String,
    /// Native authority code.
    pub authority: String,
    /// Source labels.
    #[serde(default)]
    pub descriptions: Vec<FiDescription>,
    /// Source start date.
    pub registration_date: Option<String>,
    /// Source end date.
    pub end_date: Option<String>,
}

/// Source locality and language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FiPostOffice {
    /// Original locality name.
    pub city: String,
    /// Native language code.
    pub language_code: String,
    /// Source municipality code.
    pub municipality_code: Option<String>,
}

/// Native street or postal contact address, with its explicit role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FiRegistryAddress {
    /// Source role: 1 street, 2 postal; other values retained.
    pub r#type: u32,
    /// Native source code.
    pub source: String,
    /// Original street.
    pub street: Option<String>,
    /// Original postcode, preserving zeroes.
    pub post_code: Option<String>,
    /// Source localities in their languages.
    #[serde(default)]
    pub post_offices: Vec<FiPostOffice>,
    /// Source PO box.
    pub post_office_box: Option<String>,
    /// Source building number.
    pub building_number: Option<String>,
    /// Source entrance.
    pub entrance: Option<String>,
    /// Source apartment number.
    pub apartment_number: Option<String>,
    /// Source apartment suffix.
    pub apartment_id_suffix: Option<String>,
    /// Source care-of text.
    pub co: Option<String>,
    /// Source country code; no jurisdiction default.
    pub country: Option<String>,
    /// Original provider-encoded free address text; no underscore replacement.
    pub free_address_line: Option<String>,
    /// Source address date.
    pub registration_date: Option<String>,
}

/// Native business-line classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FiBusinessLine {
    /// Native classification code.
    pub r#type: String,
    /// Native source code.
    pub source: String,
    /// Source labels.
    #[serde(default)]
    pub descriptions: Vec<FiDescription>,
    /// Native code-list identifier.
    pub type_code_set: Option<String>,
    /// Source classification date.
    pub registration_date: Option<String>,
}

/// Source company website, not an official registry permalink.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FiWebsite {
    /// Original source URL.
    pub url: String,
    /// Native source code.
    pub source: String,
    /// Source date.
    pub registration_date: Option<String>,
}

/// Typed PRH source observations; native codes are not inferred lifecycle or VAT status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FiPrhDetails {
    /// Original business ID and attribution.
    pub business_id: FiSourceIdentifier,
    /// Source EUID.
    pub eu_id: Option<FiSourceIdentifier>,
    /// Source names and history.
    #[serde(default)]
    pub names: Vec<FiRegisterName>,
    /// Source business classification.
    pub main_business_line: Option<FiBusinessLine>,
    /// Source company website.
    pub website: Option<FiWebsite>,
    /// Source legal forms and history.
    #[serde(default)]
    pub company_forms: Vec<FiCompanyForm>,
    /// Source situations.
    #[serde(default)]
    pub company_situations: Vec<FiCompanySituation>,
    /// Native register entries.
    #[serde(default)]
    pub registered_entries: Vec<FiRegisterEntry>,
    /// Source street and postal addresses; roles retained.
    #[serde(default)]
    pub addresses: Vec<FiRegistryAddress>,
    /// Native Trade Register status code.
    pub trade_register_status: String,
    /// Native business-ID status code.
    pub status: Option<String>,
    /// Source company registration date.
    pub registration_date: Option<String>,
    /// Source dissolution date.
    pub end_date: Option<String>,
    /// Source local timestamp when present; no timezone inferred.
    pub last_modified: Option<String>,
}

/// Exact matched Finnish company observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct FiCompanyLookupResult {
    /// Matched business ID, preserving its hyphen and zeroes.
    pub tax_id: super::FiBusinessId,
    /// Latest primary source name, including for dissolved companies.
    pub company_name: String,
    /// Finnish source legal-form label, if unambiguous.
    pub legal_form: Option<String>,
    /// Original Trade Register status code.
    pub native_registry_status: Option<String>,
    /// Source observations and address roles.
    pub details: FiPrhDetails,
}
