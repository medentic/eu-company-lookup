use super::{EeBusinessId, EeCompanyDetails, EeCompanyLookupResult};
use crate::RegistryError;
#[derive(serde::Deserialize)]
struct Wire {
    status: String,
    data: Vec<EeCompanyDetails>,
}
pub(super) fn decode(
    bytes: &[u8],
    id: &EeBusinessId,
) -> Result<EeCompanyLookupResult, RegistryError> {
    let wire: Wire =
        serde_json::from_slice(bytes).map_err(|_| RegistryError::UnexpectedResponse)?;
    if wire.status != "OK" {
        return Err(RegistryError::UnexpectedResponse);
    }
    let mut matches: Vec<_> = wire
        .data
        .into_iter()
        .filter(|v| v.reg_code == id.as_str())
        .collect();
    if matches.len() != 1 {
        return Err(RegistryError::MismatchedResponse);
    }
    let details = matches.remove(0);
    if details.name.trim().is_empty() {
        return Err(RegistryError::UnexpectedResponse);
    }
    Ok(EeCompanyLookupResult {
        tax_id: id.clone(),
        company_name: details.name.clone(),
        details,
    })
}
