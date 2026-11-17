use super::*;
use crate::RegistryError;
use scraper::{Html, Selector};
fn selector(s: &str) -> Selector {
    Selector::parse(s).expect("static selector")
}
pub(super) fn decode(
    bytes: &[u8],
    id: &IsBusinessId,
) -> Result<IsCompanyLookupResult, RegistryError> {
    let raw = std::str::from_utf8(bytes).map_err(|_| RegistryError::UnexpectedResponse)?;
    let html = Html::parse_document(raw);
    let boxes: Vec<_> = html.select(&selector("div.company")).collect();
    if boxes.len() != 1 {
        return Err(RegistryError::UnexpectedResponse);
    }
    let page = boxes[0];
    let headings: Vec<_> = page.select(&selector("h1")).collect();
    if headings.len() != 1 {
        return Err(RegistryError::UnexpectedResponse);
    }
    let heading = headings[0].text().collect::<String>();
    let suffix = format!(" ({})", id.as_str());
    let name = heading
        .trim()
        .strip_suffix(&suffix)
        .ok_or(RegistryError::MismatchedResponse)?
        .trim();
    if name.is_empty() {
        return Err(RegistryError::UnexpectedResponse);
    }
    let mut matches = Vec::new();
    for table in page.select(&selector("table")) {
        let headers: Vec<_> = table
            .select(&selector("thead th"))
            .map(|e| e.text().collect::<String>().trim().to_owned())
            .collect();
        if headers == ["Póstfang", "Lögheimili", "Sveitarfélag", "Rekstrarform"] {
            matches.push(table)
        }
    }
    if matches.len() != 1 {
        return Err(RegistryError::UnexpectedResponse);
    }
    let rows: Vec<_> = matches[0].select(&selector("tbody tr")).collect();
    if rows.len() != 1 {
        return Err(RegistryError::UnexpectedResponse);
    }
    let cells: Vec<_> = rows[0].select(&selector("td")).collect();
    if cells.len() != 4 {
        return Err(RegistryError::UnexpectedResponse);
    }
    let form: Vec<_> = cells[3]
        .text()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let subtitles: Vec<_> = page
        .select(&selector("h2.subtitle"))
        .filter(|e| {
            e.text()
                .collect::<String>()
                .trim_start()
                .starts_with("Stofnað/Skráð:")
        })
        .collect();
    if subtitles.len() > 1 {
        return Err(RegistryError::UnexpectedResponse);
    }
    let registration_text = subtitles.first().map(|e| e.text().collect::<String>());
    let details = IsCompanyDetails {
        kennitala: id.to_string(),
        registration_text,
        postal_address: cells[0].text().collect(),
        legal_address: cells[1].text().collect(),
        municipality_text: cells[2].text().collect(),
        legal_form_text: cells[3].text().collect(),
        legal_form_code: (form.len() == 2).then(|| form[0].to_owned()),
        legal_form_description: (form.len() == 2).then(|| form[1].to_owned()),
    };
    Ok(IsCompanyLookupResult {
        tax_id: id.clone(),
        company_name: name.to_owned(),
        details,
    })
}
