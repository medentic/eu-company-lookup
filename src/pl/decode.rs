use super::*;
use crate::RegistryError;
#[derive(serde::Deserialize)]
struct Wire {
    odpis: Extract,
}
#[derive(serde::Deserialize)]
struct Extract {
    rodzaj: String,
    #[serde(rename = "naglowekA")]
    header: PlExtractHeader,
    dane: Data,
}
#[derive(serde::Deserialize)]
struct Data {
    dzial1: Section,
}
#[derive(serde::Deserialize)]
struct Section {
    #[serde(rename = "danePodmiotu")]
    subject: Option<PlSubject>,
    #[serde(rename = "danePodmiotuZagranicznego")]
    foreign_subject: Option<PlSubject>,
    #[serde(rename = "siedzibaIAdres")]
    office: Option<PlRegisteredOffice>,
    #[serde(rename = "siedzibaIAdresPodmiotuZagranicznego")]
    foreign_office: Option<PlRegisteredOffice>,
}
pub(super) fn decode(
    bytes: &[u8],
    id: &PlBusinessId,
) -> Result<PlCompanyLookupResult, RegistryError> {
    let v: Wire = serde_json::from_slice(bytes).map_err(|_| RegistryError::UnexpectedResponse)?;
    let e = v.odpis;
    if e.rodzaj != "Aktualny" || e.header.rejestr != "RejP" {
        return Err(RegistryError::UnexpectedResponse);
    }
    if e.header.numer_krs != id.as_str() {
        return Err(RegistryError::MismatchedResponse);
    }
    let section = e.dane.dzial1;
    let (subject, office, foreign_branch) = match (section.subject, section.foreign_subject) {
        (Some(subject), None) if section.foreign_office.is_none() => {
            (subject, section.office, false)
        }
        (None, Some(subject)) if section.office.is_none() => {
            (subject, section.foreign_office, true)
        }
        _ => return Err(RegistryError::UnexpectedResponse),
    };
    if subject.nazwa.trim().is_empty() {
        return Err(RegistryError::UnexpectedResponse);
    }
    Ok(PlCompanyLookupResult {
        tax_id: id.clone(),
        company_name: subject.nazwa.clone(),
        details: PlCompanyDetails {
            naglowek: e.header,
            dane_podmiotu: subject,
            siedziba_i_adres: office,
            foreign_branch,
        },
    })
}
