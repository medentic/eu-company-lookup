use serde::{Deserialize, Serialize};
/// Selected original registry dataset fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LvCompanyDetails {
    /// Original source `regcode` field.
    #[serde(deserialize_with = "code")]
    pub regcode: String,
    /// Original source `name` field.
    pub name: String,
    /// Original source `sepa` field.
    pub sepa: Option<String>,
    /// Original source `regtype` field.
    pub regtype: Option<String>,
    /// Original source `regtype_text` field.
    pub regtype_text: Option<String>,
    /// Original source `type_text` field.
    pub type_text: Option<String>,
    /// Original source `registered` field.
    pub registered: Option<String>,
    /// Original source `terminated` field.
    pub terminated: Option<String>,
    /// Original source `closed` field.
    pub closed: Option<String>,
    /// Original source `address` field.
    pub address: Option<String>,
    /// Original source `reregistration_term` field.
    pub reregistration_term: Option<String>,
    /// Original source `index` field.
    pub index: Option<u64>,
    /// Original source `addressid` field.
    pub addressid: Option<u64>,
    /// Original source `region` field.
    pub region: Option<u64>,
    /// Original source `city` field.
    pub city: Option<u64>,
    /// Original source `atvk` field.
    pub atvk: Option<u64>,
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
/// Exact matched registry dataset record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct LvCompanyLookupResult {
    /// Full requested registration number.
    pub tax_id: super::LvBusinessId,
    /// Original company name.
    pub company_name: String,
    /// Native source fields.
    pub details: LvCompanyDetails,
}
