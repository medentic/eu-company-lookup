use super::{EeBusinessId, EeCompanyDetails, EeCompanyLookupResult};
use crate::RegistryFailure;
#[derive(serde::Deserialize)]
struct Wire {
    status: String,
    data: Vec<EeCompanyDetails>,
}
pub(super) fn decode(
    bytes: &[u8],
    id: &EeBusinessId,
) -> Result<EeCompanyLookupResult, RegistryFailure> {
    let wire: Wire =
        serde_json::from_slice(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?;
    if wire.status != "OK" {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    let mut matches: Vec<_> = wire
        .data
        .into_iter()
        .filter(|v| v.reg_code == id.as_str())
        .collect();
    if matches.len() != 1 {
        return Err(RegistryFailure::MismatchedResponse);
    }
    let details = matches.remove(0);
    if details.name.trim().is_empty() {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    Ok(EeCompanyLookupResult {
        tax_id: id.clone(),
        company_name: details.name.clone(),
        details,
    })
}
