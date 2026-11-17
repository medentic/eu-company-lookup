use super::*;
use crate::RegistryFailure;
#[derive(serde::Deserialize)]
struct Selection {
    id: Vec<u64>,
    #[serde(rename = "existujeDalsieId")]
    more: bool,
}
pub(super) fn select(bytes: &[u8]) -> Result<u64, RegistryFailure> {
    let v: Selection =
        serde_json::from_slice(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?;
    if v.more || v.id.len() != 1 {
        return Err(RegistryFailure::MismatchedResponse);
    }
    Ok(v.id[0])
}
pub(super) fn decode(
    bytes: &[u8],
    id: &SkBusinessId,
    internal: u64,
) -> Result<SkCompanyLookupResult, RegistryFailure> {
    let d: SkCompanyDetails =
        serde_json::from_slice(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?;
    if d.id != internal || d.ico != id.as_str() {
        return Err(RegistryFailure::MismatchedResponse);
    }
    if d.nazov_uj.trim().is_empty() || d.stav.is_some() {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    Ok(SkCompanyLookupResult {
        tax_id: id.clone(),
        company_name: d.nazov_uj.clone(),
        details: d,
    })
}
