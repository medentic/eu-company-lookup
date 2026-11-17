use super::*;
use crate::RegistryFailure;
pub(super) const RESOURCE: &str = "3fef41bc-b8f4-4b10-8434-ce51c29b1bba";
pub(super) fn decode(
    bytes: &[u8],
    id: &IeBusinessId,
) -> Result<IeCompanyLookupResult, RegistryFailure> {
    let d: IeCompanyDetails = crate::registry_transport::ckan::single_record(bytes, RESOURCE)?;
    if d.company_num != id.as_str() {
        return Err(RegistryFailure::MismatchedResponse);
    }
    if d.company_name.trim().is_empty() {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    Ok(IeCompanyLookupResult {
        tax_id: id.clone(),
        company_name: d.company_name.clone(),
        details: d,
    })
}
