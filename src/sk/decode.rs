use super::*;
use crate::RegistryError;
#[derive(serde::Deserialize)]
struct Selection {
    id: Vec<u64>,
    #[serde(rename = "existujeDalsieId")]
    more: bool,
}
pub(super) fn select(bytes: &[u8]) -> Result<u64, RegistryError> {
    let v: Selection =
        serde_json::from_slice(bytes).map_err(|_| RegistryError::UnexpectedResponse)?;
    if v.more || v.id.len() != 1 {
        return Err(RegistryError::MismatchedResponse);
    }
    Ok(v.id[0])
}
pub(super) fn decode(
    bytes: &[u8],
    id: &SkBusinessId,
    internal: u64,
) -> Result<SkCompanyLookupResult, RegistryError> {
    let d: SkCompanyDetails =
        serde_json::from_slice(bytes).map_err(|_| RegistryError::UnexpectedResponse)?;
    if d.id != internal || d.ico != id.as_str() {
        return Err(RegistryError::MismatchedResponse);
    }
    if d.nazov_uj.trim().is_empty() || d.stav.is_some() {
        return Err(RegistryError::UnexpectedResponse);
    }
    Ok(SkCompanyLookupResult {
        tax_id: id.clone(),
        company_name: d.nazov_uj.clone(),
        details: d,
    })
}
