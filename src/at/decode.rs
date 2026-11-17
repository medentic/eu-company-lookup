use super::*;
use crate::RegistryError;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Search {
    num_results: usize,
    companies: Vec<Hit>,
    state: String,
}
#[derive(Deserialize)]
struct Hit {
    id: String,
    fnr: String,
}
pub(super) fn select(bytes: &[u8], id: &AtBusinessId) -> Result<String, RegistryError> {
    let v: Search = serde_json::from_slice(bytes).map_err(|_| RegistryError::UnexpectedResponse)?;
    if v.state != "ACTIVE" || v.num_results != v.companies.len() {
        return Err(RegistryError::UnexpectedResponse);
    }
    let mut matched = v
        .companies
        .into_iter()
        .filter(|h| AtBusinessId::parse(&h.fnr).ok().as_ref() == Some(id));
    let h = matched.next().ok_or(RegistryError::UnexpectedResponse)?;
    if matched.next().is_some() {
        return Err(RegistryError::UnexpectedResponse);
    }
    // Only a source-returned, identity-bound route suffix can select a detail.
    let (fnr, revision) =
        h.id.split_once('_')
            .ok_or(RegistryError::UnexpectedResponse)?;
    if AtBusinessId::parse(fnr).ok().as_ref() != Some(id)
        || revision.is_empty()
        || !revision.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(RegistryError::MismatchedResponse);
    }
    Ok(h.id)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Detail {
    id: String,
    fnr: String,
    name: String,
    status: String,
    domicile: Option<String>,
    date: Option<String>,
    legal_form: Option<AtLegalForm>,
    address: Option<AtRegistryAddress>,
}
pub(super) fn detail(
    bytes: &[u8],
    id: &AtBusinessId,
    record: &str,
) -> Result<AtCompanyLookupResult, RegistryError> {
    let v: Detail = serde_json::from_slice(bytes).map_err(|_| RegistryError::UnexpectedResponse)?;
    if AtBusinessId::parse(&v.fnr).ok().as_ref() != Some(id) || v.id != record {
        return Err(RegistryError::MismatchedResponse);
    }
    if v.name.trim().is_empty() || v.status.trim().is_empty() {
        return Err(RegistryError::UnexpectedResponse);
    }
    Ok(AtCompanyLookupResult {
        tax_id: id.clone(),
        record_id: v.id,
        company_name: v.name,
        native_registry_status: v.status,
        domicile: v.domicile,
        source_date: v.date,
        legal_form: v.legal_form,
        registered_office: v.address,
    })
}
