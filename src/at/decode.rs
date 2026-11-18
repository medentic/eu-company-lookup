use super::*;
use crate::RegistryFailure;
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
    status: Option<String>,
}
pub(super) fn select(bytes: &[u8], id: &AtBusinessId) -> Result<String, RegistryFailure> {
    let v: Search =
        serde_json::from_slice(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?;
    // Accept retained active-only captures as well as the explicit all-status search.
    // HISTORIC names may accompany the current registered/deleted entry.
    // Only explicitly historical alternatives can be excluded.
    if !matches!(v.state.as_str(), "ACTIVE" | "ALL") || v.num_results != v.companies.len() {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    let mut candidates: Vec<_> = v
        .companies
        .into_iter()
        .filter(|h| AtBusinessId::parse(&h.fnr).ok().as_ref() == Some(id))
        .collect();
    if candidates.len() > 1 {
        candidates.retain(|h| h.status.as_deref() != Some("HISTORIC"));
    }
    let mut matched = candidates.into_iter();
    let h = matched.next().ok_or(RegistryFailure::UnexpectedResponse)?;
    if matched.next().is_some() {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    // Only a source-returned, identity-bound route suffix can select a detail.
    let (fnr, revision) =
        h.id.split_once('_')
            .ok_or(RegistryFailure::UnexpectedResponse)?;
    if AtBusinessId::parse(fnr).ok().as_ref() != Some(id)
        || revision.is_empty()
        || !revision.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(RegistryFailure::MismatchedResponse);
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
) -> Result<AtCompanyLookupResult, RegistryFailure> {
    let v: Detail =
        serde_json::from_slice(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?;
    if AtBusinessId::parse(&v.fnr).ok().as_ref() != Some(id) || v.id != record {
        return Err(RegistryFailure::MismatchedResponse);
    }
    if v.name.trim().is_empty() || v.status.trim().is_empty() {
        return Err(RegistryFailure::UnexpectedResponse);
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
