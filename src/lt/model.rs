use serde::{Deserialize, Serialize};
/// Opaque source reference; no inferred label or follow-up lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LtSourceReference {
    /// Original referenced source record ID.
    #[serde(rename = "_id")]
    pub id: String,
}
/// Selected native published legal-entity fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LtCompanyDetails {
    /// Original source model type.
    #[serde(rename = "_type")]
    pub source_type: String,
    /// Original internal record identifier.
    #[serde(rename = "_id")]
    pub source_id: String,
    /// Source revision identifier.
    #[serde(rename = "_revision")]
    pub revision: Option<String>,
    /// Full company code, numeric JSON represented as decimal text.
    #[serde(deserialize_with = "code")]
    pub ja_kodas: String,
    /// Published name, possibly anonymized by the source.
    pub ja_pavadinimas: String,
    /// Source full address text, without inferred common role.
    pub pilnas_adresas: Option<String>,
    /// Native address reference.
    pub adresas: Option<LtSourceReference>,
    /// Native legal-form reference.
    pub forma: Option<LtSourceReference>,
    /// Native status reference.
    pub statusas: Option<LtSourceReference>,
    /// Original registration date.
    pub reg_data: Option<String>,
    /// Original removal date.
    pub isreg_data: Option<String>,
    /// Original status date.
    pub stat_data: Option<String>,
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
/// Exact matched published Lithuanian legal entity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct LtCompanyLookupResult {
    /// Full requested company code.
    pub tax_id: super::LtBusinessId,
    /// Published source name.
    pub company_name: String,
    /// Native dataset fields.
    pub details: LtCompanyDetails,
}
