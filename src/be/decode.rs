use super::{BeBusinessId, BeCompanyLookupResult, BeNbbDetails, BeRegistryAddress};
use crate::RegistryFailure;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Wire {
    cbe: String,
    name: String,
    legal_form: Option<String>,
    legal_situation: Option<String>,
    #[serde(flatten)]
    address: BeRegistryAddress,
    #[serde(flatten)]
    details: BeNbbDetails,
}

pub(super) fn decode(
    bytes: &[u8],
    id: &BeBusinessId,
) -> Result<BeCompanyLookupResult, RegistryFailure> {
    let v: Wire = serde_json::from_slice(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?;
    if BeBusinessId::parse(&v.cbe).ok().as_ref() != Some(id) {
        return Err(RegistryFailure::MismatchedResponse);
    }
    if v.name.trim().is_empty() {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    Ok(BeCompanyLookupResult {
        tax_id: id.clone(),
        company_name: v.name,
        legal_form: v.legal_form,
        native_registry_status: v.legal_situation,
        registered_office: v.address,
        details: v.details,
    })
}
