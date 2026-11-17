//! Per-future capture. No shared last-response slot, retries or secondary requests.
use crate::{
    CapturedBytes, CapturedHeader, CapturedRequest, CapturedResponse, HttpDiagnostics,
    HttpExchange, LookupError, Provider,
};
use reqwest::header::HeaderMap;
use std::{cell::RefCell, future::Future};

tokio::task_local! { static CAPTURE: RefCell<State>; }
struct State {
    raw: bool,
    limit: usize,
    exchanges: Vec<HttpExchange>,
    body_bytes: Vec<u8>,
    pending: Option<reqwest::Response>,
    status: Option<u16>,
    content_type: Option<String>,
}
pub(crate) async fn scope<T>(
    raw: bool,
    limit: usize,
    timeout: std::time::Duration,
    provider: Provider,
    future: impl Future<Output = Result<T, LookupError>>,
) -> Result<T, LookupError> {
    let deadline = tokio::time::Instant::now() + timeout;
    CAPTURE
        .scope(
            RefCell::new(State {
                raw,
                limit,
                exchanges: Vec::new(),
                body_bytes: Vec::new(),
                pending: None,
                status: None,
                content_type: None,
            }),
            async {
                let result = future.await;
                if raw && result.is_err() {
                    // The provider has already classified the observation. Drain only the
                    // unread response, without changing that classification or sending again.
                    let pending = CAPTURE.with(|s| s.borrow_mut().pending.take());
                    if let Some(mut response) = pending {
                        let read = async {
                            loop {
                                match response.chunk().await {
                                    Ok(Some(chunk)) => {
                                        if !record_chunk(&chunk) {
                                            break;
                                        }
                                    }
                                    Ok(None) => {
                                        finish(true, None);
                                        break;
                                    }
                                    Err(error) => {
                                        finish(
                                            false,
                                            Some(if error.is_timeout() {
                                                "timeout"
                                            } else {
                                                "connection"
                                            }),
                                        );
                                        break;
                                    }
                                }
                            }
                        };
                        if tokio::time::timeout_at(deadline, read).await.is_err() {
                            finish(false, Some("timeout"));
                        }
                    }
                }
                result.map_err(|mut e| {
                    CAPTURE.with(|capture| {
                        let mut state = capture.borrow_mut();
                        e.details.provider.get_or_insert(provider);
                        if e.details.country.is_none() {
                            e.details.country = country(provider).map(str::to_owned);
                        }
                        e.details.upstream_http_status =
                            state.status.or(e.details.upstream_http_status);
                        e.details.response_content_type = state.content_type.take();
                        if state.raw {
                            flush_body(&mut state);
                            e.details.debug = Some(HttpDiagnostics {
                                exchanges: std::mem::take(&mut state.exchanges),
                            });
                        }
                    });
                    e
                })
            },
        )
        .await
}
// Preserve a bounded prefix only when the transport limit itself interrupts capture.
fn record_chunk(chunk: &[u8]) -> bool {
    CAPTURE
        .try_with(|s| {
            let mut s = s.borrow_mut();
            let limit = s.limit;
            let Some(_) = s.exchanges.last().and_then(|e| e.response.as_ref()) else {
                return true;
            };
            let remaining = limit.saturating_sub(s.body_bytes.len());
            s.body_bytes
                .extend_from_slice(&chunk[..chunk.len().min(remaining)]);
            let response = s
                .exchanges
                .last_mut()
                .and_then(|e| e.response.as_mut())
                .expect("response checked");
            if chunk.len() > remaining {
                response.complete = false;
                response.incomplete_reason = Some("size_limit");
                false
            } else {
                true
            }
        })
        .unwrap_or(true)
}
fn flush_body(state: &mut State) {
    if let Some(response) = state.exchanges.last_mut().and_then(|e| e.response.as_mut()) {
        response.body = CapturedBytes::new(&state.body_bytes);
    }
}
fn finish(complete: bool, reason: Option<&'static str>) {
    let _ = CAPTURE.try_with(|s| {
        if let Some(response) = s
            .borrow_mut()
            .exchanges
            .last_mut()
            .and_then(|e| e.response.as_mut())
            && response.incomplete_reason != Some("size_limit")
        {
            response.complete = complete;
            response.incomplete_reason = reason;
        }
    });
}
fn country(provider: Provider) -> Option<&'static str> {
    use Provider::*;
    Some(match provider {
        Vies => return None,
        BulgarianCommercialRegister | Bulstat => "BG",
        RomanianAnaf => "RO",
        CzechAres => "CZ",
        AustrianFirmenbuch => "AT",
        BelgianNbb => "BE",
        GreekGemi => "GR",
        FinnishPrh => "FI",
        FrenchRechercheEntreprises => "FR",
        IcelandicSkatturinn => "IS",
        IrishCro => "IE",
        LithuanianJar => "LT",
        LatvianOpenData => "LV",
        SlovakRuz => "SK",
        PolishKrs => "PL",
        NorwegianBrreg => "NO",
        EstonianRik => "EE",
    })
}
fn headers(map: &HeaderMap) -> Vec<CapturedHeader> {
    map.iter()
        .map(|(name, value)| CapturedHeader {
            name: name.to_string(),
            value: CapturedBytes::new(value.as_bytes()),
        })
        .collect()
}
fn content_type(map: &HeaderMap) -> Option<String> {
    map.get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned)
}
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}
fn request_capture(request: &reqwest::Request) -> CapturedRequest {
    let bytes = request.body().and_then(|b| b.as_bytes());
    let mut command = format!(
        "curl --globoff --request {} --url {}",
        quote(request.method().as_str()),
        quote(request.url().as_str())
    );
    let mut unavailable = None;
    for (name, value) in request.headers() {
        match std::str::from_utf8(value.as_bytes()) {
            Ok(value) if !value.contains('\0') => {
                // cURL uses a semicolon to send an empty header; a colon removes it.
                let header = if value.is_empty() {
                    format!("{name};")
                } else {
                    format!("{name}: {value}")
                };
                command.push_str(&format!(" --header {}", quote(&header)))
            }
            _ => unavailable = Some("header_not_representable_in_shell"),
        }
    }
    if let Some(body) = bytes {
        if !request.headers().contains_key("content-type") {
            command.push_str(" --header 'Content-Type:'");
        }
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(body);
        command = format!(
            "printf '%s' {} | base64 --decode | {command} --data-binary @-",
            quote(&encoded)
        );
    } else if request.body().is_some() {
        unavailable = Some("streaming_request_body");
    }
    CapturedRequest {
        method: request.method().to_string(),
        url: request.url().to_string(),
        headers: headers(request.headers()),
        content_type: content_type(request.headers()),
        body: bytes.map(CapturedBytes::new),
        curl: unavailable.is_none().then_some(command),
        curl_unavailable_reason: unavailable,
        capture_layer: "prepared_request",
    }
}
/// Response facade captures bytes as the adapter reads them. Unread error bodies
/// are retained for a bounded diagnostic drain after provider classification.
pub(crate) struct Response {
    inner: Option<reqwest::Response>,
}
impl Response {
    pub(crate) fn status(&self) -> reqwest::StatusCode {
        self.inner.as_ref().expect("live response").status()
    }
    pub(crate) fn headers(&self) -> &HeaderMap {
        self.inner.as_ref().expect("live response").headers()
    }
    pub(crate) fn content_length(&self) -> Option<u64> {
        self.inner.as_ref().expect("live response").content_length()
    }
    pub(crate) async fn chunk(&mut self) -> Result<Option<Vec<u8>>, reqwest::Error> {
        match self.inner.as_mut().expect("live response").chunk().await {
            Ok(Some(chunk)) => {
                record_chunk(&chunk);
                Ok(Some(chunk.to_vec()))
            }
            Ok(None) => {
                finish(true, None);
                Ok(None)
            }
            Err(error) => {
                finish(
                    false,
                    Some(if error.is_timeout() {
                        "timeout"
                    } else {
                        "connection"
                    }),
                );
                Err(error)
            }
        }
    }
}
impl Drop for Response {
    fn drop(&mut self) {
        let _ = CAPTURE.try_with(|s| {
            let mut s = s.borrow_mut();
            if s.raw
                && s.exchanges
                    .last()
                    .and_then(|e| e.response.as_ref())
                    .is_some_and(|r| !r.complete && r.incomplete_reason == Some("interrupted"))
            {
                s.pending = self.inner.take();
            }
        });
    }
}
pub(crate) trait SendTraced {
    async fn send_traced(self) -> Result<Response, reqwest::Error>;
}
impl SendTraced for reqwest::RequestBuilder {
    async fn send_traced(self) -> Result<Response, reqwest::Error> {
        let (client, request) = self.build_split();
        let mut request = request?;
        // Configure client defaults explicitly so captured values agree with execution.
        if !request.headers().contains_key("user-agent") {
            request.headers_mut().insert(
                "user-agent",
                concat!("eu-company-lookup/", env!("CARGO_PKG_VERSION"))
                    .parse()
                    .expect("static agent"),
            );
        }
        if !request.headers().contains_key("accept") {
            request
                .headers_mut()
                .insert("accept", "*/*".parse().expect("static accept"));
        }
        let raw = CAPTURE
            .try_with(|s| {
                let mut s = s.borrow_mut();
                s.status = None;
                s.content_type = None;
                s.pending = None;
                if s.raw {
                    flush_body(&mut s);
                    s.body_bytes.clear();
                    s.exchanges.push(HttpExchange {
                        request: request_capture(&request),
                        response: None,
                    });
                }
                s.raw
            })
            .unwrap_or(false);
        let response = client.execute(request).await?;
        let _ = CAPTURE.try_with(|s| {
            let mut s = s.borrow_mut();
            s.status = Some(response.status().as_u16());
            s.content_type = content_type(response.headers());
            if raw {
                s.exchanges.last_mut().unwrap().response = Some(CapturedResponse {
                    http_status: response.status().as_u16(),
                    headers: headers(response.headers()),
                    content_type: content_type(response.headers()),
                    body: CapturedBytes::new(&[]),
                    complete: false,
                    incomplete_reason: Some("interrupted"),
                });
            }
        });
        Ok(Response {
            inner: Some(response),
        })
    }
}

#[cfg(test)]
#[path = "http_capture_tests.rs"]
mod tests;
