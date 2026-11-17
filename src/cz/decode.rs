use super::{CzBusinessId, CzCompanyLookupResult};
use crate::RegistryFailure;
pub(super) fn decode(
    bytes: &[u8],
    id: &CzBusinessId,
) -> Result<Option<CzCompanyLookupResult>, RegistryFailure> {
    let result: CzCompanyLookupResult = serde_json::from_slice::<super::model::Wire>(bytes)
        .map_err(|_| RegistryFailure::UnexpectedResponse)?
        .into();
    if result.tax_id != *id {
        return Err(RegistryFailure::MismatchedResponse);
    }
    if result.company_name.trim().is_empty() {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    Ok(Some(result))
}

/// Requires the documented error discriminator; transport decides whether HTTP 404 applies.
pub(super) fn is_definitive_miss(bytes: &[u8]) -> bool {
    #[derive(serde::Deserialize)]
    struct ErrorEnvelope {
        kod: String,
        popis: String,
    }
    serde_json::from_slice::<ErrorEnvelope>(bytes)
        .is_ok_and(|e| e.kod == "NENALEZENO" && !e.popis.trim().is_empty())
}
