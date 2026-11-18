use super::{PlBusinessId, PlCompanyLookupResult, decode};
use crate::http_capture::SendTraced;
use crate::registry_transport::{self, Admission, Bounds, Spacing, network};
use crate::{LookupError, Provider, RegistryFailure};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

const ENDPOINT: &str = "https://api-krs.ms.gov.pl/api/krs/OdpisAktualny";

/// Polish public registry transport bounds, independent of VIES and Bulgarian registry limits.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct PlRegistryConfig {
    /// Total deadline including admission waiting. Default: 30 seconds.
    pub timeout: Duration,
    /// Connection timeout. Default: 3 seconds.
    pub connect_timeout: Duration,
    /// Gap after lookup completion and between current/full exchanges. Default: 2 seconds; minimum: 1 second (library policy).
    pub start_interval: Duration,
    /// Cooldown on availability failures or throttling. Default: 60 seconds.
    pub failure_cooldown: Duration,
    /// Maximum response size. Default: 1 MiB; maximum: 4 MiB.
    pub max_response_bytes: usize,
    /// Capture raw HTTP requests/responses in errors. Default: false; may expose sensitive data.
    pub raw_debug: bool,
}
impl Default for PlRegistryConfig {
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
impl PlRegistryConfig {
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

/// Single-identifier Polish public registry client with admission shared across clones. No automatic
/// retries, redirects, ambient proxies, cookies or persistent cache.
#[derive(Clone)]
pub struct PlRegistryClient {
    http: reqwest::Client,
    config: PlRegistryConfig,
    admission: Arc<Mutex<Admission>>,
    endpoint: String,
    full_endpoint: String,
    #[cfg(test)]
    captured: Arc<std::sync::Mutex<serde_json::Value>>,
}
impl std::fmt::Debug for PlRegistryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlRegistryClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}
impl PlRegistryClient {
    /// Construct a client with bounded defaults.
    pub fn new() -> Result<Self, LookupError> {
        Self::with_config(PlRegistryConfig::default())
    }
    /// Construct with explicit bounds; library policy requires at least one-second spacing.
    pub fn with_config(config: PlRegistryConfig) -> Result<Self, LookupError> {
        let http = config.bounds().client("invalid Polish registry bounds")?;
        Ok(Self {
            http,
            config,
            admission: Arc::new(Admission::new()),
            endpoint: ENDPOINT.into(),
            full_endpoint: "https://api-krs.ms.gov.pl/api/krs/OdpisPelny".into(),
            #[cfg(test)]
            captured: Arc::new(std::sync::Mutex::new(serde_json::Value::Null)),
        })
    }
    /// Look up the current Polish public registry observation by full business ID.
    pub async fn lookup(&self, value: &str) -> Result<PlCompanyLookupResult, LookupError> {
        let id = PlBusinessId::parse(value)?;
        registry_transport::execute_company(
            &self.admission,
            &self.config.bounds(),
            Provider::PolishKrs,
            Spacing::AfterCompletion,
            self.send(&id),
        )
        .await
    }
    async fn send(&self, id: &PlBusinessId) -> Result<PlCompanyLookupResult, RegistryFailure> {
        let mut association = false;
        loop {
            let register = if association { "S" } else { "P" };
            let expected = if association { "RejS" } else { "RejP" };
            let response = self
                .http
                .get(format!(
                    "{}/{}?rejestr={register}&format=json",
                    self.endpoint,
                    id.as_str()
                ))
                .header("Accept", "application/json")
                .send_traced()
                .await
                .map_err(network)?;
            let status = response.status().as_u16();
            #[cfg(test)]
            {
                let value = serde_json::json!({"httpStatus":status,"contentType":response.headers().get("content-type").and_then(|v|v.to_str().ok()),"retryAfter":response.headers().get("retry-after").and_then(|v|v.to_str().ok())});
                if association {
                    self.captured.lock().unwrap()["association"] = value;
                } else {
                    *self.captured.lock().unwrap() = value;
                }
            }
            // Only a P-route 404 permits the alternate register. Outages, malformed
            // bodies and unresolved records never initiate a second-register lookup.
            if status == 404 && !association {
                association = true;
                tokio::time::sleep(self.config.start_interval).await;
                continue;
            }
            if status == 204 {
                tokio::time::sleep(self.config.start_interval).await;
                let response = self
                    .http
                    .get(format!(
                        "{}/{}?rejestr={register}&format=json",
                        self.full_endpoint,
                        id.as_str()
                    ))
                    .header("Accept", "application/json")
                    .send_traced()
                    .await
                    .map_err(network)?;
                #[cfg(test)]
                {
                    let mut capture = self.captured.lock().unwrap();
                    let slot = if association {
                        &mut capture["association"]
                    } else {
                        &mut *capture
                    };
                    slot["full"] = serde_json::json!({"httpStatus":response.status().as_u16()});
                }
                registry_transport::check_status(
                    response.status().as_u16(),
                    registry_transport::retry_after(&response),
                    &[200],
                )?;
                let bytes =
                    registry_transport::json_body(response, self.config.max_response_bytes).await?;
                #[cfg(test)]
                {
                    let mut capture = self.captured.lock().unwrap();
                    let slot = if association {
                        &mut capture["association"]
                    } else {
                        &mut *capture
                    };
                    slot["full"]["body"] = serde_json::json!(String::from_utf8_lossy(&bytes));
                }
                return super::full::decode_for(&bytes, id, expected);
            }
            registry_transport::check_status(
                status,
                registry_transport::retry_after(&response),
                &[200],
            )?;
            let bytes =
                registry_transport::json_body(response, self.config.max_response_bytes).await?;
            #[cfg(test)]
            {
                let mut capture = self.captured.lock().unwrap();
                let slot = if association {
                    &mut capture["association"]
                } else {
                    &mut *capture
                };
                slot["body"] = serde_json::json!(String::from_utf8_lossy(&bytes));
            }
            return decode::decode_for(&bytes, id, expected);
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(endpoint: String, config: PlRegistryConfig) -> Self {
        let mut client = Self::with_config(config).unwrap();
        client.full_endpoint = format!("{endpoint}/full");
        client.endpoint = endpoint;
        client
    }
}

#[cfg(test)]
impl PlRegistryClient {
    pub(super) fn begin_capture(&self) {
        *self.captured.lock().unwrap() = serde_json::Value::Null;
    }
    pub(super) fn take_capture(&self) -> serde_json::Value {
        self.captured.lock().unwrap().take()
    }
}
