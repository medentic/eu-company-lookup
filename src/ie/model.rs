use serde::{Deserialize, Serialize};
/// Selected native CRO company fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct IeCompanyDetails {
    /// Original source `company_num` field.
    #[serde(deserialize_with = "code")]
    pub company_num: String,
    /// Original source `company_name` field.
    pub company_name: String,
    /// Original source `company_status` field.
    pub company_status: Option<String>,
    /// Original source `company_type` field.
    pub company_type: Option<String>,
    /// Original source `company_reg_date` field.
    pub company_reg_date: Option<String>,
    /// Original source `last_ar_date` field.
    pub last_ar_date: Option<String>,
    /// Original source `company_address_1` field.
    pub company_address_1: Option<String>,
    /// Original source `company_address_2` field.
    pub company_address_2: Option<String>,
    /// Original source `company_address_3` field.
    pub company_address_3: Option<String>,
    /// Original source `company_address_4` field.
    pub company_address_4: Option<String>,
    /// Original source `comp_dissolved_date` field.
    pub comp_dissolved_date: Option<String>,
    /// Original source `nard` field.
    pub nard: Option<String>,
    /// Original source `last_accounts_date` field.
    pub last_accounts_date: Option<String>,
    /// Original source `company_status_date` field.
    pub company_status_date: Option<String>,
    /// Original source `eircode` field.
    pub eircode: Option<String>,
    /// Original source `company_name_eff_date` field.
    pub company_name_eff_date: Option<String>,
    /// Original source `company_type_eff_date` field.
    pub company_type_eff_date: Option<String>,
    /// Original source `princ_object_code` field.
    pub princ_object_code: Option<String>,
    /// Original source `company_status_code` field.
    pub company_status_code: Option<u64>,
    /// Original source `company_type_code` field.
    pub company_type_code: Option<u64>,
    /// Original source `nace_v2_code` field.
    pub nace_v2_code: Option<u64>,
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
/// Exact matched published CRO company.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct IeCompanyLookupResult {
    /// Full requested company number.
    pub tax_id: super::IeBusinessId,
    /// Original company name.
    pub company_name: String,
    /// Native source fields.
    pub details: IeCompanyDetails,
}
