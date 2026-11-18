//! Narrow full-extract recovery for a terminal, final removal. No arbitrary history API.
use super::*;
use crate::RegistryFailure;
use serde_json::{Value, json};
const REMOVAL: &str = "WYKREŚLENIE Z KRAJOWEGO REJESTRU SĄDOWEGO";
fn invalid() -> RegistryFailure {
    RegistryFailure::UnexpectedResponse
}
fn day(s: &str) -> Result<jiff::civil::Date, RegistryFailure> {
    jiff::civil::Date::strptime("%d.%m.%Y", s)
        .ok()
        .filter(|d| d.strftime("%d.%m.%Y").to_string() == s)
        .ok_or_else(invalid)
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, RegistryFailure> {
    v[key]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(invalid)
}
fn number(v: &Value) -> Result<u64, RegistryFailure> {
    let s = v.as_str().ok_or_else(invalid)?;
    let n: u64 = s.parse().map_err(|_| invalid())?;
    if n == 0 || n.to_string() != s {
        return Err(invalid());
    }
    Ok(n)
}
// Select a unique value still present at terminal removal. Earlier withdrawn
// values do not become current again simply because this is a full extract.
fn historical(object: &Value, key: &str, terminal: u64) -> Result<Option<Value>, RegistryFailure> {
    let Some(values) = object.get(key) else {
        return Ok(None);
    };
    let values = values.as_array().ok_or_else(invalid)?;
    let mut selected: Option<(u64, Value)> = None;
    for record in values {
        let introduced = number(&record["nrWpisuWprow"])?;
        let removed = match record.get("nrWpisuWykr") {
            None => None,
            Some(v) => Some(number(v)?),
        };
        if removed.is_some_and(|r| r < introduced) || introduced > terminal {
            return Err(invalid());
        }
        if removed.is_some_and(|r| r < terminal) {
            continue;
        }
        if removed.is_some_and(|r| r > terminal) {
            return Err(invalid());
        }
        // Full-extract seat/address histories store components directly, unlike
        // scalar histories that wrap the value under its field name.
        let value = if matches!(key, "siedziba" | "adres") {
            let mut components = record.as_object().ok_or_else(invalid)?.clone();
            components.remove("nrWpisuWprow");
            components.remove("nrWpisuWykr");
            Value::Object(components)
        } else {
            record.get(key).ok_or_else(invalid)?.clone()
        };
        if selected.replace((introduced, value)).is_some() {
            return Err(invalid());
        }
    }
    Ok(selected.map(|(_, value)| value))
}
fn parties(
    object: &Value,
    key: &str,
    terminal: u64,
) -> Result<Option<Vec<PlMergerParty>>, RegistryFailure> {
    let Some(values) = object.get(key) else {
        return Ok(None);
    };
    let mut result = Vec::new();
    for party in values.as_array().ok_or_else(invalid)? {
        let name = historical(party, "nazwa", terminal)?;
        let register = historical(party, "krajNazwaRejestru", terminal)?;
        let number = historical(party, "numerWRejestrzeAlboEwidencji", terminal)?;
        // A foreign register number must not be presented as a Polish KRS.
        let is_krs = register
            .as_ref()
            .and_then(Value::as_str)
            .is_some_and(|s| s.split(',').any(|s| s.trim() == "KRAJOWY REJESTR SĄDOWY"));
        if name.is_none() && register.is_none() && number.is_none() {
            continue;
        }
        result.push(serde_json::from_value(json!({"nazwa":name,"krajNazwaRejestru":register,"numerWRejestrzeAlboEwidencji":number,"krs":if is_krs { number.clone().map(|v| json!({"krs":v})) } else { None }})).map_err(|_| invalid())?);
    }
    Ok(Some(result))
}
#[cfg(test)]
pub(super) fn decode(
    bytes: &[u8],
    id: &PlBusinessId,
) -> Result<PlCompanyLookupResult, RegistryFailure> {
    decode_for(bytes, id, "RejP")
}
pub(super) fn decode_for(
    bytes: &[u8],
    id: &PlBusinessId,
    register: &str,
) -> Result<PlCompanyLookupResult, RegistryFailure> {
    let root: Value = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    let extract = &root["odpis"];
    let header = &extract["naglowekP"];
    if extract["rodzaj"] != "Pełny" || header["rejestr"] != register {
        return Err(invalid());
    }
    if header["numerKRS"] != id.as_str() {
        return Err(RegistryFailure::MismatchedResponse);
    }
    let entries = header["wpis"]
        .as_array()
        .filter(|e| !e.is_empty())
        .ok_or_else(invalid)?;
    let mut previous = 0;
    for entry in entries {
        let n = entry["numerWpisu"]
            .as_u64()
            .filter(|n| *n > previous)
            .ok_or_else(invalid)?;
        previous = n;
    }
    let last = entries.last().unwrap();
    if last["opis"] != REMOVAL {
        return Err(invalid());
    }
    let entry_date = text(last, "dataWpisu")?;
    let final_date = last
        .get("dataUprawomocnienia")
        .map(|_| text(last, "dataUprawomocnienia"))
        .transpose()?;
    let non_final = last
        .get("nieprawomocny")
        .map(|_| text(last, "nieprawomocny"))
        .transpose()?;
    let snapshot = day(text(header, "stanZDnia")?)?;
    if day(entry_date)? > snapshot || (final_date.is_none() && non_final.is_none()) {
        return Err(invalid());
    }
    if let Some(date) = final_date
        && (day(entry_date)? > day(date)? || day(date)? > snapshot)
    {
        return Err(invalid());
    }
    let section = &extract["dane"]["dzial1"];
    if !section["danePodmiotuZagranicznego"].is_null() {
        return Err(invalid());
    }
    let subject = &section["danePodmiotu"];
    let name = historical(subject, "nazwa", previous)?.ok_or_else(invalid)?;
    let name = name
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(invalid)?
        .to_owned();
    let form = historical(subject, "formaPrawna", previous)?;
    let identifiers = historical(subject, "identyfikatory", previous)?;
    let subject: PlSubject = serde_json::from_value(
        json!({"nazwa":name,"formaPrawna":form,"identyfikatory":identifiers}),
    )
    .map_err(|_| invalid())?;
    let office = &section["siedzibaIAdres"];
    let office = if office.is_null() {
        None
    } else {
        Some(serde_json::from_value(json!({"siedziba":historical(office,"siedziba",previous)?,"adres":historical(office,"adres",previous)?})).map_err(|_| invalid())?)
    };
    let mut reorganizations = Vec::new();
    if let Some(entries) = extract["dane"]["dzial6"].get("polaczeniePodzialPrzeksztalcenie") {
        for event in entries.as_array().ok_or_else(invalid)? {
            let kind = historical(event, "okreslenieOkolicznosci", previous)?;
            let description = historical(event, "opisPolaczeniaPodzialuPrzeksztalcenia", previous)?;
            if kind.is_none() && description.is_none() {
                continue;
            }
            reorganizations.push(PlReorganization {
                opis_podzialu_polaczenia_przeksztalcenia: None,
                okreslenie_okolicznosci: kind
                    .map(serde_json::from_value)
                    .transpose()
                    .map_err(|_| invalid())?,
                opis_polaczenia_podzialu_przeksztalcenia: description
                    .map(serde_json::from_value)
                    .transpose()
                    .map_err(|_| invalid())?,
                podmioty_powstale_przejmujace: parties(
                    event,
                    "podmiotyPowstalePrzejmujace",
                    previous,
                )?,
                podmioty_przejmowane: parties(event, "podmiotyPrzejmowane", previous)?,
            });
        }
    }
    Ok(PlCompanyLookupResult {
        tax_id: id.clone(),
        company_name: name,
        details: PlCompanyDetails {
            unmapped_lifecycle_fields: Vec::new(),
            corporate_events: super::mergers::events(&reorganizations, id.as_str()),
            full_extract_removal: Some(PlRemoval {
                entry_number: previous,
                description: REMOVAL.into(),
                entry_date: entry_date.into(),
                final_date: final_date.map(str::to_owned),
                non_final: non_final.map(str::to_owned),
            }),
            naglowek: serde_json::from_value(header.clone()).map_err(|_| invalid())?,
            dzial6: None,
            foreign_branch: false,
            dane_podmiotu: subject,
            siedziba_i_adres: office,
        },
    })
}
