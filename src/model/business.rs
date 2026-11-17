use crate::{
    BgBusinessId, CompanyAddress, CompanyStatus, IdentifierScheme, LookupError, Provider,
    RoBusinessId, RoRegistryAddress, RoTaxStatus,
};
use serde::{Deserialize, Deserializer, Serialize, de::Error};

/// Validated national business identity. Country and scheme are explicit;
/// syntax acceptance never proves registration or checksum validity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BusinessId {
    country: String,
    scheme: IdentifierScheme,
    value: String,
}

impl BusinessId {
    /// Validate a supported country/scheme pair and normalize its value.
    /// VAT identifiers are deliberately excluded; use `VatId` for VIES.
    pub fn parse(
        country: &str,
        scheme: IdentifierScheme,
        value: &str,
    ) -> Result<Self, LookupError> {
        let country = country.to_ascii_uppercase();
        if !crate::Client::SUPPORTED_COMPANY_COUNTRIES.contains(&country.as_str()) {
            return Err(LookupError::from(
                crate::model::LookupFailure::UnsupportedIdentifierScheme,
            ));
        }
        match (country.as_str(), scheme) {
            ("BG", IdentifierScheme::BgBusinessId) => Ok(BgBusinessId::parse(value)?.into()),

            ("IE", IdentifierScheme::IeBusinessId) => Ok(crate::IeBusinessId::parse(value)?.into()),

            ("NO", IdentifierScheme::NoBusinessId) => Ok(crate::NoBusinessId::parse(value)?.into()),

            ("RO", IdentifierScheme::RoBusinessId) => Ok(RoBusinessId::parse(value)?.into()),
            _ => Err(LookupError::from(
                crate::model::LookupFailure::UnsupportedIdentifierScheme,
            )),
        }
    }
    /// ISO country owning this identifier scheme, not the physical address country.
    pub fn country(&self) -> &str {
        &self.country
    }
    /// National identifier scheme; BG business codes do not alone identify a register.
    pub fn scheme(&self) -> IdentifierScheme {
        self.scheme
    }
    /// Normalized value, including leading zeroes and branch suffixes.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}
impl From<BgBusinessId> for BusinessId {
    fn from(id: BgBusinessId) -> Self {
        Self {
            country: "BG".into(),
            scheme: IdentifierScheme::BgBusinessId,
            value: id.into(),
        }
    }
}
impl From<RoBusinessId> for BusinessId {
    fn from(id: RoBusinessId) -> Self {
        Self {
            country: "RO".into(),
            scheme: IdentifierScheme::RoBusinessId,
            value: id.into(),
        }
    }
}

impl From<crate::IeBusinessId> for BusinessId {
    fn from(id: crate::IeBusinessId) -> Self {
        Self {
            country: "IE".into(),
            scheme: IdentifierScheme::IeBusinessId,
            value: id.into(),
        }
    }
}

impl From<crate::NoBusinessId> for BusinessId {
    fn from(id: crate::NoBusinessId) -> Self {
        Self {
            country: "NO".into(),
            scheme: IdentifierScheme::NoBusinessId,
            value: id.into(),
        }
    }
}

impl<'de> Deserialize<'de> for BusinessId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            country: String,
            scheme: IdentifierScheme,
            value: String,
        }
        let wire = Wire::deserialize(d)?;
        Self::parse(&wire.country, wire.scheme, &wire.value).map_err(D::Error::custom)
    }
}

/// Country-independent company observation. Tax registration and VIES validity
/// remain separate facts; missing addresses and lifecycle status remain unknown.
/// Top-level fields have provider-independent meanings. Native codes, source-specific
/// status text and other provider facts belong exclusively in `registry_details`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CompanyLookupResult {
    /// ISO country owning the national identifier, not the physical address country.
    pub country: String,
    /// Matched national business identifier, preserving leading zeroes and branch suffixes.
    pub tax_id: String,
    /// VAT registration supported by the source; null when unknown.
    /// RO reflects ANAF article 316. BG is true only after a valid, name-matched
    /// VIES check; a negative VIES result does not establish domestic nonregistration.
    pub vat_registered: Option<bool>,
    /// VAT identifier supported by the matched observation; see provider details.
    /// Null when unavailable; never derived by a generic country-prefix rule.
    pub vat_id: Option<crate::VatId>,
    /// Primary registry source; supplementary VIES evidence is in registry details.
    pub source: Provider,
    /// Public human-readable record URL, when available; never an API endpoint.
    pub registry_url: Option<String>,
    /// Separately registered foreign-script name, not a translation.
    pub foreign_name: Option<String>,
    /// Source company name without translation.
    pub company_name: String,
    /// Source legal form; no cross-country legal equivalence is inferred.
    pub legal_form: Option<String>,
    /// Registered office, distinct from a fiscal address.
    pub registered_office: Option<CompanyAddress>,
    /// Fiscal address, when explicitly supplied separately.
    pub fiscal_address: Option<CompanyAddress>,
    /// Normalized lifecycle, never inferred from domestic tax flags.
    pub status: Option<CompanyStatus>,
    /// Provider-specific facts with an explicit source discriminator.
    pub registry_details: CompanyDetails,
}

/// Typed source-specific company facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "provider", content = "data", rename_all_fields = "camelCase")]
#[non_exhaustive]
pub enum CompanyDetails {
    /// Bulgarian Commercial Register-specific facts.
    #[non_exhaustive]
    BulgarianCommercialRegister {
        /// Unmodified source status; null when unavailable.
        native_registry_status: Option<String>,
        /// Supplementary VIES response, including unmatched names and negative validity.
        /// Null when the ID is not nine digits or this is a native conversion.
        vat_verification: Option<Box<crate::VatLookupResult>>,
        /// Provider-specific fields, serialized alongside the native status.
        #[serde(flatten)]
        details: Box<crate::CommercialRegisterDetails>,
    },
    /// Bulgarian BULSTAT-specific facts.
    #[non_exhaustive]
    Bulstat {
        /// Unmodified source status; null when unavailable.
        native_registry_status: Option<String>,
        /// Supplementary VIES response, including unmatched names and negative validity.
        /// Null when the ID is not nine digits or this is a native conversion.
        vat_verification: Option<Box<crate::VatLookupResult>>,
        /// Provider-specific fields, serialized alongside the native status.
        #[serde(flatten)]
        details: crate::BulstatDetails,
    },

    /// Irish CRO public company record.
    #[non_exhaustive]
    IrishCro {
        /// Original CRO company-status label.
        native_registry_status: Option<String>,
        /// Typed native source fields and contact-address roles.
        #[serde(flatten)]
        details: Box<crate::IeCompanyDetails>,
    },

    /// Norwegian Enhetsregisteret company details.
    #[non_exhaustive]
    NorwegianBrreg {
        /// Aggregate status is unavailable; native facts remain in details.
        native_registry_status: Option<String>,
        /// Typed native source fields and contact-address roles.
        #[serde(flatten)]
        details: Box<crate::NoCompanyDetails>,
    },

    /// Romanian fiscal observation; not an ONRC company extract.
    #[non_exhaustive]
    RomanianAnaf {
        /// Unmodified source status; null when unavailable.
        native_registry_status: Option<String>,
        /// Provider-specific fields, serialized alongside the native status.
        #[serde(flatten)]
        details: Box<AnafDetails>,
    },
}

impl CompanyDetails {
    /// One authority for the provider discriminator used by adapter projections.
    pub(crate) fn provider(&self) -> Provider {
        match self {
            Self::BulgarianCommercialRegister { .. } => Provider::BulgarianCommercialRegister,
            Self::Bulstat { .. } => Provider::Bulstat,
            Self::RomanianAnaf { .. } => Provider::RomanianAnaf,

            Self::IrishCro { .. } => Provider::IrishCro,

            Self::NorwegianBrreg { .. } => Provider::NorwegianBrreg,
        }
    }
}

/// ANAF facts preserved alongside the common company view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AnafDetails {
    /// Requested fiscal date; does not date every identity field historically.
    pub request_date: chrono::NaiveDate,
    /// UTC response decoding timestamp.
    pub checked_at: chrono::DateTime<chrono::Utc>,
    /// Source trade-register reference, not independently verified.
    pub trade_register_number: Option<String>,
    /// Source registration date text, not inferred incorporation date.
    pub registration_date: Option<String>,
    /// Original ANAF registration-status text.
    pub registration_status: Option<String>,
    /// Source CAEN code.
    pub activity_code: Option<String>,
    /// Source structured office components, retaining street number separately.
    pub registered_office: Option<RoRegistryAddress>,
    /// Source structured fiscal components.
    pub fiscal_address: Option<RoRegistryAddress>,
    /// Domestic fiscal flags; never substituted for VIES validity.
    pub tax_status: RoTaxStatus,
}
