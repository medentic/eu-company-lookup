//! CKAN wire mechanics shared by anonymous dataset adapters.
//! Dataset identity, record identity and field meanings remain provider-owned.
use crate::RegistryError;
use serde::{Deserialize, de::DeserializeOwned};

pub(crate) fn query(
    endpoint: &str,
    resource: &str,
    field: &str,
    value: &str,
) -> Result<reqwest::Url, RegistryError> {
    let mut url = reqwest::Url::parse(endpoint).map_err(|_| RegistryError::UnexpectedResponse)?;
    url.query_pairs_mut()
        .append_pair("resource_id", resource)
        .append_pair("limit", "2")
        .append_pair("filters", &serde_json::json!({field: value}).to_string());
    Ok(url)
}

#[derive(Deserialize)]
struct Envelope<T> {
    success: bool,
    result: Records<T>,
}
#[derive(Deserialize)]
struct Records<T> {
    resource_id: String,
    total: u64,
    records: Vec<T>,
}

pub(crate) fn single_record<T: DeserializeOwned>(
    bytes: &[u8],
    resource: &str,
) -> Result<T, RegistryError> {
    let envelope: Envelope<T> =
        serde_json::from_slice(bytes).map_err(|_| RegistryError::UnexpectedResponse)?;
    let mut result = envelope.result;
    if !envelope.success || result.resource_id != resource {
        return Err(RegistryError::UnexpectedResponse);
    }
    if result.total != 1 || result.records.len() != 1 {
        return Err(RegistryError::MismatchedResponse);
    }
    result
        .records
        .pop()
        .ok_or(RegistryError::MismatchedResponse)
}
