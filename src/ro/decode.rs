use super::{RoBusinessId, RoCompanyLookupResult, RoRegistryAddress, RoTaxStatus};
use crate::{Provider, RegistryError};
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{Map, Value};

type Object = Map<String, Value>;

#[derive(Deserialize)]
struct Envelope {
    #[serde(default, deserialize_with = "present")]
    cod: Option<u16>,
    #[serde(default, deserialize_with = "present")]
    message: Option<String>,
    found: Vec<Value>,
    #[serde(rename = "notFound")]
    not_found: Vec<Value>,
}

// Missing status metadata is observed in live v9; an explicit null is not success.
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

// ANAF transports CUI as a number. Compare its decimal value without truncation,
// while retaining the original requested spelling in the public result.
fn identity(value: &Value, id: &RoBusinessId) -> bool {
    let actual = match value {
        Value::String(s) if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) => {
            s.parse::<u64>().ok()
        }
        Value::Number(n) => n.as_u64(),
        _ => None,
    };
    actual == id.as_str().parse::<u64>().ok()
}
fn object(value: &Value) -> Result<&Object, RegistryError> {
    value.as_object().ok_or(RegistryError::UnexpectedResponse)
}
fn text(fields: &Object, key: &str) -> Result<Option<String>, RegistryError> {
    match fields.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok((!s.trim().is_empty()).then(|| s.clone())),
        _ => Err(RegistryError::UnexpectedResponse),
    }
}
fn flag(fields: &Object, key: &str) -> Result<Option<bool>, RegistryError> {
    match fields.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        _ => Err(RegistryError::UnexpectedResponse),
    }
}
fn section_flag(fields: &Object, section: &str, key: &str) -> Result<Option<bool>, RegistryError> {
    match fields.get(section) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => flag(object(value)?, key),
    }
}
fn address(
    fields: &Object,
    section: &str,
    prefix: &str,
) -> Result<Option<RoRegistryAddress>, RegistryError> {
    let fields = match fields.get(section) {
        None | Some(Value::Null) => return Ok(None),
        Some(value) => object(value)?,
    };
    let get = |suffix: &str| text(fields, &format!("{prefix}{suffix}"));
    Ok(Some(RoRegistryAddress {
        street: get("denumire_Strada")?,
        street_number: get("numar_Strada")?,
        locality: get("denumire_Localitate")?,
        county: get("denumire_Judet")?,
        country: get("tara")?,
        postcode: get("cod_Postal")?,
        details: get("detalii_Adresa")?,
    }))
}

pub(super) fn decode(
    body: &[u8],
    id: &RoBusinessId,
    date: NaiveDate,
) -> Result<Option<RoCompanyLookupResult>, RegistryError> {
    let response: Envelope =
        serde_json::from_slice(body).map_err(|_| RegistryError::UnexpectedResponse)?;
    if response.cod.is_some_and(|code| code != 200)
        || response
            .message
            .as_deref()
            .is_some_and(|message| message != "SUCCESS")
    {
        return Err(RegistryError::UnexpectedResponse);
    }
    if response.found.is_empty() && response.not_found.len() == 1 {
        return if identity(&response.not_found[0], id) {
            Ok(None)
        } else {
            Err(RegistryError::MismatchedResponse)
        };
    }
    if response.found.len() != 1 || !response.not_found.is_empty() {
        return Err(RegistryError::UnexpectedResponse);
    }
    let fields = object(&response.found[0])?;
    let general = object(
        fields
            .get("date_generale")
            .ok_or(RegistryError::UnexpectedResponse)?,
    )?;
    if !identity(
        general
            .get("cui")
            .ok_or(RegistryError::UnexpectedResponse)?,
        id,
    ) {
        return Err(RegistryError::MismatchedResponse);
    }
    if text(general, "data")?.as_deref() != Some(date.to_string().as_str()) {
        return Err(RegistryError::MismatchedResponse);
    }
    let registration_status = text(general, "stare_inregistrare")?;
    // No verified ANAF lifecycle mapping yet. Fiscal inactivity is not closure.
    let status = registration_status
        .as_ref()
        .map(|_| crate::CompanyStatus::Unknown);
    Ok(Some(RoCompanyLookupResult {
        tax_id: id.clone(),
        source: Provider::RomanianAnaf,
        request_date: date,
        checked_at: Utc::now(),
        company_name: text(general, "denumire")?.ok_or(RegistryError::UnexpectedResponse)?,
        trade_register_number: text(general, "nrRegCom")?,
        status,
        native_registry_status: registration_status.clone(),
        registration_status,
        registration_date: text(general, "data_inregistrare")?,
        legal_form: text(general, "forma_juridica")?,
        activity_code: text(general, "cod_CAEN")?,
        full_fiscal_address: text(general, "adresa")?,
        registered_office: address(fields, "adresa_sediu_social", "s")?,
        fiscal_address: address(fields, "adresa_domiciliu_fiscal", "d")?,
        tax_status: RoTaxStatus {
            vat_registered: section_flag(fields, "inregistrare_scop_Tva", "scpTVA")?,
            vat_on_collection: section_flag(fields, "inregistrare_RTVAI", "statusTvaIncasare")?,
            inactive: section_flag(fields, "stare_inactiv", "statusInactivi")?,
            split_vat: section_flag(fields, "inregistrare_SplitTVA", "statusSplitTVA")?,
            e_invoice_registered: flag(general, "statusRO_e_Factura")?,
        },
    }))
}
