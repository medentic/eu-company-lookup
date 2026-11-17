use super::{GrBusinessId, GrCompanyLookupResult, GrGemiDetails};
use crate::RegistryError;
use serde::Deserialize;

#[derive(Deserialize)]
struct Envelope {
    #[serde(rename = "companyInfo")]
    info: Info,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Info {
    error_code: Option<serde_json::Value>,
    error_message: Option<serde_json::Value>,
    payload: Option<Payload>,
}
#[derive(Deserialize)]
struct Payload {
    company: Company,
}
#[derive(Deserialize)]
struct LegalType {
    id: Option<String>,
    desc: Option<String>,
}
#[derive(Deserialize)]
struct Status {
    id: Option<String>,
    status: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Company {
    id: String,
    name: String,
    namei18n: Option<String>,
    afm: Option<String>,
    company_status: Option<Status>,
    legal_type: Option<LegalType>,
    date_start: Option<String>,
    date_gemi_registered: Option<String>,
    #[serde(rename = "company_address")]
    address: Option<String>,
    #[serde(rename = "company_address_map")]
    map_address: Option<String>,
    #[serde(rename = "isABranch")]
    is_branch: Option<bool>,
    is_foreign_branch: Option<bool>,
}

pub(super) fn decode(
    bytes: &[u8],
    id: &GrBusinessId,
) -> Result<GrCompanyLookupResult, RegistryError> {
    let e: Envelope =
        serde_json::from_slice(bytes).map_err(|_| RegistryError::UnexpectedResponse)?;
    if e.info.error_code.is_some() || e.info.error_message.is_some() {
        return Err(RegistryError::UnexpectedResponse);
    }
    let v = e
        .info
        .payload
        .ok_or(RegistryError::UnexpectedResponse)?
        .company;
    if GrBusinessId::parse(&v.id).ok().as_ref() != Some(id) {
        return Err(RegistryError::MismatchedResponse);
    }
    if v.name.trim().is_empty() {
        return Err(RegistryError::UnexpectedResponse);
    }
    Ok(GrCompanyLookupResult {
        tax_id: id.clone(),
        company_name: v.name,
        foreign_name: v.namei18n,
        legal_form: v.legal_type.as_ref().and_then(|l| l.desc.clone()),
        native_registry_status: v.company_status.as_ref().and_then(|s| s.status.clone()),
        full_address_by_registry: v.address,
        details: GrGemiDetails {
            tax_identifier: v.afm,
            legal_type_code: v.legal_type.and_then(|l| l.id),
            status_code: v.company_status.and_then(|s| s.id),
            start_date: v.date_start,
            gemi_registration_date: v.date_gemi_registered,
            map_address: v.map_address,
            is_branch: v.is_branch,
            is_foreign_branch: v.is_foreign_branch,
        },
    })
}
