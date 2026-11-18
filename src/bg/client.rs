use super::{BgBusinessId, bulstat, commercial};
use crate::http_capture::SendTraced;
use crate::{
    BgCompanyLookupResult, LookupError, Provider, RegistryFailure, RetryAfter, UnavailabilityReason,
};
use jiff::Timestamp;
use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::Instant};

const COMMERCIAL: &str = "https://portal.registryagency.bg/CR/api/Deeds/";

/// Bounds for Bulgarian registry lookup, independent of VIES admission limits.
///
/// Start from `Default` and assign public fields. Struct literals are intentionally
/// unavailable to downstream crates so new options can be added compatibly.
///
/// ```compile_fail
/// use eu_company_lookup::BgRegistryConfig;
/// let config = BgRegistryConfig { ..Default::default() };
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct BgRegistryConfig {
    /// Deadline for the entire lookup, including both registers. Default: 30 seconds.
    pub timeout: Duration,
    /// Connection timeout. Default: 3 seconds.
    pub connect_timeout: Duration,
    /// Optional minimum interval between HTTP requests to each provider. Default: zero.
    /// Requests proceed immediately unless callers opt into pacing or a cooldown applies.
    pub start_interval: Duration,
    /// Cooldown after unavailability or throttling. Default: 60 seconds.
    pub failure_cooldown: Duration,
    /// Maximum bytes per response. Default: 1 MiB; maximum: 4 MiB.
    pub max_response_bytes: usize,
    /// Capture raw HTTP requests/responses in errors. Default: false; may expose sensitive data.
    pub raw_debug: bool,
}
impl Default for BgRegistryConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(3),
            start_interval: Duration::ZERO,
            failure_cooldown: Duration::from_secs(60),
            max_response_bytes: 1_048_576,
            raw_debug: false,
        }
    }
}

struct Admission {
    next: Instant,
    cooldown: Option<Instant>,
}
struct State {
    providers: [Admission; 2],
}

/// Cloneable Bulgarian business lookup client. Commercial Register is tried first;
/// only a definitive miss permits BULSTAT fallback. No retries, redirects, cookie
/// jar, ambient proxy, or persistent cache. One lookup at a time across clones.
#[derive(Clone)]
pub struct BgRegistryClient {
    http: reqwest::Client,
    config: BgRegistryConfig,
    state: Arc<Mutex<State>>,
    commercial_endpoint: String,
    bulstat_endpoint: String,
    #[cfg(test)]
    trace: Arc<std::sync::Mutex<RequestTrace>>,
    #[cfg(test)]
    captured: Arc<std::sync::Mutex<Option<Vec<serde_json::Value>>>>,
}

#[cfg(test)]
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RequestTrace {
    pub admission_wait_ms: u128,
    pub http_ms: Option<u128>,
    pub http_status: Option<u16>,
    pub retry_after: Option<RetryAfter>,
}

struct Response {
    body: Vec<u8>,
    content_type: String,
}

impl std::fmt::Debug for BgRegistryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BgRegistryClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl BgRegistryClient {
    /// Construct with default deadlines and shared admission limits.
    pub fn new() -> Result<Self, LookupError> {
        Self::with_config(BgRegistryConfig::default())
    }

    /// Construct with explicit bounds. Zero or excessive bounds are rejected.
    pub fn with_config(config: BgRegistryConfig) -> Result<Self, LookupError> {
        if config.timeout.is_zero()
            || config.timeout > Duration::from_secs(300)
            || config.connect_timeout.is_zero()
            || config.connect_timeout > config.timeout
            || config.start_interval > Duration::from_secs(86400)
            || config.failure_cooldown.is_zero()
            || config.failure_cooldown > Duration::from_secs(86400)
            || !(1..=4_194_304).contains(&config.max_response_bytes)
        {
            return Err(LookupError::from(
                crate::model::LookupFailure::InvalidConfiguration(
                    "invalid Bulgarian registry bounds",
                ),
            ));
        }
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(config.connect_timeout)
            .timeout(config.timeout)
            .user_agent(concat!("eu-company-lookup/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| LookupError::from(crate::model::LookupFailure::ClientInitialization))?;
        Ok(Self {
            http,
            config,
            state: Arc::new(Mutex::new(State {
                providers: std::array::from_fn(|_| Admission {
                    next: Instant::now(),
                    cooldown: None,
                }),
            })),
            commercial_endpoint: COMMERCIAL.into(),
            #[cfg(test)]
            trace: Arc::new(std::sync::Mutex::new(RequestTrace::default())),
            #[cfg(test)]
            captured: Arc::new(std::sync::Mutex::new(None)),
            bulstat_endpoint: format!("{}preview", bulstat::BASE),
        })
    }

    /// Look up one complete nine- or thirteen-digit ЕИК/BULSTAT code.
    /// `NotFound` means both sources definitively missed; outages never mean absence.
    pub async fn lookup(&self, value: &str) -> Result<BgCompanyLookupResult, LookupError> {
        crate::http_capture::scope(
            self.config.raw_debug,
            self.config.max_response_bytes,
            self.config.timeout,
            Provider::BulgarianCommercialRegister,
            async {
                let id = BgBusinessId::parse(value)?;
                let deadline = Instant::now() + self.config.timeout;
                let mut state = self
                    .state
                    .try_lock()
                    .map_err(|_| LookupError::from(crate::model::LookupFailure::Busy))?;
                let provider = Provider::BulgarianCommercialRegister;
                let mut url = reqwest::Url::parse(&format!("{}{}", self.commercial_endpoint, id))
                    .map_err(|_| {
                    LookupError::from(crate::model::LookupFailure::InvalidConfiguration(
                        "invalid registry endpoint",
                    ))
                })?;
                url.query_pairs_mut()
                    .append_pair("entryDate", &end_of_day(Timestamp::now()))
                    .append_pair("loadFieldsFromAllLegalForms", "false");
                let response = self
                    .fetch(provider, url, &mut state.providers[0], deadline)
                    .await?;
                if !response.body.is_empty() {
                    if response.content_type != "application/json" {
                        return Err(wrap(provider, RegistryFailure::UnexpectedResponse));
                    }
                    return commercial::decode(&response.body, &id)
                        .map_err(|e| wrap(provider, e))
                        .and_then(crate::model::company_validation::ensure_resolved);
                }
                // The portal's getDeed UI treats an empty HTTP 200 response as no result.
                // Whitespace, JSON null, 404, HTML, and all operational failures are not misses.
                if !response.content_type.is_empty() && response.content_type != "application/json"
                {
                    return Err(wrap(provider, RegistryFailure::UnexpectedResponse));
                }
                let provider = Provider::Bulstat;
                let search = self
                    .fetch(
                        provider,
                        self.report_url(&id, false),
                        &mut state.providers[1],
                        deadline,
                    )
                    .await?;
                if search.content_type != "text/html" {
                    return Err(wrap(provider, RegistryFailure::UnexpectedResponse));
                }
                let Some(name) =
                    bulstat::search(&search.body, &id).map_err(|e| wrap(provider, e))?
                else {
                    return Err(LookupError::from(crate::model::LookupFailure::NotFound));
                };
                let detail = self
                    .fetch(
                        provider,
                        self.report_url(&id, true),
                        &mut state.providers[1],
                        deadline,
                    )
                    .await?;
                if detail.content_type != "text/html" {
                    return Err(wrap(provider, RegistryFailure::UnexpectedResponse));
                }
                bulstat::detail(&detail.body, &id, name)
                    .map_err(|e| wrap(provider, e))
                    .and_then(crate::model::company_validation::ensure_resolved)
            },
        )
        .await
    }

    fn report_url(&self, id: &BgBusinessId, detail: bool) -> reqwest::Url {
        let mut url =
            reqwest::Url::parse(&self.bulstat_endpoint).expect("internal registry endpoint");
        url.query_pairs_mut()
            .append_pair(
                "__report",
                &format!(
                    "{}{}.rptdesign",
                    bulstat::BASE,
                    if detail { "v_rpt1" } else { "rpt1" }
                ),
            )
            .append_pair("__format", "html")
            .append_pair("__locale", "bg")
            .append_pair("report_title", "Справка за субект на БУЛСТАТ")
            .append_pair("uic", id.as_str());
        url
    }

    async fn fetch(
        &self,
        provider: Provider,
        url: reqwest::Url,
        admission: &mut Admission,
        deadline: Instant,
    ) -> Result<Response, LookupError> {
        if let Some(end) = admission.cooldown.filter(|end| *end > Instant::now()) {
            let left = end - Instant::now();
            return Err(wrap(
                provider,
                RegistryFailure::CoolingDown {
                    retry_after_seconds: left.as_secs() + u64::from(left.subsec_nanos() > 0),
                },
            ));
        }
        // A locally exhausted admission budget is not evidence of an upstream outage.
        if admission.next >= deadline || Instant::now() >= deadline {
            return Err(LookupError::from(crate::model::LookupFailure::Busy));
        }
        #[cfg(test)]
        let waiting = Instant::now();
        tokio::time::sleep_until(admission.next).await;
        #[cfg(test)]
        {
            *self.trace.lock().unwrap() = RequestTrace {
                admission_wait_ms: waiting.elapsed().as_millis(),
                ..Default::default()
            };
        }
        #[cfg(test)]
        let sending = Instant::now();
        admission.next = Instant::now() + self.config.start_interval;
        #[cfg(test)]
        let request_url = url.to_string();
        let result = tokio::time::timeout_at(deadline, self.send(url))
            .await
            .unwrap_or_else(|_| Err(unavailable(UnavailabilityReason::Timeout, None)));
        #[cfg(test)]
        {
            self.trace.lock().unwrap().http_ms = Some(sending.elapsed().as_millis());
            if let Some(rows) = self.captured.lock().unwrap().as_mut() {
                rows.push(serde_json::json!({
                    "provider":provider, "url":request_url,
                    "trace":self.trace.lock().unwrap().clone(),
                    "body":result.as_ref().ok().map(|r| &r.body),
                    "contentType":result.as_ref().ok().map(|r| &r.content_type),
                    "error":result.as_ref().err().map(ToString::to_string),
                }));
            }
        }
        if let Err(
            RegistryFailure::ServiceUnavailable { retry_after, .. }
            | RegistryFailure::RateLimited { retry_after },
        ) = &result
        {
            let delay = retry_after
                .as_ref()
                .map(retry_delay)
                .unwrap_or_default()
                .max(self.config.failure_cooldown)
                .min(Duration::from_secs(315_360_000));
            admission.cooldown = Some(Instant::now() + delay);
        }
        result.map_err(|e| wrap(provider, e))
    }

    async fn send(&self, url: reqwest::Url) -> Result<Response, RegistryFailure> {
        let mut response = self
            .http
            .get(url)
            .header("Accept-Language", "bg")
            .header("Accept", "application/json, text/html")
            .send_traced()
            .await
            .map_err(transport)?;
        let status = response.status().as_u16();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|h| h.to_str().ok())
            .and_then(parse_retry_after);
        #[cfg(test)]
        {
            let mut trace = self.trace.lock().unwrap();
            trace.http_status = Some(status);
            trace.retry_after = retry_after.clone();
        }
        match status {
            200 => (),
            429 => return Err(RegistryFailure::RateLimited { retry_after }),
            401 | 403 => return Err(RegistryFailure::AccessDenied),
            408 | 504 => return Err(unavailable(UnavailabilityReason::Timeout, retry_after)),
            500..=599 => return Err(unavailable(UnavailabilityReason::ServerError, retry_after)),
            _ => return Err(RegistryFailure::Http { status }),
        }
        if response
            .content_length()
            .is_some_and(|n| n > self.config.max_response_bytes as u64)
        {
            return Err(RegistryFailure::ResponseTooLarge);
        }
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("")
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport)? {
            if chunk.len() > self.config.max_response_bytes.saturating_sub(body.len()) {
                return Err(RegistryFailure::ResponseTooLarge);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(Response { body, content_type })
    }
}

fn wrap(provider: Provider, error: RegistryFailure) -> LookupError {
    LookupError::from(crate::model::LookupFailure::Registry { provider, error })
}
fn unavailable(reason: UnavailabilityReason, retry_after: Option<RetryAfter>) -> RegistryFailure {
    RegistryFailure::ServiceUnavailable {
        reason,
        retry_after,
    }
}
fn transport(error: reqwest::Error) -> RegistryFailure {
    unavailable(
        if error.is_timeout() {
            UnavailabilityReason::Timeout
        } else {
            UnavailabilityReason::Connection
        },
        None,
    )
}
fn parse_retry_after(value: &str) -> Option<RetryAfter> {
    let value = value.trim();
    if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
        return value.parse().ok().map(RetryAfter::Seconds);
    }
    httpdate::parse_http_date(value)
        .ok()
        .and_then(|d| jiff::Timestamp::try_from(d).ok())
        .map(RetryAfter::Date)
}
fn retry_delay(value: &RetryAfter) -> Duration {
    match value {
        RetryAfter::Seconds(seconds) => Duration::from_secs(*seconds),
        RetryAfter::Date(date) => (date.duration_since(jiff::Timestamp::now()))
            .try_into()
            .unwrap_or_default(),
    }
}
pub(super) fn end_of_day(now: Timestamp) -> String {
    let local = now
        .in_tz("Europe/Sofia")
        .expect("bundled Sofia timezone")
        .date()
        .at(23, 59, 59, 999_000_000)
        .in_tz("Europe/Sofia")
        .expect("Sofia end of day is unambiguous");
    local.strftime("%Y-%m-%dT%H:%M:%S%.3f%:z").to_string()
}

#[cfg(test)]
impl BgRegistryClient {
    pub(crate) fn for_test(cr: String, bulstat: String, config: BgRegistryConfig) -> Self {
        let mut client = Self::with_config(config).unwrap();
        client.commercial_endpoint = cr;
        client.bulstat_endpoint = bulstat;
        client
    }
}

#[cfg(test)]
impl BgRegistryClient {
    // Development observation path: same transport/admission policy, no BULSTAT fallback.
    pub(super) fn observation_trace(&self) -> RequestTrace {
        self.trace.lock().unwrap().clone()
    }
    pub(super) async fn observe_commercial(&self, value: &str) -> Result<Vec<u8>, LookupError> {
        *self.trace.lock().unwrap() = RequestTrace::default();
        let id = BgBusinessId::parse(value)?;
        let deadline = Instant::now() + self.config.timeout;
        let mut state = self
            .state
            .try_lock()
            .map_err(|_| LookupError::from(crate::model::LookupFailure::Busy))?;
        let provider = Provider::BulgarianCommercialRegister;
        let mut url =
            reqwest::Url::parse(&format!("{}{}", self.commercial_endpoint, id)).map_err(|_| {
                LookupError::from(crate::model::LookupFailure::InvalidConfiguration(
                    "invalid registry endpoint",
                ))
            })?;
        url.query_pairs_mut()
            .append_pair("entryDate", &end_of_day(Timestamp::now()))
            .append_pair("loadFieldsFromAllLegalForms", "false");
        let response = self
            .fetch(provider, url, &mut state.providers[0], deadline)
            .await?;
        if response.content_type != "application/json"
            && !(response.body.is_empty() && response.content_type.is_empty())
        {
            return Err(wrap(provider, RegistryFailure::UnexpectedResponse));
        }
        Ok(response.body)
    }
}

#[cfg(test)]
impl BgRegistryClient {
    pub(super) fn begin_capture(&self) {
        *self.captured.lock().unwrap() = Some(Vec::new());
        *self.trace.lock().unwrap() = RequestTrace::default();
    }
    pub(super) fn take_capture(&self) -> Vec<serde_json::Value> {
        self.captured.lock().unwrap().take().unwrap_or_default()
    }
}
