use super::{NoBusinessId, NoCompanyDetails, NoCompanyLookupResult};
use crate::RegistryFailure;
pub(super) fn decode(
    bytes: &[u8],
    id: &NoBusinessId,
) -> Result<NoCompanyLookupResult, RegistryFailure> {
    let details: NoCompanyDetails =
        serde_json::from_slice(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?;
    if details.organisasjonsnummer != id.as_str() {
        return Err(RegistryFailure::MismatchedResponse);
    }
    if details.navn.trim().is_empty() {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    Ok(NoCompanyLookupResult {
        tax_id: id.clone(),
        company_name: details.navn.clone(),
        details,
    })
}
