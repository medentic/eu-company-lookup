//! Private mechanics for bounded anonymous registry requests.
//! Identity, accepted HTTP statuses and source decoding remain adapter-owned.
use crate::{LookupError, Provider, RegistryFailure, RetryAfter, UnavailabilityReason};
use std::{
    future::Future,
    time::{Duration, SystemTime},
};
use tokio::{sync::Mutex, time::Instant};

pub(crate) struct Bounds {
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub start_interval: Duration,
    pub failure_cooldown: Duration,
    pub max_response_bytes: usize,
    /// Capture raw HTTP requests/responses in errors. Default: false; may expose sensitive data.
    pub raw_debug: bool,
}
impl Bounds {
    pub(crate) fn client(&self, message: &'static str) -> Result<reqwest::Client, LookupError> {
        if self.timeout.is_zero()
            || self.timeout > Duration::from_secs(300)
            || self.connect_timeout.is_zero()
            || self.connect_timeout > self.timeout
            || self.start_interval < Duration::from_secs(1)
            || self.start_interval > Duration::from_secs(86400)
            || self.failure_cooldown.is_zero()
            || self.failure_cooldown > Duration::from_secs(86400)
            || !(1..=4_194_304).contains(&self.max_response_bytes)
        {
            return Err(LookupError::from(
                crate::model::LookupFailure::InvalidConfiguration(message),
            ));
        }
        reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(self.connect_timeout)
            .timeout(self.timeout)
            .user_agent(concat!("eu-company-lookup/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| LookupError::from(crate::model::LookupFailure::ClientInitialization))
    }
}
pub(crate) struct Admission {
    next: Instant,
    cooldown: Option<Instant>,
}
impl Admission {
    pub(crate) fn new() -> Mutex<Self> {
        Mutex::new(Self {
            next: Instant::now(),
            cooldown: None,
        })
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Spacing {
    FromStart,
    // Multi-request adapters retain the gap after their final response.
    AfterCompletion,
}
pub(crate) async fn execute<T>(
    admission: &Mutex<Admission>,
    bounds: &Bounds,
    provider: Provider,
    spacing: Spacing,
    request: impl Future<Output = Result<T, RegistryFailure>>,
) -> Result<T, LookupError> {
    crate::http_capture::scope(
        bounds.raw_debug,
        bounds.max_response_bytes,
        bounds.timeout,
        provider,
        async {
            let deadline = Instant::now() + bounds.timeout;
            let mut state = admission
                .try_lock()
                .map_err(|_| LookupError::from(crate::model::LookupFailure::Busy))?;
            if let Some(end) = state.cooldown.filter(|end| *end > Instant::now()) {
                let left = end.saturating_duration_since(Instant::now());
                return Err(wrap(
                    provider,
                    RegistryFailure::CoolingDown {
                        retry_after_seconds: left.as_secs() + u64::from(left.subsec_nanos() > 0),
                    },
                ));
            }
            if state.next >= deadline {
                return Err(LookupError::from(crate::model::LookupFailure::Busy));
            }
            tokio::time::sleep_until(state.next).await;
            if Instant::now() >= deadline {
                return Err(LookupError::from(crate::model::LookupFailure::Busy));
            }
            state.next = Instant::now() + bounds.start_interval;
            let result = tokio::time::timeout_at(deadline, request)
                .await
                .unwrap_or_else(|_| Err(unavailable(UnavailabilityReason::Timeout, None)));
            if let Err(
                RegistryFailure::ServiceUnavailable { retry_after, .. }
                | RegistryFailure::RateLimited { retry_after },
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
                    .max(bounds.failure_cooldown)
                    .min(Duration::from_secs(315_360_000));
                state.cooldown = Some(Instant::now() + delay);
            }
            if spacing == Spacing::AfterCompletion {
                state.next = Instant::now() + bounds.start_interval;
            }
            result.map_err(|error| wrap(provider, error))
        },
    )
    .await
}
fn wrap(provider: Provider, error: RegistryFailure) -> LookupError {
    LookupError::from(crate::model::LookupFailure::Registry { provider, error })
}
// Accepted statuses are explicit at the call site. In particular, accepting an
// ARES 404 body does not turn any other registry's 404 into a definitive miss.
pub(crate) fn check_status(
    status: u16,
    retry_after: Option<RetryAfter>,
    accepted: &[u16],
) -> Result<(), RegistryFailure> {
    match status {
        429 => Err(RegistryFailure::RateLimited { retry_after }),
        408 => Err(unavailable(UnavailabilityReason::Timeout, retry_after)),
        500..=599 => Err(unavailable(UnavailabilityReason::ServerError, retry_after)),
        401 | 403 => Err(RegistryFailure::AccessDenied),
        status if accepted.contains(&status) => Ok(()),
        status => Err(RegistryFailure::Http { status }),
    }
}

pub(crate) mod ckan;

pub(crate) fn retry_after(response: &crate::http_capture::Response) -> Option<RetryAfter> {
    response
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
        })
}
pub(crate) async fn json_body(
    response: crate::http_capture::Response,
    limit: usize,
) -> Result<Vec<u8>, RegistryFailure> {
    body_for_type(response, limit, "application/json").await
}

// Pin the provider's media type (including HTML and versioned JSON) without
// relaxing another provider's response contract.
pub(crate) async fn body_for_type(
    mut response: crate::http_capture::Response,
    limit: usize,
    expected_type: &'static str,
) -> Result<Vec<u8>, RegistryFailure> {
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .map(str::trim);
    if !content_type.is_some_and(|v| v.eq_ignore_ascii_case(expected_type)) {
        return Err(RegistryFailure::UnexpectedResponse);
    }
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err(RegistryFailure::ResponseTooLarge);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(network)? {
        if chunk.len() > limit.saturating_sub(bytes.len()) {
            return Err(RegistryFailure::ResponseTooLarge);
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}
pub(crate) fn unavailable(
    reason: UnavailabilityReason,
    retry_after: Option<RetryAfter>,
) -> RegistryFailure {
    RegistryFailure::ServiceUnavailable {
        reason,
        retry_after,
    }
}
pub(crate) fn network(error: reqwest::Error) -> RegistryFailure {
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
mod tests;
