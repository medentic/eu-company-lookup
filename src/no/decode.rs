use super::{NoBusinessId, NoCompanyDetails, NoCompanyLookupResult};
use crate::RegistryError;
pub(super) fn decode(
    bytes: &[u8],
    id: &NoBusinessId,
) -> Result<NoCompanyLookupResult, RegistryError> {
    let details: NoCompanyDetails =
        serde_json::from_slice(bytes).map_err(|_| RegistryError::UnexpectedResponse)?;
    if details.organisasjonsnummer != id.as_str() {
        return Err(RegistryError::MismatchedResponse);
    }
    if details.navn.trim().is_empty() {
        return Err(RegistryError::UnexpectedResponse);
    }
    Ok(NoCompanyLookupResult {
        tax_id: id.clone(),
        company_name: details.navn.clone(),
        details,
    })
}
