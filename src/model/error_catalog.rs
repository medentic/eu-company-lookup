use serde::Serialize;

/// Stable provider-independent error identifiers. Match unknown future IDs defensively.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ErrorId {
    /// Input failed local syntax validation; registration was not checked.
    InvalidInput,
    /// The country, VAT jurisdiction or identifier scheme is unsupported. Greece uses the EL VAT prefix.
    UnsupportedIdentifier,
    /// Client configuration disabled the requested operation; no request was sent.
    OperationDisabled,
    /// Client configuration is invalid. Inspect the configuration reason.
    InvalidConfiguration,
    /// The HTTP client could not initialize.
    InitializationFailed,
    /// Local capacity or admission timing prevented sending a request.
    Busy,
    /// The provider requested reduced traffic. Respect its retry instruction when supplied.
    RateLimited,
    /// A previous response established a cooldown; no request was sent.
    CoolingDown,
    /// Server, network or timeout failure prevented verification. Identifier validity remains unknown.
    ServiceUnavailable,
    /// The provider requires authentication or denied access; this is not a negative lookup.
    AccessDenied,
    /// The provider rejected the request; this does not establish invalid registration.
    ProviderRejectedInput,
    /// An unexpected HTTP status, malformed response, changed schema or unrecognized fault prevented verification.
    UnexpectedResponse,
    /// The response identifies a different subject; it cannot establish validity of the requested identifier.
    ResponseMismatch,
    /// The response exceeded the configured transport size limit.
    ResponseTooLarge,
    /// The identified registry sources definitively reported no matching company record. This does not establish VAT invalidity.
    NotFound,
    /// An unclassified failure prevented verification. No validity conclusion follows.
    UnknownError,
}
impl ErrorId {
    /// Stable machine-readable spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid_input",
            Self::UnsupportedIdentifier => "unsupported_identifier",
            Self::OperationDisabled => "operation_disabled",
            Self::InvalidConfiguration => "invalid_configuration",
            Self::InitializationFailed => "initialization_failed",
            Self::Busy => "busy",
            Self::RateLimited => "rate_limited",
            Self::CoolingDown => "cooling_down",
            Self::ServiceUnavailable => "service_unavailable",
            Self::AccessDenied => "access_denied",
            Self::ProviderRejectedInput => "provider_rejected_input",
            Self::UnexpectedResponse => "unexpected_response",
            Self::ResponseMismatch => "response_mismatch",
            Self::ResponseTooLarge => "response_too_large",
            Self::NotFound => "not_found",
            Self::UnknownError => "unknown_error",
        }
    }
    /// Default English user-facing title.
    pub fn title(self) -> &'static str {
        match self {
            Self::InvalidInput => "Check the identifier",
            Self::UnsupportedIdentifier => "Identifier type not supported",
            Self::OperationDisabled => "Lookup is disabled",
            Self::InvalidConfiguration => "Lookup configuration needs attention",
            Self::InitializationFailed => "Lookup could not start",
            Self::Busy => "Verification is busy",
            Self::RateLimited => "Too many verification requests",
            Self::CoolingDown => "Please wait before retrying",
            Self::ServiceUnavailable => "Verification service unavailable",
            Self::AccessDenied => "Verification access denied",
            Self::ProviderRejectedInput => "Verification request rejected",
            Self::UnexpectedResponse => "Verification could not be completed",
            Self::ResponseMismatch => "Verification response could not be confirmed",
            Self::ResponseTooLarge => "Verification could not be completed",
            Self::NotFound => "No matching company record found",
            Self::UnknownError => "Verification could not be completed",
        }
    }
    /// English administrator explanation, independent of raw diagnostics.
    pub fn description(self) -> &'static str {
        match self {
            Self::InvalidInput => {
                "Input failed local syntax validation; registration was not checked."
            }
            Self::UnsupportedIdentifier => {
                "The country, VAT jurisdiction or identifier scheme is unsupported. Greece uses the EL VAT prefix."
            }
            Self::OperationDisabled => {
                "Client configuration disabled the requested operation; no request was sent."
            }
            Self::InvalidConfiguration => {
                "Client configuration is invalid. Inspect the configuration reason."
            }
            Self::InitializationFailed => "The HTTP client could not initialize.",
            Self::Busy => "Local capacity or admission timing prevented sending a request.",
            Self::RateLimited => {
                "The provider requested reduced traffic. Respect its retry instruction when supplied."
            }
            Self::CoolingDown => "A previous response established a cooldown; no request was sent.",
            Self::ServiceUnavailable => {
                "Server, network or timeout failure prevented verification. Identifier validity remains unknown."
            }
            Self::AccessDenied => {
                "The provider requires authentication or denied access; this is not a negative lookup."
            }
            Self::ProviderRejectedInput => {
                "The provider rejected the request; this does not establish invalid registration."
            }
            Self::UnexpectedResponse => {
                "An unexpected HTTP status, malformed response, changed schema or unrecognized fault prevented verification."
            }
            Self::ResponseMismatch => {
                "The response identifies a different subject; it cannot establish validity of the requested identifier."
            }
            Self::ResponseTooLarge => "The response exceeded the configured transport size limit.",
            Self::NotFound => {
                "The identified registry sources definitively reported no matching company record. This does not establish VAT invalidity."
            }
            Self::UnknownError => {
                "An unclassified failure prevented verification. No validity conclusion follows."
            }
        }
    }
}
