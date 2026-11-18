//! Success policy; provider-owned projections remain the only lifecycle mappings.
use crate::{CompanyLookupResult, CompanyStatus, LookupError};

pub(crate) fn ensure_resolved<T>(record: T) -> Result<T, LookupError>
where
    T: Clone + Into<CompanyLookupResult>,
{
    let common: CompanyLookupResult = record.clone().into();
    if common
        .status
        .is_some_and(|status| status != CompanyStatus::Unknown)
    {
        return Ok(record);
    }
    let native = common.registry_details.native_status().map(str::to_owned);
    let reason = if common.status.is_none() {
        "missing_lifecycle"
    } else {
        "unmapped_or_ambiguous_lifecycle"
    };
    Err(super::LookupFailure::UnresolvedData {
        provider: common.source,
        country: common.country,
        native_status: native,
        reason,
    }
    .into())
}
