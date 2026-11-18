use super::*;
use crate::RegistryFailure;
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
    dzial6: Option<serde_json::Value>,
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
    let v: Wire = serde_json::from_slice(bytes).map_err(|_| RegistryFailure::UnexpectedResponse)?;
    let mut e = v.odpis;
    if e.rodzaj != "Aktualny" || e.header.rejestr != register {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    if e.header.numer_krs != id.as_str() {
        return Err(RegistryFailure::MismatchedResponse);
    }
    let mut unmapped_lifecycle_fields = Vec::new();
    if let Some(raw) = &e.dane.dzial6 {
        let fields = raw.as_object().ok_or(RegistryFailure::UnexpectedResponse)?;
        for key in fields.keys() {
            if !matches!(
                key.as_str(),
                "polaczeniePodzialPrzeksztalcenie"
                    | "rozwiazanieUniewaznienie"
                    | "zawieszenieWznowienie"
                    | "likwidacja"
                    | "postepowanieUpadlosciowe"
                    | "postepowanieRestrukturyzacyjneNaprawczePrzymusowaRestrukturyzacjaUporzadkowanaLikwidacja"
            ) {
                unmapped_lifecycle_fields.push(key.clone());
            }
        }
        for (group, allowed) in [
            (
                "likwidacja",
                &[
                    "otwarcieLikwidacji",
                    "zakonczenieLikwidacji",
                    "likwidatorzy",
                    "sposobReprezentacjiPodmiotuPrzezLikwidatorow",
                    "liquidatorCount",
                ][..],
            ),
            (
                "zawieszenieWznowienie",
                &["dataZawieszenia", "dataWznowienia"][..],
            ),
            (
                "polaczeniePodzialPrzeksztalcenie",
                &[
                    "okreslenieOkolicznosci",
                    "opisPolaczeniaPodzialuPrzeksztalcenia",
                    "opisPodzialuPolaczeniaPrzeksztalcenia",
                    "podmiotyPowstalePrzejmujace",
                    "podmiotyPrzejmowane",
                ][..],
            ),
            (
                "postepowanieUpadlosciowe",
                &[
                    "informacjaOOgloszeniuUpadlosci",
                    "opisZakonczeniaProcesuUpadlosci",
                    "sposobProwadzeniaPostepowania",
                ][..],
            ),
        ] {
            for entry in fields
                .get(group)
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(object) = entry.as_object() {
                    for (key, value) in object {
                        // An empty annulment object reports no fact; nonempty content
                        // needs interpretation before an ended bankruptcy permits active.
                        let empty_annulment = group == "postepowanieUpadlosciowe"
                            && key == "informacjaOUchyleniuUkladu"
                            && value.as_object().is_some_and(|o| o.is_empty());
                        if !allowed.contains(&key.as_str()) && !empty_annulment {
                            unmapped_lifecycle_fields.push(format!("{group}.{key}"));
                        }
                    }
                }
            }
        }
        unmapped_lifecycle_fields.sort();
        unmapped_lifecycle_fields.dedup();
    }
    if let Some(entries) = e
        .dane
        .dzial6
        .as_mut()
        .and_then(|s| s.get_mut("likwidacja"))
        .and_then(serde_json::Value::as_array_mut)
    {
        for entry in entries {
            let object = entry
                .as_object_mut()
                .ok_or(RegistryFailure::UnexpectedResponse)?;
            object.remove("liquidatorCount"); // Never trust provider-injected derived fields.
            if let Some(people) = object.get("likwidatorzy") {
                let count = people
                    .as_array()
                    .ok_or(RegistryFailure::UnexpectedResponse)?
                    .len();
                object.insert("liquidatorCount".into(), serde_json::json!(count));
            }
        }
    }
    let lifecycle: Option<PlLifecycleSection> = e
        .dane
        .dzial6
        .map(serde_json::from_value)
        .transpose()
        .map_err(|_| RegistryFailure::UnexpectedResponse)?;
    let section = e.dane.dzial1;
    let (subject, office, foreign_branch) = match (section.subject, section.foreign_subject) {
        (Some(subject), None) if section.foreign_office.is_none() => {
            (subject, section.office, false)
        }
        (None, Some(subject)) if section.office.is_none() => {
            (subject, section.foreign_office, true)
        }
        _ => return Err(RegistryFailure::UnexpectedResponse),
    };
    if subject.nazwa.trim().is_empty() {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    Ok(PlCompanyLookupResult {
        tax_id: id.clone(),
        company_name: subject.nazwa.clone(),
        details: PlCompanyDetails {
            unmapped_lifecycle_fields,
            corporate_events: if foreign_branch {
                Vec::new()
            } else {
                super::mergers::events(
                    lifecycle
                        .as_ref()
                        .and_then(|s| s.polaczenie_podzial_przeksztalcenie.as_deref())
                        .unwrap_or_default(),
                    id.as_str(),
                )
            },
            full_extract_removal: None,
            naglowek: e.header,
            dzial6: lifecycle,
            dane_podmiotu: subject,
            siedziba_i_adres: office,
            foreign_branch,
        },
    })
}
