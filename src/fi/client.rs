use super::{FiBusinessId, FiCompanyLookupResult, decode};
use crate::http_capture::SendTraced;
use crate::registry_transport::{self, Admission, Bounds, Spacing, network};
use crate::{LookupError, Provider, RegistryFailure};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

const ENDPOINT: &str = "https://avoindata.prh.fi/opendata-ytj-api/v3/companies";

/// PRH YTJ transport bounds, independent of VIES and Bulgarian registry limits.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FiRegistryConfig {
    /// Total deadline including admission waiting. Default: 30 seconds.
    pub timeout: Duration,
    /// Connection timeout. Default: 3 seconds.
    pub connect_timeout: Duration,
    /// Request start spacing. Default: 2 seconds; minimum: 1 second (library policy).
    pub start_interval: Duration,
    /// Cooldown on availability failures or throttling. Default: 60 seconds.
    pub failure_cooldown: Duration,
    /// Maximum response size. Default: 1 MiB; maximum: 4 MiB.
    pub max_response_bytes: usize,
    /// Capture raw HTTP requests/responses in errors. Default: false; may expose sensitive data.
    pub raw_debug: bool,
}
impl Default for FiRegistryConfig {
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
impl FiRegistryConfig {
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

/// Single-business ID PRH YTJ client with admission shared across clones. No automatic
/// retries, redirects, ambient proxies, cookies or persistent cache.
#[derive(Clone)]
pub struct FiRegistryClient {
    http: reqwest::Client,
    config: FiRegistryConfig,
    admission: Arc<Mutex<Admission>>,
    endpoint: String,
    #[cfg(test)]
    captured: Arc<std::sync::Mutex<serde_json::Value>>,
}
impl std::fmt::Debug for FiRegistryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FiRegistryClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}
impl FiRegistryClient {
    /// Construct a client with bounded defaults.
    pub fn new() -> Result<Self, LookupError> {
        Self::with_config(FiRegistryConfig::default())
    }
    /// Construct with explicit bounds; library policy requires at least one-second spacing.
    pub fn with_config(config: FiRegistryConfig) -> Result<Self, LookupError> {
        let http = config.bounds().client("invalid Finnish registry bounds")?;
        Ok(Self {
            http,
            config,
            admission: Arc::new(Admission::new()),
            endpoint: ENDPOINT.into(),
            #[cfg(test)]
            captured: Arc::new(std::sync::Mutex::new(serde_json::Value::Null)),
        })
    }
    /// Look up the current PRH YTJ observation by full business ID.
    pub async fn lookup(&self, value: &str) -> Result<FiCompanyLookupResult, LookupError> {
        let id = FiBusinessId::parse(value)?;
        registry_transport::execute_company(
            &self.admission,
            &self.config.bounds(),
            Provider::FinnishPrh,
            Spacing::FromStart,
            self.send(&id),
        )
        .await
    }
    async fn send(&self, id: &FiBusinessId) -> Result<FiCompanyLookupResult, RegistryFailure> {
        let response = self
            .http
            .get(format!("{}?businessId={}", self.endpoint, id.as_str()))
            .header("Accept", "application/json")
            .send_traced()
            .await
            .map_err(network)?;
        #[cfg(test)]
        {
            *self.captured.lock().unwrap() = serde_json::json!({"httpStatus":response.status().as_u16(), "contentType":response.headers().get("content-type").and_then(|v|v.to_str().ok()), "retryAfter":response.headers().get("retry-after").and_then(|v|v.to_str().ok())});
        }
        let status = response.status().as_u16();
        registry_transport::check_status(
            status,
            registry_transport::retry_after(&response),
            &[200],
        )?;
        let bytes = registry_transport::json_body(response, self.config.max_response_bytes).await?;
        #[cfg(test)]
        {
            self.captured.lock().unwrap()["body"] =
                serde_json::json!(String::from_utf8_lossy(&bytes));
        }
        decode::decode(&bytes, id)
    }
    #[cfg(test)]
    pub(crate) fn for_test(endpoint: String, config: FiRegistryConfig) -> Self {
        let mut client = Self::with_config(config).unwrap();
        client.endpoint = endpoint;
        client
    }
}

#[cfg(test)]
impl FiRegistryClient {
    pub(super) fn begin_capture(&self) {
        *self.captured.lock().unwrap() = serde_json::Value::Null;
    }
    pub(super) fn take_capture(&self) -> serde_json::Value {
        self.captured.lock().unwrap().take()
    }
}
