//! Browser-printed evidence. A single form submission owns every print attempt.
use super::{VatId, failure::Failure};
use crate::model::LookupFailure;
use chromiumoxide::{Browser, BrowserConfig, Page, cdp::browser_protocol::page::PrintToPdfParams};
use futures_util::StreamExt;
use serde::Deserialize;
use std::time::Duration;

mod document;
#[cfg(test)]
mod tests;

pub(super) const FORM_URL: &str =
    "https://ec.europa.eu/taxation_customs/vies/?locale=en#/vat-validation";

type Result<T> = std::result::Result<T, LookupFailure>;

pub(super) async fn capture(
    url: &str,
    checked: &VatId,
    requester: &VatId,
) -> std::result::Result<Vec<u8>, Failure> {
    let profile =
        tempfile::tempdir().map_err(|_| LookupFailure::ViesPdf("browser_profile_unavailable"))?;
    let config = BrowserConfig::builder()
        .user_data_dir(profile.path())
        .new_headless_mode()
        .respect_https_errors();
    let config = config
        .build()
        .map_err(|_| LookupFailure::InvalidConfiguration("chrome_or_chromium_required"))?;
    let (mut browser, mut handler) = Browser::launch(config)
        .await
        .map_err(|_| LookupFailure::ViesPdf("browser_launch_failed"))?;
    let events = tokio::spawn(async move { while handler.next().await.is_some() {} });
    let result = async {
        let page = browser
            .new_page(url)
            .await
            .map_err(|_| LookupFailure::Transport)?;
        submit(&page, url, checked, requester).await?;
        let fields = read_result(&page, url, checked).await?;
        print(&page, fields).await
    }
    .await;
    // Dropping Browser also kills its owned child if the caller cancels or times out.
    if browser.close().await.is_err() {
        let _ = browser.kill().await;
    }
    let _ = browser.wait().await;
    events.abort();
    result.map_err(Failure::from)
}

async fn evaluate<T: serde::de::DeserializeOwned>(page: &Page, script: &str) -> Result<T> {
    page.evaluate_expression(script)
        .await
        .map_err(|_| LookupFailure::ViesPdf("browser_page_unavailable"))?
        .into_value()
        .map_err(|_| LookupFailure::ViesPdf("browser_page_unavailable"))
}

async fn submit(page: &Page, url: &str, checked: &VatId, requester: &VatId) -> Result<()> {
    // Angular's selects are populated asynchronously. No submit occurs while waiting.
    let args = serde_json::json!([
        url,
        checked.prefix(),
        checked.number(),
        requester.prefix(),
        requester.number()
    ]);
    let script = format!("({SUBMIT})(...{args})");
    loop {
        match evaluate::<String>(page, &script).await?.as_str() {
            "submitted" => return Ok(()),
            "loading" => tokio::time::sleep(Duration::from_millis(100)).await,
            "rejected" => return Err(LookupFailure::ProviderRejectedInput),
            _ => return Err(LookupFailure::ViesPdf("unexpected_validation_form")),
        }
    }
}

const SUBMIT: &str = r#"(url, country, number, requesterCountry, requesterNumber) => {
    const expected = new URL(url);
    if (location.origin !== expected.origin || location.pathname !== expected.pathname ||
        location.hash !== expected.hash) return 'unexpected';
    const form = document.querySelector('#vat-validation-form');
    if (!form) return 'loading';
    const control = name => form.querySelector('[formcontrolname="' + name + '"]');
    const countryBox = control('country'), requesterBox = control('requesterCountry');
    const numberBox = control('vatNumber'), requesterNumberBox = control('requesterVatNumber');
    const verify = document.querySelector('[data-testid="verifyBtn"]');
    if (!countryBox || !requesterBox || !numberBox || !requesterNumberBox || !verify) return 'unexpected';
    const select = (box, value) => {
        if (![...box.options].some(option => option.value === value)) return false;
        if (box.value !== value) {
            box.value = value;
            box.dispatchEvent(new Event('change', {bubbles:true}));
        }
        return true;
    };
    const enter = (box, value) => {
        if (box.value === value) return;
        box.focus();
        box.value = value;
        box.dispatchEvent(new Event('input', {bubbles:true}));
        box.dispatchEvent(new Event('change', {bubbles:true}));
        box.blur();
    };
    if (!select(countryBox, country)) return 'loading';
    enter(numberBox, number);
    if (!select(requesterBox, requesterCountry)) {
        // The requester list arrives independently; focus/blur refreshes the form view.
        requesterBox.focus();
        requesterBox.blur();
        return 'loading';
    }
    enter(requesterNumberBox, requesterNumber);
    if (!form.checkValidity() || verify.disabled)
        return 'rejected';
    verify.click();
    return 'submitted';
}"#;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Observation {
    country: String,
    number: String,
    date: String,
    reference: String,
    outcome: String,
    cells: Vec<String>,
}

#[derive(Deserialize)]
#[serde(tag = "state", content = "result", rename_all = "snake_case")]
enum PageState {
    Waiting,
    Rejected,
    Complete(Observation),
}

async fn read_result(page: &Page, url: &str, checked: &VatId) -> Result<Vec<String>> {
    let expected = url.replace("#/vat-validation", "#/vat-validation-result");
    let script = format!("({READ_RESULT})({})", serde_json::json!(expected));
    let observation = loop {
        match evaluate::<PageState>(page, &script).await? {
            PageState::Complete(result) => break result,
            PageState::Rejected => return Err(LookupFailure::ProviderRejectedInput),
            PageState::Waiting => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    };
    observation.fields(checked)
}

impl Observation {
    fn fields(self, checked: &VatId) -> Result<Vec<String>> {
        if self.country != checked.prefix() || self.number != checked.number() {
            return Err(LookupFailure::MismatchedResponse);
        }
        if self.outcome != "Yes, valid VAT number"
            && !self
                .outcome
                .starts_with("No, invalid VAT number for cross border transactions within the EU")
        {
            return Err(LookupFailure::MalformedResponse);
        }
        if self.date.is_empty() {
            return Err(LookupFailure::MalformedResponse);
        }
        if !self.reference.chars().any(|c| c.is_ascii_alphanumeric()) {
            return Err(LookupFailure::ViesPdf("consultation_number_missing"));
        }
        let mut fields = self.cells;
        fields.extend([
            self.country,
            self.number,
            self.date,
            self.reference,
            self.outcome,
        ]);
        Ok(fields)
    }
}

const READ_RESULT: &str = r#"async url => {
    const expected = new URL(url);
    if (location.pathname !== expected.pathname || location.origin !== expected.origin)
        throw Error('unexpected origin');
    if (location.hash !== expected.hash) {
        if (document.querySelector('#vat-validation-form.ng-submitted.ng-invalid')) return {state:'rejected'};
        return {state:'waiting'};
    }
    const result = document.querySelector('.vat-result');
    if (!result || !result.querySelector('#print')) return {state:'waiting'};
    await document.fonts.ready;
    const field = id => result.querySelector('[data-testid="' + id + '"]')?.innerText.trim() ?? '';
    if (!field('msCode') || !(field('correctedVatNumber') || field('vatNumber')) || !field('requestDate') || !field('textResult')) return {state:'waiting'};
    return {state:'complete', result:{country:field('msCode'), number:field('correctedVatNumber') || field('vatNumber'),
            date:field('requestDate'), reference:field('requestIdentifier'), outcome:field('textResult'),
            cells:[...result.querySelectorAll('td')].map(cell => cell.innerText.trim()).filter(Boolean)}};
}"#;

async fn print(page: &Page, fields: Vec<String>) -> Result<Vec<u8>> {
    for scale in [1.0, 0.95, 0.9, 0.85, 0.8, 0.75] {
        let params = PrintToPdfParams::builder()
            .paper_width(210.0 / 25.4)
            .paper_height(297.0 / 25.4)
            .scale(scale)
            .margin_top(0.32)
            .margin_bottom(0.32)
            .margin_left(0.32)
            .margin_right(0.32)
            .print_background(true)
            .display_header_footer(false)
            .build();
        let bytes = page
            .pdf(params)
            .await
            .map_err(|_| LookupFailure::ViesPdf("print_failed"))?;
        let output = tokio::task::spawn_blocking(move || document::finish(&bytes))
            .await
            .map_err(|_| LookupFailure::ViesPdf("pdf_processing_failed"))??;
        if let Some(pdf) = output {
            document::verify_text(&pdf, &fields).await?;
            return Ok(pdf);
        }
    }
    Err(LookupFailure::ViesPdf("result_does_not_fit_one_a4_page"))
}
