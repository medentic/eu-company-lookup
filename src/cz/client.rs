use super::{CzBusinessId, CzCompanyLookupResult, decode};
use crate::http_capture::SendTraced;
use crate::registry_transport::{self, Admission, Bounds, Spacing, network};
use crate::{LookupError, Provider, RegistryFailure};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

const ENDPOINT: &str = "https://ares.gov.cz/ekonomicke-subjekty-v-be/rest/ekonomicke-subjekty";

/// ARES transport bounds, independent of VIES and Bulgarian registry limits.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CzRegistryConfig {
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
impl Default for CzRegistryConfig {
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
impl CzRegistryConfig {
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

/// Single-IČO ARES client with admission shared across clones. No automatic
/// retries, redirects, ambient proxies, cookies or persistent cache.
#[derive(Clone)]
pub struct CzRegistryClient {
    http: reqwest::Client,
    config: CzRegistryConfig,
    admission: Arc<Mutex<Admission>>,
    endpoint: String,
    #[cfg(test)]
    captured: Arc<std::sync::Mutex<serde_json::Value>>,
}
impl std::fmt::Debug for CzRegistryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CzRegistryClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}
impl CzRegistryClient {
    /// Construct a client with bounded defaults.
    pub fn new() -> Result<Self, LookupError> {
        Self::with_config(CzRegistryConfig::default())
    }
    /// Construct with explicit bounds; library policy requires at least one-second spacing.
    pub fn with_config(config: CzRegistryConfig) -> Result<Self, LookupError> {
        let http = config.bounds().client("invalid Czech registry bounds")?;
        Ok(Self {
            http,
            config,
            admission: Arc::new(Admission::new()),
            endpoint: ENDPOINT.into(),
            #[cfg(test)]
            captured: Arc::new(std::sync::Mutex::new(serde_json::Value::Null)),
        })
    }
    /// Look up the current ARES observation by full IČO.
    pub async fn lookup(&self, value: &str) -> Result<CzCompanyLookupResult, LookupError> {
        let id = CzBusinessId::parse(value)?;
        registry_transport::execute(
            &self.admission,
            &self.config.bounds(),
            Provider::CzechAres,
            Spacing::FromStart,
            self.send(&id),
        )
        .await?
        .ok_or(LookupError::from(
            crate::model::LookupFailure::RegistryNotFound {
                provider: Provider::CzechAres,
            },
        ))
    }
    async fn send(
        &self,
        id: &CzBusinessId,
    ) -> Result<Option<CzCompanyLookupResult>, RegistryFailure> {
        let response = self
            .http
            .get(format!("{}/{}", self.endpoint, id.as_str()))
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
            &[200, 404],
        )?;
        let bytes = registry_transport::json_body(response, self.config.max_response_bytes).await?;
        #[cfg(test)]
        {
            self.captured.lock().unwrap()["body"] =
                serde_json::json!(String::from_utf8_lossy(&bytes));
        }
        if status == 404 {
            // ARES documents its typed error envelope for this exact IČO route.
            // An unrelated endpoint 404 or unrecognized error is not absence.
            if decode::is_definitive_miss(&bytes) {
                return Ok(None);
            }
            return Err(RegistryFailure::Http { status });
        }
        decode::decode(&bytes, id)
    }
    #[cfg(test)]
    pub(crate) fn for_test(endpoint: String, config: CzRegistryConfig) -> Self {
        let mut client = Self::with_config(config).unwrap();
        client.endpoint = endpoint;
        client
    }
}

#[cfg(test)]
impl CzRegistryClient {
    pub(super) fn begin_capture(&self) {
        *self.captured.lock().unwrap() = serde_json::Value::Null;
    }
    pub(super) fn take_capture(&self) -> serde_json::Value {
        self.captured.lock().unwrap().take()
    }
}
