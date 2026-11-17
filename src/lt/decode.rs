use super::*;
use crate::RegistryError;
pub(super) const SOURCE_TYPE: &str = "datasets/gov/rc/jar/iregistruoti/JuridinisAsmuo";
#[derive(serde::Deserialize)]
struct Wire {
    #[serde(rename = "_data")]
    data: Vec<LtCompanyDetails>,
}
pub(super) fn decode(
    bytes: &[u8],
    id: &LtBusinessId,
) -> Result<LtCompanyLookupResult, RegistryError> {
    let v: Wire = serde_json::from_slice(bytes).map_err(|_| RegistryError::UnexpectedResponse)?;
    if v.data.len() != 1 {
        return Err(RegistryError::MismatchedResponse);
    }
    let d = v.data.into_iter().next().unwrap();
    if d.source_type != SOURCE_TYPE {
        return Err(RegistryError::UnexpectedResponse);
    }
    if d.ja_kodas != id.as_str() {
        return Err(RegistryError::MismatchedResponse);
    }
    if d.ja_pavadinimas.trim().is_empty() || d.source_id.trim().is_empty() {
        return Err(RegistryError::UnexpectedResponse);
    }
    Ok(LtCompanyLookupResult {
        tax_id: id.clone(),
        company_name: d.ja_pavadinimas.clone(),
        details: d,
    })
}
