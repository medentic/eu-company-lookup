//! Current UR lifecycle datasets. Empty results count only after complete filtered responses.
use super::*;
use crate::{CompanyStatus, RegistryFailure};
use serde::Deserialize;

pub(super) const SOURCES: [(&str, &str, &str); 3] = [
    (
        "liquidation",
        "59e7ec49-f1c6-4410-8ee6-e7737ac5eaee",
        "legal_entity_registration_number",
    ),
    (
        "suspension",
        "1a077ff6-4c4e-4cfe-b603-b7e40851e8a9",
        "legal_entity_registration_number",
    ),
    (
        "insolvency",
        "8065ad80-1a4d-4afb-b1d1-d93b9a62b1cc",
        "debtor_registration_number",
    ),
];
#[derive(Deserialize)]
struct Envelope<T> {
    success: bool,
    result: Records<T>,
}
#[derive(Deserialize)]
struct Records<T> {
    resource_id: String,
    total: usize,
    records: Vec<T>,
    filters: serde_json::Value,
}
pub(super) fn records<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
    resource: &str,
    field: &str,
    id: &str,
) -> Result<Vec<T>, RegistryFailure> {
    let envelope: Envelope<T> =
        serde_json::from_slice(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?;
    let r = envelope.result;
    if !envelope.success
        || r.resource_id != resource
        || r.total != r.records.len()
        || r.total > 100
        || r.filters != serde_json::json!({field:id})
    {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    Ok(r.records)
}
pub(super) fn decode(
    liquidations: &[u8],
    suspensions: &[u8],
    insolvencies: &[u8],
    id: &LvBusinessId,
) -> Result<LvLifecycleDetails, RegistryFailure> {
    let data = LvLifecycleDetails {
        liquidations: records(liquidations, SOURCES[0].1, SOURCES[0].2, id.as_str())?,
        suspensions: records(suspensions, SOURCES[1].1, SOURCES[1].2, id.as_str())?,
        insolvencies: records(insolvencies, SOURCES[2].1, SOURCES[2].2, id.as_str())?,
    };
    if data
        .liquidations
        .iter()
        .any(|r| r.legal_entity_registration_number != id.as_str())
        || data
            .suspensions
            .iter()
            .any(|r| r.legal_entity_registration_number != id.as_str())
        || data
            .insolvencies
            .iter()
            .any(|r| r.debtor_registration_number != id.as_str())
    {
        return Err(RegistryFailure::MismatchedResponse);
    }
    Ok(data)
}
fn date(value: &str) -> bool {
    jiff::civil::Date::strptime("%Y-%m-%d", value)
        .is_ok_and(|d| d.strftime("%Y-%m-%d").to_string() == value)
        || jiff::civil::DateTime::strptime("%Y-%m-%dT%H:%M:%S", value)
            .is_ok_and(|d| d.strftime("%Y-%m-%dT%H:%M:%S").to_string() == value)
}
pub(super) fn status(details: &LvCompanyDetails) -> Option<CompanyStatus> {
    use CompanyStatus::*;
    let evidence = details.lifecycle.as_ref()?;
    if details.terminated.is_some()
        || !details.registered.as_deref().is_some_and(date)
        || !details
            .closed
            .as_deref()
            .is_some_and(|s| s.trim().is_empty())
    {
        return Some(Unknown);
    }
    if evidence
        .insolvencies
        .iter()
        .any(|r| !r.proceeding_ended_on.as_deref().is_some_and(date))
    {
        return Some(Unknown); // A proceeding entry is not necessarily a bankruptcy order.
    }
    if evidence.liquidations.iter().any(|r| {
        r.liquidation_type.as_deref() != Some("LIQUIDATION")
            || !r.date_from.as_deref().is_some_and(date)
    }) || evidence.suspensions.iter().any(|r| {
        r.suspension_code.as_deref() != Some("ECONOMIC_ACTIVITY_SUSPENDED")
            || !r.date_from.as_deref().is_some_and(date)
    }) {
        return Some(Unknown);
    }
    // Economic-activity suspension can coexist with liquidation; preserve both
    // native entries and project the more specific winding-up lifecycle.
    Some(
        match (
            !evidence.liquidations.is_empty(),
            !evidence.suspensions.is_empty(),
        ) {
            (true, _) => Liquidation,
            (false, true) => Inactive,
            (false, false) => Active,
        },
    )
}
