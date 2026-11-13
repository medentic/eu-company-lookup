use super::{VatId, capabilities, identifiers::FORMATS, soap, validate_vat_syntax};
use crate::{JurisdictionCapabilities, LimitScope, LookupError, Provider, RetryAfter, VatCheck};
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

const ENDPOINT: &str = "https://ec.europa.eu/taxation_customs/vies/services/checkVatService";

/// Transport and admission settings. Limits are shared across client clones,
/// not across separately constructed clients or application processes.
#[derive(Debug, Clone)]
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
    fn check(&self, scope: LimitScope) -> Result<(), LookupError> {
        if let Some(end) = self.cooldown.filter(|end| *end > Instant::now()) {
            let left = end.saturating_duration_since(Instant::now());
            return Err(LookupError::CoolingDown {
                scope,
                retry_after_seconds: left.as_secs() + u64::from(left.subsec_nanos() > 0),
            });
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

/// Cloneable asynchronous VAT client. Use inside a Tokio runtime with I/O and time
/// enabled. Reuse one client so limits and cooldowns are shared. No automatic
/// retries, redirects, persistent cache, ambient proxy or cookies are used.
#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    config: ClientConfig,
    state: Arc<State>,
    endpoint: String,
}

impl Client {
    /// Build a client with bounded default settings.
    pub fn new() -> Result<Self, LookupError> {
        Self::with_config(ClientConfig::default())
    }

    /// Build with explicit deadlines and limits. Zero concurrency, response limit,
    /// timeout or cooldown and excessively large values are rejected.
    pub fn with_config(config: ClientConfig) -> Result<Self, LookupError> {
        if config.timeout.is_zero()
            || config.timeout > Duration::from_secs(300)
            || config.connect_timeout.is_zero()
            || config.connect_timeout > config.timeout
        {
            return Err(LookupError::InvalidConfiguration(
                "timeouts must be positive, connect <= total <= 300 seconds",
            ));
        }
        if !(1..=64).contains(&config.max_concurrent_requests)
            || !(1..=1_048_576).contains(&config.max_response_bytes)
        {
            return Err(LookupError::InvalidConfiguration(
                "concurrency must be 1..=64 and response limit 1..=1048576",
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
            return Err(LookupError::InvalidConfiguration(
                "intervals must be <= 24 hours and cooldown must be positive",
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
            .map_err(|_| LookupError::ClientInitialization)?;
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

    /// Return implemented capabilities without network calls. Enabled support is
    /// distinct from live source availability; VIES has no language parameter.
    pub fn capabilities(&self) -> Vec<JurisdictionCapabilities> {
        capabilities::catalog()
    }

    /// Check one VAT ID. `valid: false` is returned only for a matching negative
    /// VIES response. Capacity, service failures and deadlines remain errors.
    pub async fn lookup_vat(&self, vat_id: &str) -> Result<VatCheck, LookupError> {
        let vat = validate_vat_syntax(vat_id)?;
        tokio::time::timeout(self.config.timeout, self.check(&vat))
            .await
            .map_err(|_| LookupError::DeadlineExceeded)?
    }

    /// Convenience wrapper around [`Self::lookup_vat`]. Do not call both for one
    /// interaction: the richer result already contains the validity flag.
    pub async fn is_vat_valid(&self, vat_id: &str) -> Result<bool, LookupError> {
        Ok(self.lookup_vat(vat_id).await?.valid)
    }

    async fn check(&self, vat: &VatId) -> Result<VatCheck, LookupError> {
        let _permit = self
            .state
            .capacity
            .try_acquire()
            .map_err(|_| LookupError::Busy)?;
        let mut country = self.state.countries[vat.prefix()]
            .try_lock()
            .map_err(|_| LookupError::Busy)?;
        country.check(LimitScope::MemberState)?;
        // Check the global cooldown before waiting, and again immediately before sending.
        self.state.global.lock().await.check(LimitScope::Global)?;
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
            // An in-flight response can establish a cooldown while we sleep.
            // Recheck both spacing and cooldown after waking.
            tokio::time::sleep_until(start).await;
        }
        country.next_start = Instant::now() + self.config.country_start_interval;
        let result = self.send(vat).await;
        let scope = match &result {
            Err(LookupError::RateLimited { scope, .. }) => Some(*scope),
            Err(
                LookupError::MemberStateUnavailable { .. } | LookupError::ProviderTimeout { .. },
            ) => Some(LimitScope::MemberState),
            Err(LookupError::ServiceUnavailable { .. } | LookupError::Http { status: 503, .. }) => {
                Some(LimitScope::Global)
            }
            _ => None,
        };
        if let Some(scope) = scope {
            let retry = match &result {
                Err(
                    LookupError::RateLimited { retry_after, .. }
                    | LookupError::Http { retry_after, .. }
                    | LookupError::ServiceUnavailable { retry_after }
                    | LookupError::MemberStateUnavailable { retry_after }
                    | LookupError::ProviderTimeout { retry_after },
                ) => retry_after.as_ref(),
                _ => None,
            };
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

    async fn send(&self, vat: &VatId) -> Result<VatCheck, LookupError> {
        let mut response = self
            .http
            .post(&self.endpoint)
            .header("Content-Type", "text/xml; charset=utf-8")
            .header("SOAPAction", "\"\"")
            .body(soap::request(vat))
            .send()
            .await
            .map_err(transport_error)?;
        let status = response.status();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(parse_retry_after);
        // A 429 is already authoritative; do not spend time downloading its body.
        if status.as_u16() == 429 {
            return Err(LookupError::RateLimited {
                scope: LimitScope::Global,
                retry_after,
            });
        }
        if response
            .content_length()
            .is_some_and(|n| n > self.config.max_response_bytes as u64)
        {
            return Err(LookupError::ResponseTooLarge);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(transport_error)? {
            if chunk.len() > self.config.max_response_bytes.saturating_sub(bytes.len()) {
                return Err(LookupError::ResponseTooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        let decoded = std::str::from_utf8(&bytes)
            .map_err(|_| LookupError::MalformedResponse)
            .and_then(|xml| soap::decode(xml, vat));
        let decoded = match decoded {
            Err(LookupError::RateLimited { scope, .. }) => {
                return Err(LookupError::RateLimited { scope, retry_after });
            }
            Err(e @ (LookupError::ProviderRejectedInput | LookupError::UnknownSoapFault)) => {
                return Err(e);
            }
            Err(LookupError::ServiceUnavailable { .. }) => {
                return Err(LookupError::ServiceUnavailable { retry_after });
            }
            Err(LookupError::MemberStateUnavailable { .. }) => {
                return Err(LookupError::MemberStateUnavailable { retry_after });
            }
            Err(LookupError::ProviderTimeout { .. }) => {
                return Err(LookupError::ProviderTimeout { retry_after });
            }
            other => other,
        };
        if !status.is_success() {
            return Err(LookupError::Http {
                status: status.as_u16(),
                retry_after,
            });
        }
        let decoded = decoded?;
        Ok(VatCheck {
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

fn transport_error(error: reqwest::Error) -> LookupError {
    if error.is_timeout() {
        LookupError::DeadlineExceeded
    } else {
        LookupError::Transport
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
impl Client {
    pub(super) fn for_test(endpoint: String, config: ClientConfig) -> Self {
        let mut client = Self::with_config(config).unwrap();
        client.endpoint = endpoint;
        client
    }
}
