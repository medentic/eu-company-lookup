//! One bounded HTTP exchange shared by production and development probes.
//! Admission, retries and SOAP error policy belong to callers.
use super::{VatId, soap};
use std::time::Duration;

pub(super) const ENDPOINT: &str =
    "https://ec.europa.eu/taxation_customs/vies/services/checkVatService";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Failure {
    Timeout,
    Network,
    TooLarge,
    InvalidUtf8,
}
impl Failure {
    fn network(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::Timeout
        } else {
            Self::Network
        }
    }
}
#[derive(Debug)]
pub(super) enum Body {
    Text(String),
    Skipped,
    Failed(Failure),
}
#[derive(Debug)]
pub(super) struct Response {
    pub status: reqwest::StatusCode,
    pub retry_after: Option<String>,
    pub body: Body,
}

pub(super) fn client(
    connect: Duration,
    total: Option<Duration>,
) -> Result<reqwest::Client, reqwest::Error> {
    let mut builder = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .connect_timeout(connect)
        .user_agent(concat!("eu-company-lookup/", env!("CARGO_PKG_VERSION")));
    if let Some(total) = total {
        builder = builder.timeout(total);
    }
    builder.build()
}

pub(super) async fn exchange(
    http: &reqwest::Client,
    endpoint: &str,
    vat: &VatId,
    limit: usize,
) -> Result<Response, Failure> {
    let mut response = http
        .post(endpoint)
        .header("Content-Type", "text/xml; charset=utf-8")
        .header("SOAPAction", "\"\"")
        .body(soap::request(vat))
        .send()
        .await
        .map_err(Failure::network)?;
    let status = response.status();
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = if matches!(status.as_u16(), 408 | 429 | 502 | 504) {
        Body::Skipped
    } else if response.content_length().is_some_and(|n| n > limit as u64) {
        Body::Failed(Failure::TooLarge)
    } else {
        read_body(&mut response, limit).await
    };
    Ok(Response {
        status,
        retry_after,
        body,
    })
}

async fn read_body(response: &mut reqwest::Response, limit: usize) -> Body {
    let mut bytes = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) if chunk.len() <= limit.saturating_sub(bytes.len()) => {
                bytes.extend_from_slice(&chunk)
            }
            Ok(Some(_)) => return Body::Failed(Failure::TooLarge),
            Ok(None) => break,
            Err(error) => return Body::Failed(Failure::network(error)),
        }
    }
    match String::from_utf8(bytes) {
        Ok(text) => Body::Text(text),
        Err(_) => Body::Failed(Failure::InvalidUtf8),
    }
}
