use serde::{Deserialize, Serialize};
/// Selected public portal facts, without inferred status or legal-form labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct EeCompanyDetails {
    /// Native internal company identifier, not registry identity.
    #[serde(alias = "company_id")]
    pub company_id: u64,
    /// Full source registry code, with numeric JSON represented as decimal text.
    #[serde(alias = "reg_code", deserialize_with = "code")]
    pub reg_code: String,
    /// Original registered name.
    pub name: String,
    /// Previous source names.
    #[serde(alias = "historical_names", default)]
    pub historical_names: Vec<String>,
    /// Native status code.
    pub status: Option<String>,
    /// Original legal-address text.
    #[serde(alias = "legal_address")]
    pub legal_address: Option<String>,
    /// Source postcode, preserving zeroes.
    #[serde(alias = "zip_code")]
    pub zip_code: Option<String>,
    /// Native legal-form code, not a generated label.
    #[serde(alias = "legal_form")]
    pub legal_form: Option<String>,
    /// Original portal URL; no common link claim.
    pub url: Option<String>,
}
fn code<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Code {
        Text(String),
        Number(u64),
    }
    Ok(match Code::deserialize(d)? {
        Code::Text(s) => s,
        Code::Number(n) => n.to_string(),
    })
}
/// Matched Estonian registry observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct EeCompanyLookupResult {
    /// Complete requested registry code.
    pub tax_id: super::EeBusinessId,
    /// Original source company name.
    pub company_name: String,
    /// Typed native portal observations.
    pub details: EeCompanyDetails,
}
