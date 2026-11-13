use chrono::{DateTime, Utc};
use serde::Serialize;

/// Scope of a provider admission limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum LimitScope {
    /// All jurisdictions on this shared client.
    Global,
    /// Only the requested VAT jurisdiction.
    MemberState,
}

/// Parsed HTTP Retry-After value, preserved without converting dates to seconds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RetryAfter {
    /// Delay in seconds.
    Seconds(u64),
    /// Absolute HTTP date.
    Date(DateTime<Utc>),
}

/// Failure of a lookup. Error text never contains a VAT ID, owner or response body.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LookupError {
    /// Malformed input or unsupported national syntax.
    #[error("identifier has invalid syntax")]
    InvalidInput,
    /// Well-formed prefix is not supported by this version's VIES adapter.
    #[error("VAT jurisdiction is not supported; Greece uses EL, not GR")]
    UnsupportedJurisdiction,
    /// Invalid client configuration.
    #[error("invalid client configuration: {0}")]
    InvalidConfiguration(&'static str),
    /// HTTP client could not be initialized.
    #[error("could not initialize the HTTP client")]
    ClientInitialization,
    /// Shared global capacity or the country's active slot is occupied.
    #[error("client capacity is occupied; try later")]
    Busy,
    /// Network/response deadline expired. Local spacing that cannot fit the
    /// configured lookup deadline returns Busy before a request is sent.
    #[error("lookup deadline exceeded")]
    DeadlineExceeded,
    /// HTTP/network/TLS failure, with no provider payload in the error.
    #[error("VIES transport failed")]
    Transport,
    /// Provider rejected the supplied input; distinct from a negative VAT check.
    #[error("VIES rejected the input")]
    ProviderRejectedInput,
    /// Provider asked for reduced traffic.
    #[error("VIES rate limit ({scope:?})")]
    RateLimited {
        /// Scope of the reported limit.
        scope: LimitScope,
        /// Provider's Retry-After header, if valid.
        retry_after: Option<RetryAfter>,
    },
    /// A previous request established a shared cooldown; no request was sent.
    #[error("VIES client is cooling down ({scope:?})")]
    CoolingDown {
        /// Scope of the cooldown.
        scope: LimitScope,
        /// Remaining delay, rounded up to seconds.
        retry_after_seconds: u64,
    },
    /// The central VIES service is unavailable.
    #[error("VIES service unavailable")]
    ServiceUnavailable {
        /// Provider Retry-After header, when available.
        retry_after: Option<RetryAfter>,
    },
    /// The requested national service is unavailable.
    #[error("VIES member-state service unavailable")]
    MemberStateUnavailable {
        /// Provider Retry-After header, when available.
        retry_after: Option<RetryAfter>,
    },
    /// VIES reported its own upstream timeout.
    #[error("VIES member-state request timed out")]
    ProviderTimeout {
        /// Provider Retry-After header, when available.
        retry_after: Option<RetryAfter>,
    },
    /// An unrecognized SOAP fault; its arbitrary text is intentionally not exposed.
    #[error("unrecognized VIES SOAP fault")]
    UnknownSoapFault,
    /// Non-success HTTP response without a recognized SOAP fault.
    #[error("VIES HTTP status {status}")]
    Http {
        /// HTTP status code.
        status: u16,
        /// Provider's Retry-After header, if valid.
        retry_after: Option<RetryAfter>,
    },
    /// Response exceeds the configured byte limit.
    #[error("VIES response exceeds the size limit")]
    ResponseTooLarge,
    /// Invalid XML, SOAP structure, field or date.
    #[error("malformed VIES response")]
    MalformedResponse,
    /// Country or number differs from the exact requested identifier.
    #[error("VIES response does not match the requested identifier")]
    MismatchedResponse,
    /// Both Bulgarian registers definitively returned no matching business record.
    /// Does not describe a VIES negative result or any service failure.
    #[error("business identifier was not found in either Bulgarian register")]
    NotFound,
    /// Identifier-matched negative observation from a specific registry.
    /// Does not prove absence from other sources or VAT invalidity.
    #[error("business identifier was not found by {provider:?}")]
    RegistryNotFound {
        /// Source that explicitly reported the miss.
        provider: super::Provider,
    },
    /// Source-specific registry failure; no conclusion about business existence.
    #[error("{provider:?}: {error}")]
    Registry {
        /// Register that failed; fallback is not attempted after this failure.
        provider: super::Provider,
        /// Sanitized operational or response error.
        #[source]
        error: super::RegistryError,
    },
}

impl LookupError {
    /// Whether service/network availability prevented verification. Such failures
    /// must never be presented as an invalid VAT/business ID. A transport failure
    /// may be local connectivity rather than a confirmed outage at the provider.
    /// Local admission limits and rate limits remain separately typed errors.
    pub fn is_service_unavailable(&self) -> bool {
        matches!(
            self,
            Self::ServiceUnavailable { .. }
                | Self::MemberStateUnavailable { .. }
                | Self::ProviderTimeout { .. }
                | Self::DeadlineExceeded
                | Self::Transport
                | Self::Registry {
                    error: super::RegistryError::ServiceUnavailable { .. },
                    ..
                }
                | Self::Http {
                    status: 500..=599 | 408,
                    ..
                }
        )
    }
}
