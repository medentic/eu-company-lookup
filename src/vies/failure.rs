//! VIES policy facts before conversion into the public error object.
use crate::{LimitScope, LookupError, RetryAfter, model::LookupFailure};

#[derive(Debug)]
pub(super) struct Failure {
    pub kind: LookupFailure,
    pub upstream_code: Option<&'static str>,
}
impl From<LookupFailure> for Failure {
    fn from(kind: LookupFailure) -> Self {
        Self {
            kind,
            upstream_code: None,
        }
    }
}
impl From<Failure> for LookupError {
    fn from(failure: Failure) -> Self {
        let mut error = Self::from(failure.kind);
        if let Some(code) = failure.upstream_code {
            error.details.upstream_code = Some(code);
        }
        error
    }
}
impl Failure {
    /// Recognized SOAP faults take precedence over the enclosing HTTP status.
    pub fn is_protocol_fault(&self) -> bool {
        use LookupFailure::*;
        matches!(
            self.kind,
            RateLimited { .. }
                | ServiceUnavailable { .. }
                | MemberStateUnavailable { .. }
                | ProviderTimeout { .. }
                | ProviderRejectedInput
                | UnknownSoapFault
        )
    }

    pub fn with_retry_after(mut self, retry: Option<RetryAfter>) -> Self {
        use LookupFailure::*;
        match &mut self.kind {
            RateLimited { retry_after, .. }
            | ServiceUnavailable { retry_after }
            | MemberStateUnavailable { retry_after }
            | ProviderTimeout { retry_after } => {
                *retry_after = retry;
            }
            _ => {}
        }
        self
    }

    pub fn cooldown(&self) -> Option<(LimitScope, Option<&RetryAfter>)> {
        use LookupFailure::*;
        Some(match &self.kind {
            RateLimited { scope, retry_after } => (*scope, retry_after.as_ref()),
            MemberStateUnavailable { retry_after } | ProviderTimeout { retry_after } => {
                (LimitScope::MemberState, retry_after.as_ref())
            }
            ServiceUnavailable { retry_after }
            | Http {
                status: 503,
                retry_after,
            } => (LimitScope::Global, retry_after.as_ref()),
            Transport | DeadlineExceeded => (LimitScope::Global, None),
            _ => return None,
        })
    }
}
