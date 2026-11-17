use super::{FrBusinessId, FrCompanyDetails, FrCompanyLookupResult};
use crate::RegistryError;
#[derive(serde::Deserialize)]
struct Wire {
    total_results: usize,
    results: Vec<FrCompanyDetails>,
}
pub(super) fn decode(
    bytes: &[u8],
    id: &FrBusinessId,
) -> Result<FrCompanyLookupResult, RegistryError> {
    let mut wire: Wire =
        serde_json::from_slice(bytes).map_err(|_| RegistryError::UnexpectedResponse)?;
    if wire.total_results != 1 || wire.results.len() != 1 {
        return Err(RegistryError::UnexpectedResponse);
    }
    let details = wire.results.remove(0);
    if details.siren != id.as_str() {
        return Err(RegistryError::MismatchedResponse);
    }
    if details.nom_complet.trim().is_empty() {
        return Err(RegistryError::UnexpectedResponse);
    }
    if let Some(seat) = &details.siege {
        if seat.est_siege != Some(true) {
            return Err(RegistryError::UnexpectedResponse);
        }
        if let Some(siret) = &seat.siret
            && (siret.len() != 14
                || !siret.bytes().all(|b| b.is_ascii_digit())
                || !siret.starts_with(id.as_str()))
        {
            return Err(RegistryError::MismatchedResponse);
        }
    }
    Ok(FrCompanyLookupResult {
        tax_id: id.clone(),
        company_name: details.nom_complet.clone(),
        details,
    })
}
