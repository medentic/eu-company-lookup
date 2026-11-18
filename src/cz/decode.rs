use super::{CzBusinessId, CzCompanyLookupResult};
use crate::RegistryFailure;
pub(super) fn decode(
    bytes: &[u8],
    id: &CzBusinessId,
) -> Result<Option<CzCompanyLookupResult>, RegistryFailure> {
    let result: CzCompanyLookupResult = serde_json::from_slice::<super::model::Wire>(bytes)
        .map_err(|_| RegistryFailure::UnexpectedResponse)?
        .into();
    if result.tax_id != *id {
        return Err(RegistryFailure::MismatchedResponse);
    }
    if result.company_name.trim().is_empty() {
        return Err(RegistryFailure::UnexpectedResponse);
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

/// Supplement only an aggregate whose non-primary ROS explicitly reports cessation.
pub(super) fn needs_ros_closure(value: &CzCompanyLookupResult) -> bool {
    value.ceased_date.is_none()
        && value.primary_source.as_deref() != Some("ros")
        && value
            .source_registrations
            .as_ref()
            .is_some_and(|r| r.stav_zdroje_ros.as_deref() == Some("ZANIKLY"))
}

pub(super) fn ros_closure(
    bytes: &[u8],
    id: &CzBusinessId,
) -> Result<Option<super::CzRosClosure>, RegistryFailure> {
    #[derive(serde::Deserialize)]
    struct Response {
        #[serde(rename = "icoId")]
        ico_id: String,
        zaznamy: Vec<Record>,
    }
    #[derive(serde::Deserialize)]
    struct Record {
        ico: String,
        #[serde(rename = "datumZaniku")]
        ceased_date: Option<String>,
        #[serde(rename = "stavSubjektu")]
        state: Option<String>,
    }
    let response: Response =
        serde_json::from_slice(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?;
    if response.ico_id != id.as_str() || response.zaznamy.iter().any(|r| r.ico != id.as_str()) {
        return Err(RegistryFailure::MismatchedResponse);
    }
    let [record] = response.zaznamy.as_slice() else {
        return Ok(None);
    };
    let Some(date) = record.ceased_date.as_deref() else {
        return Ok(None);
    };
    if record.state.as_deref() != Some("ZANIKLY")
        || !jiff::civil::Date::strptime("%Y-%m-%d", date)
            .is_ok_and(|d| d.strftime("%Y-%m-%d").to_string() == date)
    {
        return Ok(None);
    }
    Ok(Some(super::CzRosClosure {
        ico: record.ico.clone(),
        ceased_date: date.into(),
        subject_state: "ZANIKLY".into(),
    }))
}
