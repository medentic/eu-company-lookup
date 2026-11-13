use super::{address, html::element_text};
use crate::{BgBusinessId, BgCompanyLookupResult, Provider, RegistryDetails, RegistryError};
use scraper::{ElementRef, Html, Selector};

pub(super) const BASE: &str = "https://reports.bulstat.bg/bulstat-ireports/";
const TITLE: &str = "Справка за субект на БУЛСТАТ";

pub(super) fn public_url(id: &BgBusinessId) -> String {
    let mut url =
        reqwest::Url::parse(&format!("{BASE}preview")).expect("fixed public registry URL");
    url.query_pairs_mut()
        .append_pair("__report", &format!("{BASE}v_rpt1.rptdesign"))
        .append_pair("__format", "html")
        .append_pair("__locale", "bg")
        .append_pair("report_title", TITLE)
        .append_pair("uic", id.as_str());
    url.into()
}

fn document(body: &[u8]) -> Result<Html, RegistryError> {
    let text = std::str::from_utf8(body).map_err(|_| RegistryError::UnexpectedResponse)?;
    let doc = Html::parse_document(text);
    let visible = element_text(doc.root_element());
    if !visible.lines().any(|s| s == "РЕГИСТЪР БУЛСТАТ - СПРАВКИ")
        || !visible.lines().any(|s| s == TITLE)
    {
        return Err(RegistryError::UnexpectedResponse);
    }
    Ok(doc)
}

fn cells(row: ElementRef<'_>) -> Vec<ElementRef<'_>> {
    row.children()
        .filter_map(ElementRef::wrap)
        .filter(|e| matches!(e.value().name(), "td" | "th"))
        .collect()
}

/// Select the exact code, never the first (possibly related) subject. Links are
/// inspected only; the client constructs its own fixed, allowlisted detail URL.
pub(super) fn search(body: &[u8], id: &BgBusinessId) -> Result<Option<String>, RegistryError> {
    let doc = document(body)?;
    let mut matches = Vec::new();
    let mut rows = 0;
    for row in doc.select(&Selector::parse("tr").unwrap()) {
        let columns = cells(row);
        if columns.len() != 2 {
            continue;
        }
        let code = element_text(columns[0]);
        if !matches!(code.len(), 9 | 13) || !code.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let links: Vec<_> = row.select(&Selector::parse("a[href]").unwrap()).collect();
        let mut valid_link = false;
        for link in links {
            let Ok(url) = reqwest::Url::parse(link.value().attr("href").unwrap()) else {
                continue;
            };
            let params: Vec<_> = url.query_pairs().collect();
            if url.origin().ascii_serialization() == "https://reports.bulstat.bg"
                && url.path() == "/bulstat-ireports/preview"
                && params.iter().filter(|(k, _)| k == "uic").count() == 1
                && params.iter().any(|(k, v)| k == "uic" && v == &code)
                && params
                    .iter()
                    .any(|(k, v)| k == "__report" && v == &format!("{BASE}v_rpt1.rptdesign"))
            {
                valid_link = true;
            }
        }
        if !valid_link {
            return Err(RegistryError::UnexpectedResponse);
        }
        rows += 1;
        if code == id.as_str() {
            let name = element_text(columns[1]);
            if name.is_empty() {
                return Err(RegistryError::UnexpectedResponse);
            }
            matches.push(name);
        }
    }
    if matches.len() > 1 {
        return Err(RegistryError::UnexpectedResponse);
    }
    if let Some(name) = matches.pop() {
        return Ok(Some(name));
    }
    let text = element_text(doc.root_element());
    // A recognized empty report is definitive. A nonempty but unmatched/capped
    // report is deliberately inconclusive until its search semantics are proven.
    if rows == 0 && text.lines().any(|s| s == "Няма намерен резултат!") {
        return Ok(None);
    }
    Err(RegistryError::UnexpectedResponse)
}

pub(super) fn detail(
    body: &[u8],
    id: &BgBusinessId,
    name: String,
) -> Result<BgCompanyLookupResult, RegistryError> {
    let doc = document(body)?;
    let table = doc
        .select(&Selector::parse("table#tab1").unwrap())
        .next()
        .ok_or(RegistryError::UnexpectedResponse)?;
    let mut values = std::collections::HashMap::new();
    let mut addresses = Vec::new();
    for row in table.select(&Selector::parse("tr").unwrap()) {
        let columns = cells(row);
        if columns.len() != 4 {
            continue;
        }
        let label = element_text(columns[0]);
        if matches!(
            label.as_str(),
            "Код по БУЛСТАТ" | "Нефизическо лице" | "Физическо лице" | "Състояние"
        ) {
            if values.insert(label, element_text(columns[1])).is_some() {
                return Err(RegistryError::UnexpectedResponse);
            }
        } else if label.is_empty() {
            let text = element_text(columns[1]);
            if text.starts_with("седалище и адрес на управление:") {
                addresses.push(text);
            }
        }
    }
    let code = values
        .get("Код по БУЛСТАТ")
        .ok_or(RegistryError::UnexpectedResponse)?;
    if code != id.as_str() {
        return Err(RegistryError::MismatchedResponse);
    }
    let subjects: Vec<_> = ["Нефизическо лице", "Физическо лице"]
        .iter()
        .filter_map(|k| values.get(*k))
        .collect();
    if subjects.len() != 1 || subjects[0].is_empty() {
        return Err(RegistryError::UnexpectedResponse);
    }
    let subject = subjects[0];
    // Names can contain commas; take the separately supplied search name instead
    // of splitting the compound subject description at an arbitrary comma.
    if subject != &name && !subject.starts_with(&format!("{name},")) {
        return Err(RegistryError::UnexpectedResponse);
    }
    let raw_address = if addresses.len() == 1 {
        addresses.pop()
    } else if addresses.is_empty() {
        None
    } else {
        Some(addresses.join("\n"))
    };
    let address = if raw_address.as_deref().is_some_and(|s| s.contains('\n')) {
        crate::CompanyAddress {
            full_address_by_registry: raw_address,
            ..Default::default()
        }
    } else {
        address::bulstat(raw_address)
    };
    let status_text = values.get("Състояние").cloned().filter(|s| !s.is_empty());
    let status = status_text.as_deref().map(|text| match text {
        "развиващ дейност" => crate::CompanyStatus::Active,
        _ => crate::CompanyStatus::Unknown,
    });
    Ok(BgCompanyLookupResult {
        tax_id: id.clone(),
        source: Provider::Bulstat,
        registry_url: public_url(id),
        company_name: name,
        company_english_name: None,
        legal_form: subject
            .split_once(", правна форма: ")
            .map(|(_, s)| super::legal_form::abbreviate(s)),
        address,
        status,
        native_registry_status: status_text.clone(),
        registry_details: RegistryDetails::Bulstat(crate::BulstatDetails {
            subject_text: subject.clone(),
            status_text,
        }),
    })
}
