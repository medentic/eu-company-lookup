use super::{AtBusinessId, AtCompanyLookupResult, decode};
use crate::http_capture::SendTraced;
use crate::registry_transport::{self, Admission, Bounds, Spacing, network};
use crate::{LookupError, Provider, RegistryFailure};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

const ENDPOINT: &str = "https://justizonline.gv.at/jop/service/fba";

/// Firmenbuch transport bounds, independent of VIES and Bulgarian registry limits.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AtRegistryConfig {
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
impl Default for AtRegistryConfig {
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
impl AtRegistryConfig {
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

/// Single-number Firmenbuch client with admission shared across clones. No automatic
/// retries, redirects, ambient proxies, cookies or persistent cache.
#[derive(Clone)]
pub struct AtRegistryClient {
    http: reqwest::Client,
    config: AtRegistryConfig,
    admission: Arc<Mutex<Admission>>,
    endpoint: String,
    #[cfg(test)]
    captured: Arc<std::sync::Mutex<serde_json::Value>>,
}
impl std::fmt::Debug for AtRegistryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AtRegistryClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}
impl AtRegistryClient {
    /// Construct a client with bounded defaults.
    pub fn new() -> Result<Self, LookupError> {
        Self::with_config(AtRegistryConfig::default())
    }
    /// Construct with explicit bounds; library policy requires at least one-second spacing.
    pub fn with_config(config: AtRegistryConfig) -> Result<Self, LookupError> {
        let http = config.bounds().client("invalid Austrian registry bounds")?;
        Ok(Self {
            http,
            config,
            admission: Arc::new(Admission::new()),
            endpoint: ENDPOINT.into(),
            #[cfg(test)]
            captured: Arc::new(std::sync::Mutex::new(serde_json::Value::Null)),
        })
    }
    /// Look up the current Firmenbuch observation by full Firmenbuch number.
    pub async fn lookup(&self, value: &str) -> Result<AtCompanyLookupResult, LookupError> {
        let id = AtBusinessId::parse(value)?;
        registry_transport::execute(
            &self.admission,
            &self.config.bounds(),
            Provider::AustrianFirmenbuch,
            Spacing::AfterCompletion,
            self.send(&id),
        )
        .await
    }
    async fn send(&self, id: &AtBusinessId) -> Result<AtCompanyLookupResult, RegistryFailure> {
        let search_url = format!(
            "{}/search?term={}&size=5&page=0",
            self.endpoint,
            id.as_str().to_ascii_lowercase()
        );
        let search = self.get_json(&search_url).await?;
        let record = decode::select(&search, id)?;
        tokio::time::sleep(self.config.start_interval).await;
        let detail = self
            .get_json(&format!("{}/{}", self.endpoint, record))
            .await?;
        decode::detail(&detail, id, &record)
    }
    async fn get_json(&self, url: &str) -> Result<Vec<u8>, RegistryFailure> {
        let response = self
            .http
            .get(url)
            .header("Accept", "application/json")
            .send_traced()
            .await
            .map_err(network)?;
        #[cfg(test)]
        {
            self.captured.lock().unwrap()["last"] = serde_json::json!({"httpStatus":response.status().as_u16(), "contentType":response.headers().get("content-type").and_then(|v|v.to_str().ok()), "retryAfter":response.headers().get("retry-after").and_then(|v|v.to_str().ok())});
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
            let mut capture = self.captured.lock().unwrap();
            capture["last"]["body"] = serde_json::json!(String::from_utf8_lossy(&bytes));
            let row = capture["last"].take();
            if capture["responses"].is_null() {
                capture["responses"] = serde_json::json!([]);
            }
            capture["responses"].as_array_mut().unwrap().push(row);
        }
        Ok(bytes)
    }
    #[cfg(test)]
    pub(crate) fn for_test(endpoint: String, config: AtRegistryConfig) -> Self {
        let mut client = Self::with_config(config).unwrap();
        client.endpoint = endpoint;
        client
    }
}

#[cfg(test)]
impl AtRegistryClient {
    pub(super) fn begin_capture(&self) {
        *self.captured.lock().unwrap() = serde_json::Value::Null;
    }
    pub(super) fn take_capture(&self) -> serde_json::Value {
        self.captured.lock().unwrap().take()
    }
}
