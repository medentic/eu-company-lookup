use serde::{Deserialize, Serialize};
/// Selected original registry dataset fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LvCompanyDetails {
    /// Complete, identity-checked observations from the three lifecycle datasets.
    /// Populated by the adapter, never trusted from the base dataset response.
    pub lifecycle: Option<LvLifecycleDetails>,
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

/// Current lifecycle datasets observed alongside the base registration record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LvLifecycleDetails {
    /// Complete current liquidation entries for this registration number.
    pub liquidations: Vec<LvLiquidation>,
    /// Complete current suspension/prohibition entries for this registration number.
    pub suspensions: Vec<LvSuspension>,
    /// Complete insolvency entries, including historical entries.
    pub insolvencies: Vec<LvInsolvency>,
}
/// Native current liquidation entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LvLiquidation {
    /// Source entry identifier.
    #[serde(deserialize_with = "code")]
    pub id: String,
    /// Full identity echoed by the supplemental dataset.
    #[serde(deserialize_with = "code")]
    pub legal_entity_registration_number: String,
    /// Source code.
    pub liquidation_type: Option<String>,
    /// Source label.
    pub liquidation_type_text: Option<String>,
    /// Start date in the source.
    pub date_from: Option<String>,
}
/// Native current suspension or prohibition entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LvSuspension {
    /// Source entry identifier.
    #[serde(deserialize_with = "code")]
    pub id: String,
    /// Full identity echoed by the supplemental dataset.
    #[serde(deserialize_with = "code")]
    pub legal_entity_registration_number: String,
    /// Source code.
    pub suspension_code: Option<String>,
    /// Source label.
    pub suspension_code_text: Option<String>,
    /// Start date in the source.
    pub date_from: Option<String>,
}
/// Selected native insolvency facts; court and personal information are excluded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LvInsolvency {
    /// Source proceeding identifier.
    #[serde(deserialize_with = "code")]
    pub proceeding_id: String,
    /// Full identity echoed by the supplemental dataset.
    #[serde(deserialize_with = "code")]
    pub debtor_registration_number: String,
    /// Source proceeding start.
    pub proceeding_started_on: Option<String>,
    /// Source proceeding end.
    pub proceeding_ended_on: Option<String>,
    /// Source proceeding form.
    pub proceeding_form: Option<String>,
    /// Source proceeding type.
    pub proceeding_type: Option<String>,
    /// Source proceeding subtype.
    pub proceeding_subtype: Option<String>,
}
