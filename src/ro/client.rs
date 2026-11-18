use super::{RoBusinessId, RoCompanyLookupResult, decode};
use crate::http_capture::SendTraced;
use crate::registry_transport::{self, Admission, Bounds, Spacing, network};
use crate::{LookupError, Provider, RegistryFailure};
use jiff::{Timestamp, civil::Date};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

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
    /// Capture raw HTTP requests/responses in errors. Default: false; may expose sensitive data.
    pub raw_debug: bool,
}
impl Default for RoRegistryConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(3),
            start_interval: Duration::from_secs(2),
            failure_cooldown: Duration::from_secs(60),
            max_response_bytes: 1_048_576,
            raw_debug: false,
        }
    }
}
impl RoRegistryConfig {
    fn bounds(&self) -> Bounds {
        Bounds {
            timeout: self.timeout,
            connect_timeout: self.connect_timeout,
            start_interval: self.start_interval,
            failure_cooldown: self.failure_cooldown,
            max_response_bytes: self.max_response_bytes,
            raw_debug: self.raw_debug,
        }
    }
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
        let http = config.bounds().client("invalid Romanian registry bounds")?;
        Ok(Self {
            http,
            config,
            admission: Arc::new(Admission::new()),
            endpoint: ENDPOINT.into(),
            #[cfg(test)]
            captured: Arc::new(std::sync::Mutex::new(serde_json::Value::Null)),
        })
    }
    /// Look up a CUI for today's date in Europe/Bucharest.
    /// A source-specific miss does not establish absence from all Romanian registers.
    pub async fn lookup(&self, value: &str) -> Result<RoCompanyLookupResult, LookupError> {
        self.lookup_at(value, fiscal_date_at(Timestamp::now()))
            .await
    }
    /// Look up fiscal status for an explicit date. Future dates are rejected locally.
    /// ANAF's historical status fields do not imply a historical company extract.
    pub async fn lookup_at(
        &self,
        value: &str,
        date: Date,
    ) -> Result<RoCompanyLookupResult, LookupError> {
        crate::http_capture::scope(
            self.config.raw_debug,
            self.config.max_response_bytes,
            self.config.timeout,
            Provider::RomanianAnaf,
            async {
                let id = RoBusinessId::parse(value)?;
                if date > fiscal_date_at(Timestamp::now())
                    || date.strftime("%Y-%m-%d").to_string().len() != 10
                {
                    return Err(LookupError::from(crate::model::LookupFailure::InvalidInput));
                }
                registry_transport::execute_in_scope(
                    &self.admission,
                    &self.config.bounds(),
                    Provider::RomanianAnaf,
                    Spacing::FromStart,
                    self.send(&id, date),
                )
                .await?
                .ok_or(LookupError::from(
                    crate::model::LookupFailure::RegistryNotFound {
                        provider: Provider::RomanianAnaf,
                    },
                ))
                .and_then(crate::model::company_validation::ensure_resolved)
            },
        )
        .await
    }
    async fn send(
        &self,
        id: &RoBusinessId,
        date: Date,
    ) -> Result<Option<RoCompanyLookupResult>, RegistryFailure> {
        let body = serde_json::json!([{"cui": id.as_str().parse::<u64>().expect("validated CUI"), "data": date.to_string()}]).to_string();
        let response = self
            .http
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(body)
            .send_traced()
            .await
            .map_err(network)?;
        #[cfg(test)]
        {
            *self.captured.lock().unwrap() = serde_json::json!({"httpStatus":response.status().as_u16(), "contentType":response.headers().get("content-type").and_then(|v|v.to_str().ok()), "retryAfter":response.headers().get("retry-after").and_then(|v|v.to_str().ok())});
        }
        registry_transport::check_status(
            response.status().as_u16(),
            registry_transport::retry_after(&response),
            &[200],
        )?;
        let bytes = registry_transport::json_body(response, self.config.max_response_bytes).await?;
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
#[cfg(test)]
impl RoRegistryClient {
    pub(super) fn begin_capture(&self) {
        *self.captured.lock().unwrap() = serde_json::Value::Null;
    }
    pub(super) fn take_capture(&self) -> serde_json::Value {
        self.captured.lock().unwrap().take()
    }
}

// ANAF dates follow the provider's calendar day, independent of the host timezone.
pub(super) fn fiscal_date_at(now: Timestamp) -> Date {
    now.in_tz("Europe/Bucharest")
        .expect("bundled Bucharest timezone")
        .date()
}
