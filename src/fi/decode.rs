use super::{FiBusinessId, FiCompanyLookupResult, FiPrhDetails};
use crate::RegistryFailure;
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Wire {
    total_results: usize,
    companies: Vec<FiPrhDetails>,
}
pub(super) fn decode(
    bytes: &[u8],
    id: &FiBusinessId,
) -> Result<FiCompanyLookupResult, RegistryFailure> {
    let mut wire: Wire =
        serde_json::from_slice(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?;
    if wire.total_results != 1 || wire.companies.len() != 1 {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    let details = wire.companies.remove(0);
    if details.business_id.value != id.as_str() {
        return Err(RegistryFailure::MismatchedResponse);
    }
    let names: Vec<_> = details
        .names
        .iter()
        .filter(|n| n.r#type == "1" && n.version == 1)
        .collect();
    if names.len() != 1 || names[0].name.trim().is_empty() {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    let company_name = names[0].name.clone();
    let forms: Vec<_> = details
        .company_forms
        .iter()
        .filter(|f| f.version == 1)
        .collect();
    let legal_form = if forms.len() == 1 {
        let labels: Vec<_> = forms[0]
            .descriptions
            .iter()
            .filter(|d| d.language_code == "1" && !d.description.trim().is_empty())
            .collect();
        (labels.len() == 1).then(|| labels[0].description.clone())
    } else {
        None
    };
    Ok(FiCompanyLookupResult {
        tax_id: id.clone(),
        company_name,
        legal_form,
        native_registry_status: Some(details.trade_register_status.clone()),
        details,
    })
}
