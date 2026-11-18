use jiff::Timestamp;
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
    Date(Timestamp),
}

/// Provider-independent failure object returned by every public lookup.
/// Display and Debug omit raw diagnostics; explicit serialization includes them when enabled.
///
/// Version 4.0 removes the legacy public registry error type:
/// ```compile_fail
/// use eu_company_lookup::RegistryError;
/// ```
/// Match `error.id` instead of the old enum variants:
/// ```compile_fail
/// use eu_company_lookup::LookupError;
/// let error = LookupError::InvalidInput;
/// ```
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct LookupError {
    /// Stable machine-readable classification.
    pub id: super::ErrorId,
    /// English title suitable for user interfaces.
    pub title: &'static str,
    /// English explanation intended for administrators.
    pub description: &'static str,
    /// Service or network availability prevented verification; validity is unknown.
    pub is_service_unavailable: bool,
    /// Optional context and explicitly enabled raw HTTP diagnostics.
    pub details: Box<ErrorDetails>,
    // Retained only for private policy regression tests and evidence probes.
    #[cfg(test)]
    #[serde(skip)]
    pub(crate) kind: super::LookupFailure,
}

/// Structured context. Missing facts remain null; raw diagnostics are opt-in.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ErrorDetails {
    /// Provider responsible for the failed observation, when known.
    pub provider: Option<super::Provider>,
    /// ISO country, when established by the lookup context.
    pub country: Option<String>,
    /// Specific sanitized reason within the common error category.
    pub reason: Option<&'static str>,
    /// Original source lifecycle text/code for unresolved company data. Not a negative result.
    pub native_registry_status: Option<String>,
    /// Recognized upstream protocol fault code; arbitrary body text is never copied here.
    pub upstream_code: Option<&'static str>,
    /// Actual upstream HTTP status, never an application HTTP response recommendation.
    pub upstream_http_status: Option<u16>,
    /// Actual response Content-Type value when valid text.
    pub response_content_type: Option<String>,
    /// Provider retry instruction, if supplied.
    pub retry_after: Option<RetryAfter>,
    /// Remaining local cooldown in seconds, if applicable.
    pub retry_after_seconds: Option<u64>,
    /// Admission/rate-limit scope, when supplied.
    pub scope: Option<LimitScope>,
    /// Disabled operation, when applicable.
    pub operation: Option<super::Operation>,
    /// Identifier scheme for a disabled operation.
    pub identifier_scheme: Option<super::IdentifierScheme>,
    /// Raw exchanges in request order; absent unless explicitly enabled.
    pub debug: Option<super::HttpDiagnostics>,
}
impl LookupError {
    /// Whether service/network availability prevented verification.
    pub fn is_service_unavailable(&self) -> bool {
        self.is_service_unavailable
    }
}
impl std::fmt::Display for LookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.title)
    }
}
impl std::fmt::Debug for LookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LookupError")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("provider", &self.details.provider)
            .field("reason", &self.details.reason)
            .finish_non_exhaustive()
    }
}
impl std::error::Error for LookupError {}
impl From<super::LookupFailure> for LookupError {
    fn from(kind: super::LookupFailure) -> Self {
        use super::{ErrorId as I, LookupFailure as F, RegistryFailure as R};
        let mut details = ErrorDetails::default();
        let id = match &kind {
            F::Unknown => I::UnknownError,
            F::UnresolvedData {
                provider,
                country,
                native_status,
                reason,
            } => {
                details.provider = Some(*provider);
                details.country = Some(country.clone());
                details.native_registry_status = native_status.clone();
                details.reason = Some(reason);
                I::UnresolvedData
            }
            F::InvalidInput => I::InvalidInput,
            F::UnsupportedJurisdiction => {
                details.reason = Some("unsupported_vat_jurisdiction");
                I::UnsupportedIdentifier
            }
            F::UnsupportedIdentifierScheme => {
                details.reason = Some("unsupported_identifier_scheme");
                I::UnsupportedIdentifier
            }
            F::OperationDisabled {
                operation,
                identifier_scheme,
            } => {
                details.operation = Some(*operation);
                details.identifier_scheme = Some(*identifier_scheme);
                I::OperationDisabled
            }
            F::InvalidConfiguration(reason) => {
                details.reason = Some(reason);
                I::InvalidConfiguration
            }
            F::ClientInitialization => I::InitializationFailed,
            F::Busy => I::Busy,
            F::DeadlineExceeded => {
                details.reason = Some("timeout");
                I::ServiceUnavailable
            }
            F::Transport => {
                details.reason = Some("connection");
                I::ServiceUnavailable
            }
            F::ProviderRejectedInput => {
                details.upstream_code = Some("INVALID_INPUT");
                I::ProviderRejectedInput
            }
            F::RateLimited { scope, retry_after } => {
                details.scope = Some(*scope);
                details.retry_after = retry_after.clone();
                I::RateLimited
            }
            F::CoolingDown {
                scope,
                retry_after_seconds,
            } => {
                details.scope = Some(*scope);
                details.retry_after_seconds = Some(*retry_after_seconds);
                I::CoolingDown
            }
            F::ServiceUnavailable { retry_after }
            | F::MemberStateUnavailable { retry_after }
            | F::ProviderTimeout { retry_after } => {
                details.retry_after = retry_after.clone();
                details.reason = Some(match kind {
                    F::MemberStateUnavailable { .. } => "member_state_unavailable",
                    F::ProviderTimeout { .. } => "upstream_timeout",
                    _ => "service_unavailable",
                });
                details.upstream_code = match kind {
                    F::MemberStateUnavailable { .. } => Some("MS_UNAVAILABLE"),
                    F::ProviderTimeout { .. } => None,
                    _ => None,
                };
                I::ServiceUnavailable
            }
            F::UnknownSoapFault => {
                details.reason = Some("unrecognized_soap_fault");
                I::UnexpectedResponse
            }
            F::Http {
                status,
                retry_after,
            } => {
                details.upstream_http_status = Some(*status);
                details.retry_after = retry_after.clone();
                if (500..=599).contains(status) || *status == 408 {
                    I::ServiceUnavailable
                } else {
                    I::UnexpectedResponse
                }
            }
            F::ResponseTooLarge => I::ResponseTooLarge,
            F::MalformedResponse => {
                details.reason = Some("malformed_response");
                I::UnexpectedResponse
            }
            F::MismatchedResponse => I::ResponseMismatch,
            F::NotFound => {
                details.reason = Some("both_bulgarian_registers_missed");
                details.country = Some("BG".into());
                I::NotFound
            }
            F::RegistryNotFound { provider } => {
                details.provider = Some(*provider);
                I::NotFound
            }
            F::Registry { provider, error } => {
                details.provider = Some(*provider);
                match error {
                    R::ServiceUnavailable {
                        reason,
                        retry_after,
                    } => {
                        details.reason = Some(match reason {
                            super::UnavailabilityReason::ServerError => "server_error",
                            super::UnavailabilityReason::Timeout => "timeout",
                            super::UnavailabilityReason::Connection => "connection",
                        });
                        details.retry_after = retry_after.clone();
                        I::ServiceUnavailable
                    }
                    R::RateLimited { retry_after } => {
                        details.retry_after = retry_after.clone();
                        I::RateLimited
                    }
                    R::CoolingDown {
                        retry_after_seconds,
                    } => {
                        details.retry_after_seconds = Some(*retry_after_seconds);
                        I::CoolingDown
                    }
                    R::AccessDenied => I::AccessDenied,
                    R::Http { status } => {
                        details.upstream_http_status = Some(*status);
                        I::UnexpectedResponse
                    }
                    R::UnexpectedResponse => I::UnexpectedResponse,
                    R::MismatchedResponse => I::ResponseMismatch,
                    R::ResponseTooLarge => I::ResponseTooLarge,
                }
            }
        };
        Self {
            id,
            title: id.title(),
            description: id.description(),
            is_service_unavailable: id == I::ServiceUnavailable,
            details: Box::new(details),
            #[cfg(test)]
            kind,
        }
    }
}

impl Default for LookupError {
    fn default() -> Self {
        super::LookupFailure::Unknown.into()
    }
}
