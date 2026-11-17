use serde::Serialize;

/// Bytes captured without redaction. UTF-8 text is stored directly; other bytes use base64.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CapturedBytes {
    /// `utf8` or `base64`.
    pub encoding: &'static str,
    /// Lossless representation in the indicated encoding.
    pub data: String,
}
impl CapturedBytes {
    pub(crate) fn new(bytes: &[u8]) -> Self {
        use base64::Engine;
        match std::str::from_utf8(bytes) {
            Ok(text) => Self {
                encoding: "utf8",
                data: text.into(),
            },
            Err(_) => Self {
                encoding: "base64",
                data: base64::engine::general_purpose::STANDARD.encode(bytes),
            },
        }
    }
}
/// One header occurrence. Arrays preserve duplicate names; wire casing/order are not retained.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct CapturedHeader {
    /// Header name as exposed by the HTTP client.
    pub name: String,
    /// Losslessly encoded header value.
    pub value: CapturedBytes,
}
/// Prepared HTTP request, before transport-added headers or HTTP framing.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CapturedRequest {
    /// Request method.
    pub method: String,
    /// Full URL, including the query; never sanitized in raw mode.
    pub url: String,
    /// All prepared header occurrences, including configured defaults.
    pub headers: Vec<CapturedHeader>,
    /// Content-Type when valid UTF-8, otherwise inspect headers.
    pub content_type: Option<String>,
    /// Original request bytes. No body is represented by null.
    pub body: Option<CapturedBytes>,
    /// Shell-quoted reproduction command, not a packet-level wire capture.
    pub curl: Option<String>,
    /// Why cURL cannot represent this request, if applicable.
    pub curl_unavailable_reason: Option<&'static str>,
    /// Capture layer; currently `prepared_request`, not wire-level headers.
    pub capture_layer: &'static str,
}
/// Upstream response as exposed by the HTTP client; no redaction or lossy decoding.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CapturedResponse {
    /// Actual upstream HTTP status.
    pub http_status: u16,
    /// All response header occurrences available from the HTTP client.
    pub headers: Vec<CapturedHeader>,
    /// Content-Type when valid UTF-8, otherwise inspect headers.
    pub content_type: Option<String>,
    /// Captured response bytes, including partial bytes on interrupted reads.
    pub body: CapturedBytes,
    /// True only when end of body was observed.
    pub complete: bool,
    /// `size_limit`, `timeout`, `connection`, or `interrupted` if incomplete.
    pub incomplete_reason: Option<&'static str>,
}
/// One attempted HTTP exchange. A connection failure has no response.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct HttpExchange {
    /// Prepared outgoing request.
    pub request: CapturedRequest,
    /// Received response; null if headers never arrived.
    pub response: Option<CapturedResponse>,
}
/// Ordered exchanges for one lookup. Explicit serialization exposes raw values.
#[derive(Clone, Serialize)]
#[non_exhaustive]
pub struct HttpDiagnostics {
    /// Includes earlier exchanges in multi-request registry workflows.
    pub exchanges: Vec<HttpExchange>,
}
impl std::fmt::Debug for HttpDiagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpDiagnostics")
            .field("exchange_count", &self.exchanges.len())
            .finish_non_exhaustive()
    }
}
