//! Public RPO lifecycle supplement for matched RÚZ records lacking dissolution evidence.
use super::{SkBusinessId, SkRpoLifecycle, SkRpoValue};
use crate::{CompanyStatus, RegistryFailure};
use serde::Deserialize;
#[derive(Deserialize)]
struct Search {
    results: Vec<SkRpoLifecycle>,
}
fn identity(record: &SkRpoLifecycle, id: &SkBusinessId) -> bool {
    record
        .identifiers
        .iter()
        .filter(|x| x.valid_to.is_none())
        .any(|x| x.value == id.as_str())
        && record
            .identifiers
            .iter()
            .filter(|x| x.valid_to.is_none())
            .all(|x| x.value == id.as_str())
}
pub(super) fn select(bytes: &[u8], id: &SkBusinessId) -> Result<Option<u64>, RegistryFailure> {
    let search: Search =
        serde_json::from_slice(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?;
    if search.results.is_empty() {
        return Ok(None);
    }
    if search.results.len() != 1
        || !identity(&search.results[0], id)
        || search.results[0].termination.is_some()
    {
        return Err(RegistryFailure::MismatchedResponse);
    }
    Ok(Some(search.results[0].id))
}
pub(super) fn decode(
    bytes: &[u8],
    id: &SkBusinessId,
    internal: u64,
) -> Result<SkRpoLifecycle, RegistryFailure> {
    let record: SkRpoLifecycle =
        serde_json::from_slice(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?;
    if record.id != internal || !identity(&record, id) {
        return Err(RegistryFailure::MismatchedResponse);
    }
    if record.termination.is_some() {
        return Err(RegistryFailure::MismatchedResponse);
    }
    Ok(record)
}
fn date(s: &str) -> bool {
    jiff::civil::Date::strptime("%Y-%m-%d", s)
        .is_ok_and(|d| d.strftime("%Y-%m-%d").to_string() == s)
}
fn current(value: &SkRpoValue) -> bool {
    value.valid_to.is_none() && value.valid_from.as_deref().is_none_or(date)
}
pub(super) fn status(record: &SkRpoLifecycle) -> CompanyStatus {
    use CompanyStatus::*;
    if record.termination.is_some()
        || !record.establishment.as_deref().is_some_and(date)
        || !record.identifiers.iter().all(current)
    {
        return Unknown;
    }
    match record.legal_statuses.as_deref().unwrap_or_default() {
        // The preceding unique exact-IČO search explicitly requested onlyActive=true.
        [] => Active,
        [entry]
            if entry.valid_to.is_none()
                && entry.valid_from.as_deref().is_none_or(date)
                && entry.value.codelist_code == "CL010108" =>
        {
            match entry.value.value.trim() {
                "V likvidácii" => Liquidation,
                "V konkurze" => Bankruptcy,
                "V reštrukturalizácii" => Restructuring,
                _ => Unknown,
            }
        }
        _ => Unknown,
    }
}
