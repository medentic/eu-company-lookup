use super::{IsBusinessId, IsCompanyLookupResult, decode, overview};
use crate::http_capture::SendTraced;
use crate::registry_transport::{self, Admission, Bounds, Spacing, network};
use crate::{LookupError, Provider, RegistryFailure};
use reqwest::cookie::CookieStore;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

const ENDPOINT: &str = "https://www.skatturinn.is/fyrirtaekjaskra/leit/kennitala";

/// Icelandic public registry transport bounds, independent of VIES and Bulgarian registry limits.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct IsRegistryConfig {
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
impl Default for IsRegistryConfig {
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
impl IsRegistryConfig {
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

/// Single-identifier Icelandic public registry client with admission shared across clones. No automatic
/// retries, redirects, ambient proxies or persistent cache. The free-overview fallback
/// uses an isolated session cookie jar that is discarded after each lookup.
#[derive(Clone)]
pub struct IsRegistryClient {
    http: reqwest::Client,
    config: IsRegistryConfig,
    admission: Arc<Mutex<Admission>>,
    endpoint: String,
    cart_endpoint: String,
    shop_endpoint: String,
    #[cfg(test)]
    captured: Arc<std::sync::Mutex<serde_json::Value>>,
}
impl std::fmt::Debug for IsRegistryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IsRegistryClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}
impl IsRegistryClient {
    /// Construct a client with bounded defaults.
    pub fn new() -> Result<Self, LookupError> {
        Self::with_config(IsRegistryConfig::default())
    }
    /// Construct with explicit bounds; library policy requires at least one-second spacing.
    pub fn with_config(config: IsRegistryConfig) -> Result<Self, LookupError> {
        let http = config
            .bounds()
            .client("invalid Icelandic registry bounds")?;
        Ok(Self {
            http,
            config,
            admission: Arc::new(Admission::new()),
            endpoint: ENDPOINT.into(),
            cart_endpoint: "https://www.skatturinn.is/da/CartService/addToCart".into(),
            shop_endpoint: "https://vefur.rsk.is/Vefverslun".into(),
            #[cfg(test)]
            captured: Arc::new(std::sync::Mutex::new(serde_json::Value::Null)),
        })
    }
    /// Look up the current Icelandic public registry observation by full business ID.
    pub async fn lookup(&self, value: &str) -> Result<IsCompanyLookupResult, LookupError> {
        let id = IsBusinessId::parse(value)?;
        registry_transport::execute_company(
            &self.admission,
            &self.config.bounds(),
            Provider::IcelandicSkatturinn,
            Spacing::AfterCompletion,
            self.send(&id),
        )
        .await
    }
    async fn send(&self, id: &IsBusinessId) -> Result<IsCompanyLookupResult, RegistryFailure> {
        let response = self
            .http
            .get(format!("{}/{}", self.endpoint, id.as_str()))
            .header("Accept", "text/html")
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
        let bytes = registry_transport::body_for_type(
            response,
            self.config.max_response_bytes,
            "text/html",
        )
        .await?;
        #[cfg(test)]
        {
            self.captured.lock().unwrap()["body"] =
                serde_json::json!(String::from_utf8_lossy(&bytes));
        }
        let mut native = decode::decode(&bytes, id)?;
        let common: crate::CompanyLookupResult = native.clone().into();
        if common
            .status
            .is_none_or(|s| s == crate::CompanyStatus::Unknown)
            && overview::available(&bytes, id)?
        {
            native.details.registration_overview = Some(self.registration_overview(id).await?);
        }
        Ok(native)
    }
    async fn registration_overview(
        &self,
        id: &IsBusinessId,
    ) -> Result<super::IsRegistrationOverview, RegistryFailure> {
        // This jar exists only for this lookup. No browser, ambient or cross-lookup cookies.
        let session = reqwest::cookie::Jar::default();
        let raw = self
            .overview_exchange(
                &format!("{}?itemid={}&typeid=9", self.cart_endpoint, id.as_str()),
                None,
                "text/html",
                &session,
            )
            .await?;
        let kid = overview::cart(&raw)?;
        let cart_url = format!("{}/Default.aspx?kid={kid}", self.shop_endpoint);
        let raw = self
            .overview_exchange(&cart_url, None, "text/html", &session)
            .await?;
        let fields = overview::page_fields(&raw, id, false)?;
        let raw = self
            .overview_exchange(&cart_url, Some(&fields), "text/html", &session)
            .await?;
        let fields = overview::confirmation(&raw, &kid)?;
        let return_url = format!("{}/ReturnPage.aspx?kid={kid}", self.shop_endpoint);
        let raw = self
            .overview_exchange(&return_url, Some(&fields), "text/html", &session)
            .await?;
        let fields = overview::page_fields(&raw, id, true)?;
        let bytes = self
            .overview_exchange(&return_url, Some(&fields), "application/pdf", &session)
            .await?;
        let id = id.clone();
        tokio::task::spawn_blocking(move || overview::pdf(&bytes, &id))
            .await
            .map_err(|_| RegistryFailure::UnexpectedResponse)?
    }

    async fn overview_exchange(
        &self,
        url: &str,
        fields: Option<&overview::Fields>,
        content_type: &'static str,
        session: &reqwest::cookie::Jar,
    ) -> Result<Vec<u8>, RegistryFailure> {
        tokio::time::sleep(self.config.start_interval).await;
        let url = reqwest::Url::parse(url).map_err(|_| RegistryFailure::UnexpectedResponse)?;
        let mut request = if let Some(fields) = fields {
            self.http
                .post(url.clone())
                .header("Content-Type", "application/x-www-form-urlencoded")
                .header("Origin", url.origin().ascii_serialization())
                .header("Referer", url.as_str())
                .body(overview::form_body(fields))
        } else {
            self.http.get(url.clone())
        };
        if let Some(cookie) = session.cookies(&url) {
            request = request.header(reqwest::header::COOKIE, cookie);
        }
        let response = request
            .header("Accept", content_type)
            .send_traced()
            .await
            .map_err(network)?;
        session.set_cookies(
            &mut response
                .headers()
                .get_all(reqwest::header::SET_COOKIE)
                .iter(),
            &url,
        );
        #[cfg(test)]
        {
            let mut captured = self.captured.lock().unwrap();
            if captured["overviewResponses"].is_null() {
                captured["overviewResponses"] = serde_json::json!([]);
            }
            captured["overviewResponses"].as_array_mut().unwrap().push(serde_json::json!({"httpStatus": response.status().as_u16(), "contentType": response.headers().get("content-type").and_then(|h|h.to_str().ok()), "body":null}));
        }
        registry_transport::check_status(
            response.status().as_u16(),
            registry_transport::retry_after(&response),
            &[200],
        )?;
        let bytes = registry_transport::body_for_type(
            response,
            self.config.max_response_bytes,
            content_type,
        )
        .await?;
        #[cfg(test)]
        {
            use base64::Engine;
            let mut captured = self.captured.lock().unwrap();
            let row = captured["overviewResponses"]
                .as_array_mut()
                .unwrap()
                .last_mut()
                .unwrap();
            row["body"] = if content_type == "application/pdf" {
                serde_json::json!(base64::engine::general_purpose::STANDARD.encode(&bytes))
            } else {
                serde_json::json!(String::from_utf8_lossy(&bytes))
            };
            row["encoding"] = serde_json::json!(if content_type == "application/pdf" {
                "base64"
            } else {
                "utf8"
            });
        }
        Ok(bytes)
    }
    #[cfg(test)]
    pub(crate) fn for_test(endpoint: String, config: IsRegistryConfig) -> Self {
        let mut client = Self::with_config(config).unwrap();
        client.cart_endpoint = format!("{endpoint}/da/CartService/addToCart");
        client.shop_endpoint = format!("{endpoint}/Vefverslun");
        client.endpoint = endpoint;
        client
    }
}

#[cfg(test)]
impl IsRegistryClient {
    pub(super) fn begin_capture(&self) {
        *self.captured.lock().unwrap() = serde_json::Value::Null;
    }
    pub(super) fn take_capture(&self) -> serde_json::Value {
        self.captured.lock().unwrap().take()
    }
}
