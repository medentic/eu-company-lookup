use crate::RetryAfter;

/// Why a provider cannot currently complete verification. None proves an ID invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnavailabilityReason {
    /// Provider returned a server error.
    ServerError,
    /// Connection, response, or response-body deadline expired.
    Timeout,
    /// DNS, TLS, connection, or interrupted-response failure.
    Connection,
}

/// Sanitized registry failure; never contains the submitted code or provider body.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RegistryError {
    /// The lookup could not be completed; validity remains unknown.
    #[error("registry service unavailable ({reason:?})")]
    ServiceUnavailable {
        /// Failure category; a connection failure does not prove a server outage.
        reason: UnavailabilityReason,
        /// Provider retry instruction, if supplied.
        retry_after: Option<RetryAfter>,
    },
    /// Provider requested reduced traffic.
    #[error("registry rate limit")]
    RateLimited {
        /// Provider retry instruction, if supplied.
        retry_after: Option<RetryAfter>,
    },
    /// A previous provider error established a cooldown; no request was sent.
    #[error("registry client is cooling down")]
    CoolingDown {
        /// Remaining delay rounded up to seconds.
        retry_after_seconds: u64,
    },
    /// Access requires authentication or has been denied; not a negative lookup.
    #[error("registry access denied")]
    AccessDenied,
    /// Unexpected non-success response, including redirects and 404 endpoint errors.
    #[error("registry HTTP status {status}")]
    Http {
        /// Received HTTP status.
        status: u16,
    },
    /// Wrong content type, changed schema, ambiguous fields or unrecognized markup.
    #[error("unexpected registry response")]
    UnexpectedResponse,
    /// The returned record identifies a different subject.
    #[error("registry response does not match the requested identifier")]
    MismatchedResponse,
    /// Response exceeded the configured bound.
    #[error("registry response exceeds the size limit")]
    ResponseTooLarge,
}
