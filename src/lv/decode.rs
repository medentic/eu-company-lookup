use super::*;
use crate::RegistryFailure;
pub(super) const RESOURCE: &str = "25e80bf3-f107-4ab4-89ef-251b5b9374e9";
pub(super) fn decode(
    bytes: &[u8],
    id: &LvBusinessId,
) -> Result<LvCompanyLookupResult, RegistryFailure> {
    let d: LvCompanyDetails = crate::registry_transport::ckan::single_record(bytes, RESOURCE)?;
    if d.regcode != id.as_str() {
        return Err(RegistryFailure::MismatchedResponse);
    }
    if d.name.trim().is_empty() {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    Ok(LvCompanyLookupResult {
        tax_id: id.clone(),
        company_name: d.name.clone(),
        details: d,
    })
}
