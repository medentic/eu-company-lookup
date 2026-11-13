use super::{address, html};
use crate::{
    BgBusinessId, BgCompanyLookupResult, Provider, RegistryCapital, RegistryDetails,
    RegistryDocument, RegistryError, RegistryMoney, RegistryPerson,
};
use crate::{CommercialRegisterDetails, CompanyStatus, RegistryDocumentType, RegistryRole};
use chrono::NaiveDateTime;
use scraper::{Html, Selector};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Deed {
    deed_status: i64,
    uic: String,
    full_name: String,
    has_instructions: Option<bool>,
    sections: Vec<Section>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Section {
    #[serde(rename = "subUICType")]
    sub_uic_type: i64,
    sub_deeds: Vec<SubDeed>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubDeed {
    #[serde(rename = "subUIC")]
    sub_uic: Option<String>,
    #[serde(rename = "subUICType")]
    sub_uic_type: i64,
    sub_deed_is_closed: bool,
    groups: Vec<Group>,
}
#[derive(Deserialize)]
struct Group {
    fields: Vec<Field>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Field {
    field_ident: String,
    name_code: String,
    field_operation: i64,
    html_data: Option<String>,
    field_entry_date: Option<String>,
}
fn field<'a>(
    deed: &'a SubDeed,
    ident: &str,
    code: &str,
) -> Result<Option<&'a Field>, RegistryError> {
    let mut fields = deed
        .groups
        .iter()
        .flat_map(|g| &g.fields)
        .filter(|f| f.field_ident == ident);
    let first = fields.next();
    if fields.next().is_some() {
        return Err(RegistryError::UnexpectedResponse);
    }
    let Some(f) = first else { return Ok(None) };
    if f.name_code != code {
        return Err(RegistryError::UnexpectedResponse);
    }
    match f.field_operation {
        2 => Ok(None),
        1 | 3 => Ok(Some(f)),
        _ => Err(RegistryError::UnexpectedResponse),
    }
}
fn text(f: Option<&Field>) -> Option<String> {
    f.and_then(|f| f.html_data.as_deref())
        .map(html::text)
        .filter(|s| !s.is_empty())
}
fn date(f: Option<&Field>) -> Option<NaiveDateTime> {
    f.and_then(|f| f.field_entry_date.as_deref())
        .and_then(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").ok())
}
// Explicit corporate-holder IDs are metadata, not part of the holder name.
// Accept only the observed label with a complete business-code syntax.
fn holder_name(value: &str) -> Option<&str> {
    if value.contains(['\n', '\r']) || value.contains("ЕГН") || value.contains("ЛНЧ") {
        return None;
    }
    let name = if let Some((name, id)) = value.rsplit_once(", ЕИК/ПИК ") {
        if !matches!(id.len(), 9 | 13) || !id.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        name
    } else {
        value
    };
    let name = name.trim();
    (!name.is_empty() && !name.contains("ЕИК")).then_some(name)
}

fn people(f: Option<&Field>, role: RegistryRole) -> Option<Vec<RegistryPerson>> {
    let raw = f?.html_data.as_deref()?;
    let doc = Html::parse_fragment(raw);
    let paragraphs = Selector::parse("p.field-text").unwrap();
    let rows: Vec<_> = doc
        .select(&paragraphs)
        .map(|p| html::text(&p.html()))
        .filter(|s| !s.is_empty())
        .collect();
    if rows.is_empty() {
        return None;
    }
    Some(
        rows.into_iter()
            .map(|source_text| {
                let parts = source_text.rsplit_once(", Държава:");
                let (name, country) = match parts {
                    Some((name, country)) if !country.trim().is_empty() => {
                        match holder_name(name) {
                            Some(name) => (Some(name.to_owned()), Some(country.trim().to_owned())),
                            None => (None, None),
                        }
                    }
                    None if !source_text.is_empty()
                        && source_text.chars().all(|c| {
                            c.is_alphabetic() || matches!(c, ' ' | '-' | '.' | '\'' | '’')
                        }) =>
                    {
                        (Some(source_text.clone()), None)
                    }
                    _ => (None, None),
                };
                let recognized = country
                    .as_deref()
                    .and_then(crate::locations::Country::from_name);
                let country_iso2 = recognized.map(|c| c.iso2().into());
                let country = recognized.map(|c| c.native_name().into()).or(country);
                RegistryPerson {
                    name,
                    role,
                    country,
                    country_iso2,
                    source_text,
                }
            })
            .collect(),
    )
}
fn money(f: Option<&Field>) -> Option<RegistryMoney> {
    let source_text = text(f)?;
    let recognized = [("€", "EUR"), ("EUR", "EUR"), ("лв.", "BGN"), ("BGN", "BGN")]
        .into_iter()
        .find_map(|(suffix, code)| source_text.strip_suffix(suffix).map(|n| (n.trim(), code)));
    let (amount, currency) = if let Some((number, currency)) = recognized {
        let mut parts = number.split('.');
        let integer = parts.next().unwrap_or("");
        let decimal = parts.next();
        let valid = !integer.is_empty()
            && integer.bytes().all(|b| b.is_ascii_digit())
            && decimal.is_none_or(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
            && parts.next().is_none();
        (valid.then(|| number.to_owned()), Some(currency.to_owned()))
    } else {
        (None, None)
    };
    Some(RegistryMoney {
        amount,
        currency,
        source_text,
    })
}
fn documents(f: Option<&Field>) -> Option<Vec<RegistryDocument>> {
    let doc = Html::parse_fragment(f?.html_data.as_deref()?);
    let links = Selector::parse("a[href]").unwrap();
    let base = reqwest::Url::parse("https://portal.registryagency.bg/CR/").unwrap();
    let official_url = |a: scraper::ElementRef<'_>| {
        let url = base.join(a.value().attr("href")?).ok()?;
        (url.origin() == base.origin()
            && url.path().starts_with("/CR/DocumentAccess/")
            && url.username().is_empty()
            && url.password().is_none())
        .then_some(url)
    };
    let documents: Vec<_> = doc
        .select(&links)
        .filter_map(|a| {
            let url = official_url(a)?;
            let title = html::text(&a.html());
            if title.is_empty() {
                return None;
            }
            // A date belongs to this link only within a block containing exactly one
            // official document link. Never borrow the first date from a multi-link field.
            let mut announced_at_local = None;
            for parent in a.ancestors().filter_map(scraper::ElementRef::wrap) {
                let count = parent
                    .select(&links)
                    .filter(|link| official_url(*link).is_some())
                    .count();
                if count > 1 {
                    break;
                }
                if count != 1 {
                    continue;
                }
                let decoded = html::text(&parent.html());
                let dates: Vec<_> = decoded
                    .lines()
                    .filter_map(|line| line.strip_prefix("Дата на обявяване:"))
                    .collect();
                if !dates.is_empty() {
                    if dates.len() == 1 {
                        announced_at_local =
                            NaiveDateTime::parse_from_str(dates[0].trim(), "%d.%m.%Y г. %H:%M:%S")
                                .ok();
                    }
                    break;
                }
            }
            Some(RegistryDocument {
                document_type: RegistryDocumentType::ConstitutiveDocument,
                title,
                announced_at_local,
                url: url.into(),
            })
        })
        .collect();
    (!documents.is_empty()).then_some(documents)
}

pub(super) fn decode(
    body: &[u8],
    id: &BgBusinessId,
) -> Result<BgCompanyLookupResult, RegistryError> {
    let deed: Deed = serde_json::from_slice(body).map_err(|_| RegistryError::UnexpectedResponse)?;
    if deed.uic != id.as_str() {
        return Err(RegistryError::MismatchedResponse);
    }
    if deed.full_name.trim().is_empty() {
        return Err(RegistryError::UnexpectedResponse);
    }
    let main: Vec<_> = deed
        .sections
        .iter()
        .filter(|s| s.sub_uic_type == 1)
        .flat_map(|s| &s.sub_deeds)
        .filter(|s| s.sub_uic_type == 1)
        .collect();
    if main.len() != 1 {
        return Err(RegistryError::UnexpectedResponse);
    }
    let main = main[0];
    let get = |ident, code| field(main, ident, code);
    let uic = get("00010", "CR_F_1_L")?;
    if let Some(value) = text(uic) {
        let mut lines = value.lines();
        if lines.next() != Some(id.as_str()) {
            return Err(RegistryError::MismatchedResponse);
        }
        // Migrated records append the original court-file reference to field 1.
        // It is metadata, not part of the UIC; do not accept arbitrary extra text.
        if lines.any(|line| !line.starts_with("Фирмено дело: ")) {
            return Err(RegistryError::UnexpectedResponse);
        }
    }
    let deleted_field = text(get("00270", "CR_F_27_L")?);
    // A deleted company may keep an open main subdeed. The top-level deleted
    // state and explicit deletion field take precedence over that subdeed flag.
    let closed = main.sub_deed_is_closed
        || deed.deed_status == 4
        || deleted_field.as_deref() == Some("Заличен търговец/ЮЛНЦ");
    let status = if closed {
        Some(CompanyStatus::Closed)
    } else if matches!(deed.deed_status, 1 | 2) {
        Some(CompanyStatus::Active)
    } else {
        Some(CompanyStatus::Unknown)
    };
    let company_name = if deed.full_name.contains("<span") {
        let decoded = html::text(&deed.full_name).replace('\u{a0}', " ");
        if closed {
            decoded
                .strip_suffix(" - Заличен търговец/ЮЛНЦ")
                .unwrap_or(&decoded)
                .trim()
                .to_owned()
        } else {
            decoded
        }
    } else {
        deed.full_name.clone()
    };
    if company_name.is_empty() {
        return Err(RegistryError::UnexpectedResponse);
    }
    let identifier_date = date(uic);
    let doc_deeds: Vec<_> = deed
        .sections
        .iter()
        .filter(|s| s.sub_uic_type == 13)
        .flat_map(|s| &s.sub_deeds)
        .filter(|s| s.sub_uic_type == 13)
        .collect();
    // The portal separates annual accounts and constitutive acts into different
    // section-13 subdeeds for the same main record. Select the field, not a section count.
    let mut constitutive = Vec::new();
    for doc_deed in doc_deeds.into_iter().filter(|d| d.sub_uic == main.sub_uic) {
        if let Some(value) = field(doc_deed, "1001AJ", "CR_F_1001_L")? {
            constitutive.push(value);
        }
    }
    if constitutive.len() > 1 {
        return Err(RegistryError::UnexpectedResponse);
    }
    let docs = documents(constitutive.first().copied());
    Ok(BgCompanyLookupResult {
        tax_id: id.clone(),
        source: Provider::BulgarianCommercialRegister,
        registry_url: format!(
            "https://portal.registryagency.bg/CR/Reports/ActiveConditionTabResult?uic={}",
            id.as_str()
        ),
        company_name,
        company_english_name: text(get("00040", "CR_F_4_L")?),
        legal_form: text(get("00030", "CR_F_3_L")?).map(|s| super::legal_form::abbreviate(&s)),
        address: address::commercial(text(get("00050", "CR_F_5_L")?)),
        status,
        native_registry_status: Some(deed.deed_status.to_string()),
        registry_details: RegistryDetails::BulgarianCommercialRegister(Box::new(
            CommercialRegisterDetails {
                status_code: deed.deed_status,
                registered_name: text(get("00020", "CR_F_2_L")?),
                identifier_entry_date: identifier_date.map(|d| d.date()),
                managers: people(get("00070", "CR_F_7_L")?, RegistryRole::Manager),
                owners: people(get("00230", "CR_F_23_L")?, RegistryRole::SoleCapitalOwner),
                capital: RegistryCapital {
                    registered: money(get("00310", "CR_F_31_L")?),
                    paid_in: money(get("00320", "CR_F_32_L")?),
                },
                business_activity: text(get("00060", "CR_F_6_L")?),
                documents: docs,
                has_instructions: deed.has_instructions,
            },
        )),
    })
}
