use super::*;
use crate::RegistryError;
pub(super) const RESOURCE: &str = "3fef41bc-b8f4-4b10-8434-ce51c29b1bba";
pub(super) fn decode(
    bytes: &[u8],
    id: &IeBusinessId,
) -> Result<IeCompanyLookupResult, RegistryError> {
    let d: IeCompanyDetails = crate::registry_transport::ckan::single_record(bytes, RESOURCE)?;
    if d.company_num != id.as_str() {
        return Err(RegistryError::MismatchedResponse);
    }
    if d.company_name.trim().is_empty() {
        return Err(RegistryError::UnexpectedResponse);
    }
    Ok(IeCompanyLookupResult {
        tax_id: id.clone(),
        company_name: d.company_name.clone(),
        details: d,
    })
}
