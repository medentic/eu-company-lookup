use super::{SkBusinessId, SkCompanyLookupResult, decode};
use crate::http_capture::SendTraced;
use crate::registry_transport::{self, Admission, Bounds, Spacing, network};
use crate::{LookupError, Provider, RegistryFailure};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

const ENDPOINT: &str = "https://www.registeruz.sk/cruz-public/api";

/// Slovak public registry transport bounds, independent of VIES and Bulgarian registry limits.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SkRegistryConfig {
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
impl Default for SkRegistryConfig {
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
impl SkRegistryConfig {
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

/// Single-identifier Slovak public registry client with admission shared across clones. No automatic
/// retries, redirects, ambient proxies, cookies or persistent cache.
#[derive(Clone)]
pub struct SkRegistryClient {
    http: reqwest::Client,
    config: SkRegistryConfig,
    admission: Arc<Mutex<Admission>>,
    endpoint: String,
    rpo_endpoint: String,
    #[cfg(test)]
    captured: Arc<std::sync::Mutex<serde_json::Value>>,
}
impl std::fmt::Debug for SkRegistryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SkRegistryClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}
impl SkRegistryClient {
    /// Construct a client with bounded defaults.
    pub fn new() -> Result<Self, LookupError> {
        Self::with_config(SkRegistryConfig::default())
    }
    /// Construct with explicit bounds; library policy requires at least one-second spacing.
    pub fn with_config(config: SkRegistryConfig) -> Result<Self, LookupError> {
        let http = config.bounds().client("invalid Slovak registry bounds")?;
        Ok(Self {
            http,
            config,
            admission: Arc::new(Admission::new()),
            endpoint: ENDPOINT.into(),
            rpo_endpoint: "https://api.statistics.sk/rpo/v1".into(),
            #[cfg(test)]
            captured: Arc::new(std::sync::Mutex::new(serde_json::Value::Null)),
        })
    }
    /// Look up the current Slovak public registry observation by full business ID.
    pub async fn lookup(&self, value: &str) -> Result<SkCompanyLookupResult, LookupError> {
        let id = SkBusinessId::parse(value)?;
        registry_transport::execute_company(
            &self.admission,
            &self.config.bounds(),
            Provider::SlovakRuz,
            Spacing::AfterCompletion,
            self.send(&id),
        )
        .await
    }
    async fn send(&self, id: &SkBusinessId) -> Result<SkCompanyLookupResult, RegistryFailure> {
        let start = tokio::time::Instant::now();
        let search = self
            .request(
                &format!(
                    "{}/uctovne-jednotky?zmenene-od=2000-01-01&ico={}&max-zaznamov=2",
                    self.endpoint,
                    id.as_str()
                ),
                "search",
            )
            .await?;
        let internal = decode::select(&search)?;
        tokio::time::sleep_until(start + self.config.start_interval).await;
        let detail = self
            .request(
                &format!("{}/uctovna-jednotka?id={internal}", self.endpoint),
                "detail",
            )
            .await?;
        let mut record = decode::decode(&detail, id, internal)?;
        if record.details.datum_zrusenia.is_none() {
            tokio::time::sleep(self.config.start_interval).await;
            let search = self
                .request(
                    &format!(
                        "{}/search?identifier={}&onlyActive=true",
                        self.rpo_endpoint,
                        id.as_str()
                    ),
                    "rpoSearch",
                )
                .await?;
            let Some(rpo_id) = super::rpo::select(&search, id)? else {
                return Ok(record);
            };
            tokio::time::sleep(self.config.start_interval).await;
            let detail = self
                .request(
                    &format!(
                        "{}/entity/{rpo_id}?showHistoricalData=false&showOrganizationUnits=false",
                        self.rpo_endpoint
                    ),
                    "rpoDetail",
                )
                .await?;
            record.details.rpo_lifecycle = Some(super::rpo::decode(&detail, id, rpo_id)?);
        }
        Ok(record)
    }
    async fn request(&self, url: &str, _stage: &str) -> Result<Vec<u8>, RegistryFailure> {
        let response = self
            .http
            .get(url)
            .header("Accept", "application/json")
            .send_traced()
            .await
            .map_err(network)?;
        #[cfg(test)]
        {
            let mut capture = self.captured.lock().unwrap();
            if capture.is_null() {
                *capture = serde_json::json!({});
            }
            capture[_stage] = serde_json::json!({"httpStatus":response.status().as_u16(),"contentType":response.headers().get("content-type").and_then(|v|v.to_str().ok()),"retryAfter":response.headers().get("retry-after").and_then(|v|v.to_str().ok())});
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
            self.captured.lock().unwrap()[_stage]["body"] =
                serde_json::json!(String::from_utf8_lossy(&bytes));
        }
        Ok(bytes)
    }
    #[cfg(test)]
    pub(crate) fn for_test(endpoint: String, config: SkRegistryConfig) -> Self {
        let mut client = Self::with_config(config).unwrap();
        client.rpo_endpoint = format!("{endpoint}/rpo");
        client.endpoint = endpoint;
        client
    }
}

#[cfg(test)]
impl SkRegistryClient {
    pub(super) fn begin_capture(&self) {
        *self.captured.lock().unwrap() = serde_json::Value::Null;
    }
    pub(super) fn take_capture(&self) -> serde_json::Value {
        self.captured.lock().unwrap().take()
    }
}
