use serde::Serialize;

// One entry owns the stable wire ID, title and administrator description.
macro_rules! error_catalog {
    ($($variant:ident => ($id:literal, $title:literal, $description:literal)),+ $(,)?) => {
        /// Stable provider-independent error identifiers. Match future variants defensively.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
        #[non_exhaustive]
        pub enum ErrorId {
            $(#[doc = $description] #[serde(rename = $id)] $variant,)+
        }
        impl ErrorId {
            /// Stable machine-readable spelling, identical to serialization.
            pub fn as_str(self) -> &'static str { match self {$(Self::$variant => $id,)+} }
            /// Default English user-facing title.
            pub fn title(self) -> &'static str { match self {$(Self::$variant => $title,)+} }
            /// English administrator explanation, independent of raw diagnostics.
            pub fn description(self) -> &'static str { match self {$(Self::$variant => $description,)+} }
        }
    };
}

error_catalog! {
    InvalidInput => ("invalid_input", "Check the identifier",
        "Input failed local syntax or supported checksum validation; registration was not checked."),
    UnsupportedIdentifier => ("unsupported_identifier", "Identifier type not supported",
        "The country, VAT jurisdiction or identifier scheme is unsupported. Greece uses the EL VAT prefix."),
    OperationDisabled => ("operation_disabled", "Lookup is disabled",
        "Client configuration disabled the requested operation; no request was sent."),
    InvalidConfiguration => ("invalid_configuration", "Lookup configuration needs attention",
        "Client configuration is invalid. Inspect the configuration reason."),
    InitializationFailed => ("initialization_failed", "Lookup could not start",
        "The HTTP client could not initialize."),
    Busy => ("busy", "Verification is busy",
        "Local capacity or admission timing prevented sending a request."),
    RateLimited => ("rate_limited", "Too many verification requests",
        "The provider requested reduced traffic. Respect its retry instruction when supplied."),
    CoolingDown => ("cooling_down", "Please wait before retrying",
        "A previous response established a cooldown; no request was sent."),
    ServiceUnavailable => ("service_unavailable", "Verification service unavailable",
        "Server, network or timeout failure prevented verification. Identifier validity remains unknown."),
    AccessDenied => ("access_denied", "Verification access denied",
        "The provider requires authentication or denied access; this is not a negative lookup."),
    ProviderRejectedInput => ("provider_rejected_input", "Verification request rejected",
        "The provider rejected the request; this does not establish invalid registration."),
    UnexpectedResponse => ("unexpected_response", "Verification could not be completed",
        "An unexpected HTTP status, malformed response, changed schema or unrecognized fault prevented verification."),
    ResponseMismatch => ("response_mismatch", "Verification response could not be confirmed",
        "The response identifies a different subject; it cannot establish validity of the requested identifier."),
    ResponseTooLarge => ("response_too_large", "Verification could not be completed",
        "The response exceeded the configured transport size limit."),
    NotFound => ("not_found", "No matching company record found",
        "The identified registry sources definitively reported no matching company record. This does not establish VAT invalidity."),
    UnresolvedData => ("unresolved_data", "Company status could not be resolved",
        "The registry record matched, but its lifecycle is missing, unsupported or ambiguous. This is not a negative registration result."),
    UnknownError => ("unknown_error", "Verification could not be completed",
        "An unclassified failure prevented verification. No validity conclusion follows."),
}
