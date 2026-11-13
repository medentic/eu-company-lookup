use super::{RoBusinessId, RoCompanyLookupResult, decode};
use crate::{LookupError, Provider, RegistryError, RetryAfter, UnavailabilityReason};
use chrono::{NaiveDate, Utc};
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::{sync::Mutex, time::Instant};

const ENDPOINT: &str = "https://webservicesp.anaf.ro/api/PlatitorTvaRest/v9/tva";

/// ANAF transport bounds, independent of VIES and Bulgarian registry limits.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RoRegistryConfig {
    /// Total deadline including admission waiting. Default: 30 seconds.
    pub timeout: Duration,
    /// Connection timeout. Default: 3 seconds.
    pub connect_timeout: Duration,
    /// Request start spacing. Default: 2 seconds; minimum: ANAF's 1 second.
    pub start_interval: Duration,
    /// Cooldown on availability failures or throttling. Default: 60 seconds.
    pub failure_cooldown: Duration,
    /// Maximum response size. Default: 1 MiB; maximum: 4 MiB.
    pub max_response_bytes: usize,
}
impl Default for RoRegistryConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(3),
            start_interval: Duration::from_secs(2),
            failure_cooldown: Duration::from_secs(60),
            max_response_bytes: 1_048_576,
        }
    }
}
struct Admission {
    next: Instant,
    cooldown: Option<Instant>,
}

/// Single-CUI ANAF client with admission shared across clones. No automatic
/// retries, redirects, ambient proxies, cookies or persistent cache.
#[derive(Clone)]
pub struct RoRegistryClient {
    http: reqwest::Client,
    config: RoRegistryConfig,
    admission: Arc<Mutex<Admission>>,
    endpoint: String,
    #[cfg(test)]
    captured: Arc<std::sync::Mutex<serde_json::Value>>,
}
impl std::fmt::Debug for RoRegistryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RoRegistryClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}
impl RoRegistryClient {
    /// Construct a client with bounded defaults.
    pub fn new() -> Result<Self, LookupError> {
        Self::with_config(RoRegistryConfig::default())
    }
    /// Construct with explicit bounds. ANAF request spacing must be at least one second.
    pub fn with_config(config: RoRegistryConfig) -> Result<Self, LookupError> {
        if config.timeout.is_zero()
            || config.timeout > Duration::from_secs(300)
            || config.connect_timeout.is_zero()
            || config.connect_timeout > config.timeout
            || config.start_interval < Duration::from_secs(1)
            || config.start_interval > Duration::from_secs(86400)
            || config.failure_cooldown.is_zero()
            || config.failure_cooldown > Duration::from_secs(86400)
            || !(1..=4_194_304).contains(&config.max_response_bytes)
        {
            return Err(LookupError::InvalidConfiguration(
                "invalid Romanian registry bounds",
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
        Ok(Self {
            http,
            config,
            admission: Arc::new(Mutex::new(Admission {
                next: Instant::now(),
                cooldown: None,
            })),
            endpoint: ENDPOINT.into(),
            #[cfg(test)]
            captured: Arc::new(std::sync::Mutex::new(serde_json::Value::Null)),
        })
    }
    /// Look up a CUI for today's date in Europe/Bucharest.
    /// A source-specific miss does not establish absence from all Romanian registers.
    pub async fn lookup(&self, value: &str) -> Result<RoCompanyLookupResult, LookupError> {
        self.lookup_at(
            value,
            Utc::now()
                .with_timezone(&chrono_tz::Europe::Bucharest)
                .date_naive(),
        )
        .await
    }
    /// Look up fiscal status for an explicit date. Future dates are rejected locally.
    /// ANAF's historical status fields do not imply a historical company extract.
    pub async fn lookup_at(
        &self,
        value: &str,
        date: NaiveDate,
    ) -> Result<RoCompanyLookupResult, LookupError> {
        let id = RoBusinessId::parse(value)?;
        if date
            > Utc::now()
                .with_timezone(&chrono_tz::Europe::Bucharest)
                .date_naive()
            || date.format("%Y-%m-%d").to_string().len() != 10
        {
            return Err(LookupError::InvalidInput);
        }
        let deadline = Instant::now() + self.config.timeout;
        let mut state = self.admission.try_lock().map_err(|_| LookupError::Busy)?;
        if let Some(end) = state.cooldown.filter(|end| *end > Instant::now()) {
            let left = end.saturating_duration_since(Instant::now());
            return Err(wrap(RegistryError::CoolingDown {
                retry_after_seconds: left.as_secs() + u64::from(left.subsec_nanos() > 0),
            }));
        }
        if state.next >= deadline {
            return Err(LookupError::Busy);
        }
        tokio::time::sleep_until(state.next).await;
        if Instant::now() >= deadline {
            return Err(LookupError::Busy);
        }
        state.next = Instant::now() + self.config.start_interval;
        let result = tokio::time::timeout_at(deadline, self.send(&id, date))
            .await
            .unwrap_or_else(|_| Err(unavailable(UnavailabilityReason::Timeout, None)));
        if let Err(
            RegistryError::ServiceUnavailable { retry_after, .. }
            | RegistryError::RateLimited { retry_after },
        ) = &result
        {
            let delay = retry_after
                .as_ref()
                .map(|retry| match retry {
                    RetryAfter::Seconds(s) => Duration::from_secs(*s),
                    RetryAfter::Date(d) => SystemTime::from(*d)
                        .duration_since(SystemTime::now())
                        .unwrap_or_default(),
                })
                .unwrap_or_default()
                .max(self.config.failure_cooldown)
                .min(Duration::from_secs(315_360_000));
            state.cooldown = Some(Instant::now() + delay);
        }
        result.map_err(wrap)?.ok_or(LookupError::RegistryNotFound {
            provider: Provider::RomanianAnaf,
        })
    }
    async fn send(
        &self,
        id: &RoBusinessId,
        date: NaiveDate,
    ) -> Result<Option<RoCompanyLookupResult>, RegistryError> {
        let body = serde_json::json!([{"cui": id.as_str().parse::<u64>().expect("validated CUI"), "data": date.to_string()}]).to_string();
        let mut response = self
            .http
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(body)
            .send()
            .await
            .map_err(network)?;
        #[cfg(test)]
        {
            *self.captured.lock().unwrap() = serde_json::json!({"httpStatus":response.status().as_u16(), "contentType":response.headers().get("content-type").and_then(|v|v.to_str().ok()), "retryAfter":response.headers().get("retry-after").and_then(|v|v.to_str().ok())});
        }
        let retry = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| {
                let v = v.trim();
                if !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) {
                    v.parse().ok().map(RetryAfter::Seconds)
                } else {
                    httpdate::parse_http_date(v)
                        .ok()
                        .map(|d| RetryAfter::Date(d.into()))
                }
            });
        match response.status().as_u16() {
            429 => return Err(RegistryError::RateLimited { retry_after: retry }),
            408 => return Err(unavailable(UnavailabilityReason::Timeout, retry)),
            500..=599 => return Err(unavailable(UnavailabilityReason::ServerError, retry)),
            401 | 403 => return Err(RegistryError::AccessDenied),
            200 => {}
            status => return Err(RegistryError::Http { status }),
        }
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .map(str::trim);
        if !content_type.is_some_and(|v| v.eq_ignore_ascii_case("application/json")) {
            return Err(RegistryError::UnexpectedResponse);
        }
        if response
            .content_length()
            .is_some_and(|n| n > self.config.max_response_bytes as u64)
        {
            return Err(RegistryError::ResponseTooLarge);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(network)? {
            if chunk.len() > self.config.max_response_bytes.saturating_sub(bytes.len()) {
                return Err(RegistryError::ResponseTooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        #[cfg(test)]
        {
            self.captured.lock().unwrap()["body"] =
                serde_json::json!(String::from_utf8_lossy(&bytes));
        }
        decode::decode(&bytes, id, date)
    }
    #[cfg(test)]
    pub(crate) fn for_test(endpoint: String, config: RoRegistryConfig) -> Self {
        let mut client = Self::with_config(config).unwrap();
        client.endpoint = endpoint;
        client
    }
}
fn wrap(error: RegistryError) -> LookupError {
    LookupError::Registry {
        provider: Provider::RomanianAnaf,
        error,
    }
}
fn unavailable(reason: UnavailabilityReason, retry_after: Option<RetryAfter>) -> RegistryError {
    RegistryError::ServiceUnavailable {
        reason,
        retry_after,
    }
}
fn network(error: reqwest::Error) -> RegistryError {
    unavailable(
        if error.is_timeout() {
            UnavailabilityReason::Timeout
        } else {
            UnavailabilityReason::Connection
        },
        None,
    )
}

#[cfg(test)]
impl RoRegistryClient {
    pub(super) fn begin_capture(&self) {
        *self.captured.lock().unwrap() = serde_json::Value::Null;
    }
    pub(super) fn take_capture(&self) -> serde_json::Value {
        self.captured.lock().unwrap().take()
    }
}
