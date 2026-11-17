use super::*;
use crate::RegistryFailure;
pub(super) const SOURCE_TYPE: &str = "datasets/gov/rc/jar/iregistruoti/JuridinisAsmuo";
#[derive(serde::Deserialize)]
struct Wire {
    #[serde(rename = "_data")]
    data: Vec<LtCompanyDetails>,
}
pub(super) fn decode(
    bytes: &[u8],
    id: &LtBusinessId,
) -> Result<LtCompanyLookupResult, RegistryFailure> {
    let v: Wire = serde_json::from_slice(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?;
    if v.data.len() != 1 {
        return Err(RegistryFailure::MismatchedResponse);
    }
    let d = v.data.into_iter().next().unwrap();
    if d.source_type != SOURCE_TYPE {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    if d.ja_kodas != id.as_str() {
        return Err(RegistryFailure::MismatchedResponse);
    }
    if d.ja_pavadinimas.trim().is_empty() || d.source_id.trim().is_empty() {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    Ok(LtCompanyLookupResult {
        tax_id: id.clone(),
        company_name: d.ja_pavadinimas.clone(),
        details: d,
    })
}
