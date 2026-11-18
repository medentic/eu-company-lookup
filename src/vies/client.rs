use super::failure::Failure;
use super::{VatId, identifiers::FORMATS, soap, transport, validate_vat_syntax};
use crate::{LimitScope, LookupError, Provider, RetryAfter, VatLookupResult};
use chrono::Utc;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::{
    sync::{Mutex, Semaphore},
    time::Instant,
};

use super::transport::ENDPOINT;

/// Transport and admission settings. Limits are shared across client clones,
/// not across separately constructed clients or application processes.
///
/// Start from `Default` and assign public fields. Struct literals are intentionally
/// unavailable to downstream crates so new options can be added compatibly.
///
/// ```compile_fail
/// use eu_company_lookup::ClientConfig;
/// let config = ClientConfig { ..Default::default() };
/// ```
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ClientConfig {
    /// Total deadline including spacing, HTTP and body reading. Default: 8 seconds.
    pub timeout: Duration,
    /// Connection deadline, also bounded by the total deadline. Default: 3 seconds.
    pub connect_timeout: Duration,
    /// Maximum active/admitted calls across countries. Default: 2.
    pub max_concurrent_requests: usize,
    /// Minimum spacing between starts across countries. Default: 2 seconds.
    pub global_start_interval: Duration,
    /// Minimum spacing between starts to the same country. Default: 5 seconds.
    pub country_start_interval: Duration,
    /// Cooldown on provider capacity/unavailability without a longer Retry-After.
    /// Default: 60 seconds.
    pub failure_cooldown: Duration,
    /// Maximum response body bytes. Default: 65,536; maximum allowed: 1 MiB.
    pub max_response_bytes: usize,
    /// Capture raw HTTP requests/responses in errors. Default: false; may expose sensitive data.
    pub raw_debug: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(8),
            connect_timeout: Duration::from_secs(3),
            max_concurrent_requests: 2,
            global_start_interval: Duration::from_secs(2),
            country_start_interval: Duration::from_secs(5),
            failure_cooldown: Duration::from_secs(60),
            max_response_bytes: 65_536,
            raw_debug: false,
        }
    }
}

struct Admission {
    next_start: Instant,
    cooldown: Option<Instant>,
}
impl Admission {
    fn new() -> Self {
        Self {
            next_start: Instant::now(),
            cooldown: None,
        }
    }
    fn check(&self, scope: LimitScope) -> Result<(), Failure> {
        if let Some(end) = self.cooldown.filter(|end| *end > Instant::now()) {
            let left = end.saturating_duration_since(Instant::now());
            return Err(Failure::from(crate::model::LookupFailure::CoolingDown {
                scope,
                retry_after_seconds: left.as_secs() + u64::from(left.subsec_nanos() > 0),
            }));
        }
        Ok(())
    }
    fn cool(&mut self, duration: Duration) {
        // Preserve exact Retry-After in the error; cap internal arithmetic at ten years.
        let end = Instant::now() + duration.min(Duration::from_secs(315_360_000));
        self.cooldown = Some(self.cooldown.map_or(end, |previous| previous.max(end)));
    }
}
struct State {
    capacity: Semaphore,
    global: Mutex<Admission>,
    countries: HashMap<&'static str, Mutex<Admission>>,
}

/// Internal VIES transport client. Use inside a Tokio runtime with I/O and time
/// enabled. Reuse one client so limits and cooldowns are shared. No automatic
/// retries, redirects, persistent cache, ambient proxy or cookies are used.
#[derive(Clone)]
pub(crate) struct ViesClient {
    http: reqwest::Client,
    config: ClientConfig,
    state: Arc<State>,
    endpoint: String,
}

impl std::fmt::Debug for ViesClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ViesClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl ViesClient {
    pub(crate) fn with_config(config: ClientConfig) -> Result<Self, LookupError> {
        if config.timeout.is_zero()
            || config.timeout > Duration::from_secs(300)
            || config.connect_timeout.is_zero()
            || config.connect_timeout > config.timeout
        {
            return Err(LookupError::from(
                crate::model::LookupFailure::InvalidConfiguration(
                    "timeouts must be positive, connect <= total <= 300 seconds",
                ),
            ));
        }
        if !(1..=64).contains(&config.max_concurrent_requests)
            || !(1..=1_048_576).contains(&config.max_response_bytes)
        {
            return Err(LookupError::from(
                crate::model::LookupFailure::InvalidConfiguration(
                    "concurrency must be 1..=64 and response limit 1..=1048576",
                ),
            ));
        }
        if config.failure_cooldown.is_zero()
            || [
                config.global_start_interval,
                config.country_start_interval,
                config.failure_cooldown,
            ]
            .iter()
            .any(|d| *d > Duration::from_secs(86_400))
        {
            return Err(LookupError::from(
                crate::model::LookupFailure::InvalidConfiguration(
                    "intervals must be <= 24 hours and cooldown must be positive",
                ),
            ));
        }
        let http = transport::client(config.connect_timeout, Some(config.timeout))
            .map_err(|_| LookupError::from(crate::model::LookupFailure::ClientInitialization))?;
        let state = Arc::new(State {
            capacity: Semaphore::new(config.max_concurrent_requests),
            global: Mutex::new(Admission::new()),
            countries: FORMATS
                .iter()
                .map(|(prefix, _)| (*prefix, Mutex::new(Admission::new())))
                .collect(),
        });
        Ok(Self {
            http,
            config,
            state,
            endpoint: ENDPOINT.into(),
        })
    }

    /// Look up VAT validity and available company identity/address details.
    /// `valid: false` is returned only for a matching negative
    /// VIES response. Capacity, service failures and deadlines remain errors.
    pub async fn lookup_vat(&self, vat_id: &str) -> Result<VatLookupResult, LookupError> {
        crate::http_capture::scope(
            self.config.raw_debug,
            self.config.max_response_bytes,
            self.config.timeout,
            Provider::Vies,
            async {
                let vat = validate_vat_syntax(vat_id)?;
                super::checksum::validate(&vat)?;
                self.lookup_with_deadline(&vat, Instant::now() + self.config.timeout)
                    .await
                    .map_err(LookupError::from)
            },
        )
        .await
        .map_err(|mut error| {
            if let Ok(vat) = validate_vat_syntax(vat_id) {
                error.details.country = Some(
                    match vat.prefix() {
                        "EL" => "GR",
                        "XI" => "GB",
                        prefix => prefix,
                    }
                    .into(),
                );
            }
            error
        })
    }

    async fn lookup_with_deadline(
        &self,
        vat: &VatId,
        deadline: Instant,
    ) -> Result<VatLookupResult, Failure> {
        let _permit = self
            .state
            .capacity
            .try_acquire()
            .map_err(|_| Failure::from(crate::model::LookupFailure::Busy))?;
        let mut country = self.state.countries[vat.prefix()]
            .try_lock()
            .map_err(|_| Failure::from(crate::model::LookupFailure::Busy))?;
        country.check(LimitScope::MemberState)?;
        // Check the global cooldown before waiting, and again immediately before sending.
        self.state.global.lock().await.check(LimitScope::Global)?;
        if country.next_start >= deadline {
            return Err(Failure::from(crate::model::LookupFailure::Busy));
        }
        tokio::time::sleep_until(country.next_start).await;
        loop {
            let mut global = self.state.global.lock().await;
            global.check(LimitScope::Global)?;
            let now = Instant::now();
            if global.next_start <= now {
                global.next_start = now + self.config.global_start_interval;
                break;
            }
            let start = global.next_start;
            drop(global);
            if start >= deadline {
                return Err(Failure::from(crate::model::LookupFailure::Busy));
            }
            // An in-flight response can establish a cooldown while we sleep.
            // Recheck both spacing and cooldown after waking.
            tokio::time::sleep_until(start).await;
        }
        if Instant::now() >= deadline {
            return Err(Failure::from(crate::model::LookupFailure::Busy));
        }
        country.next_start = Instant::now() + self.config.country_start_interval;
        let result = tokio::time::timeout_at(deadline, self.fetch_vat_result(vat))
            .await
            .unwrap_or(Err(Failure::from(
                crate::model::LookupFailure::DeadlineExceeded,
            )));
        if let Some((scope, retry)) = result.as_ref().err().and_then(Failure::cooldown) {
            let duration = retry
                .map(retry_delay)
                .unwrap_or_default()
                .max(self.config.failure_cooldown);
            match scope {
                LimitScope::Global => self.state.global.lock().await.cool(duration),
                LimitScope::MemberState => country.cool(duration),
            }
        }
        result
    }

    async fn fetch_vat_result(&self, vat: &VatId) -> Result<VatLookupResult, Failure> {
        let response = transport::exchange(
            &self.http,
            &self.endpoint,
            vat,
            self.config.max_response_bytes,
        )
        .await
        .map_err(transport_error)?;
        let status = response.status;
        let retry_after = response.retry_after.as_deref().and_then(parse_retry_after);
        // A 429 is already authoritative; do not spend time downloading its body.
        if status.as_u16() == 429 {
            return Err(Failure::from(crate::model::LookupFailure::RateLimited {
                scope: LimitScope::Global,
                retry_after,
            }));
        }
        // Gateway failures are already availability failures, even with an HTML,
        // empty, oversized or stalled body. No negative VAT result is possible.
        if matches!(status.as_u16(), 408 | 504) {
            return Err(Failure::from(
                crate::model::LookupFailure::ProviderTimeout { retry_after },
            ));
        }
        if status.as_u16() == 502 {
            return Err(Failure::from(
                crate::model::LookupFailure::ServiceUnavailable { retry_after },
            ));
        }
        // 500/503 may contain precise SOAP faults. If their body is incomplete
        // or oversized, the HTTP failure still establishes unavailability.
        let body_error = |error| {
            if status.is_server_error() {
                Failure::from(crate::model::LookupFailure::ServiceUnavailable {
                    retry_after: retry_after.clone(),
                })
            } else {
                error
            }
        };
        let decoded = match response.body {
            transport::Body::Text(xml) => soap::decode(&xml, vat),
            transport::Body::Failed(transport::Failure::InvalidUtf8) => Err(Failure::from(
                crate::model::LookupFailure::MalformedResponse,
            )),
            transport::Body::Failed(error) => return Err(body_error(transport_error(error))),
            transport::Body::Skipped => unreachable!("authoritative statuses handled above"),
        };
        let decoded = match decoded {
            Err(error) if error.is_protocol_fault() => {
                return Err(error.with_retry_after(retry_after));
            }
            other => other,
        };
        if !status.is_success() {
            if status.is_server_error() {
                return Err(Failure::from(
                    crate::model::LookupFailure::ServiceUnavailable { retry_after },
                ));
            }
            return Err(Failure::from(crate::model::LookupFailure::Http {
                status: status.as_u16(),
                retry_after,
            }));
        }
        let decoded = decoded?;
        Ok(VatLookupResult {
            vat_id: vat.clone(),
            valid: decoded.valid,
            request_date: decoded.request_date,
            checked_at: Utc::now(),
            company: decoded.company,
            address: super::address::split(vat.prefix(), decoded.address),
            source: Provider::Vies,
        })
    }
}

fn transport_error(error: transport::Failure) -> Failure {
    match error {
        transport::Failure::Timeout => Failure::from(crate::model::LookupFailure::DeadlineExceeded),
        transport::Failure::Network => Failure::from(crate::model::LookupFailure::Transport),
        transport::Failure::TooLarge => {
            Failure::from(crate::model::LookupFailure::ResponseTooLarge)
        }
        transport::Failure::InvalidUtf8 => {
            Failure::from(crate::model::LookupFailure::MalformedResponse)
        }
    }
}
pub(super) fn parse_retry_after(value: &str) -> Option<RetryAfter> {
    let value = value.trim();
    if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
        return value.parse().ok().map(RetryAfter::Seconds);
    }
    httpdate::parse_http_date(value)
        .ok()
        .map(|d| RetryAfter::Date(d.into()))
}
fn retry_delay(value: &RetryAfter) -> Duration {
    match value {
        RetryAfter::Seconds(seconds) => Duration::from_secs(*seconds),
        RetryAfter::Date(date) => SystemTime::from(*date)
            .duration_since(SystemTime::now())
            .unwrap_or_default(),
    }
}

#[cfg(test)]
impl ViesClient {
    pub(crate) fn for_test(endpoint: String, config: ClientConfig) -> Self {
        let mut client = Self::with_config(config).unwrap();
        client.endpoint = endpoint;
        client
    }
}
