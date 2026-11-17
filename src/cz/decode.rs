use super::{CzBusinessId, CzCompanyLookupResult};
use crate::RegistryError;
pub(super) fn decode(
    bytes: &[u8],
    id: &CzBusinessId,
) -> Result<Option<CzCompanyLookupResult>, RegistryError> {
    let result: CzCompanyLookupResult = serde_json::from_slice::<super::model::Wire>(bytes)
        .map_err(|_| RegistryError::UnexpectedResponse)?
        .into();
    if result.tax_id != *id {
        return Err(RegistryError::MismatchedResponse);
    }
    if result.company_name.trim().is_empty() {
        return Err(RegistryError::UnexpectedResponse);
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
