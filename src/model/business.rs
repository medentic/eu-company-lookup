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
            ("GR", IdentifierScheme::GrBusinessId) => Ok(crate::GrBusinessId::parse(value)?.into()),
            ("BE", IdentifierScheme::BeBusinessId) => Ok(crate::BeBusinessId::parse(value)?.into()),
            ("FI", IdentifierScheme::FiBusinessId) => Ok(crate::FiBusinessId::parse(value)?.into()),
            ("FR", IdentifierScheme::FrBusinessId) => Ok(crate::FrBusinessId::parse(value)?.into()),
            ("IS", IdentifierScheme::IsBusinessId) => Ok(crate::IsBusinessId::parse(value)?.into()),
            ("IE", IdentifierScheme::IeBusinessId) => Ok(crate::IeBusinessId::parse(value)?.into()),
            ("LT", IdentifierScheme::LtBusinessId) => Ok(crate::LtBusinessId::parse(value)?.into()),
            ("LV", IdentifierScheme::LvBusinessId) => Ok(crate::LvBusinessId::parse(value)?.into()),
            ("SK", IdentifierScheme::SkBusinessId) => Ok(crate::SkBusinessId::parse(value)?.into()),
            ("PL", IdentifierScheme::PlBusinessId) => Ok(crate::PlBusinessId::parse(value)?.into()),
            ("NO", IdentifierScheme::NoBusinessId) => Ok(crate::NoBusinessId::parse(value)?.into()),
            ("EE", IdentifierScheme::EeBusinessId) => Ok(crate::EeBusinessId::parse(value)?.into()),
            ("AT", IdentifierScheme::AtBusinessId) => Ok(crate::AtBusinessId::parse(value)?.into()),
            ("CZ", IdentifierScheme::CzBusinessId) => Ok(crate::CzBusinessId::parse(value)?.into()),
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
impl From<crate::CzBusinessId> for BusinessId {
    fn from(id: crate::CzBusinessId) -> Self {
        Self {
            country: "CZ".into(),
            scheme: IdentifierScheme::CzBusinessId,
            value: id.into(),
        }
    }
}
impl From<crate::AtBusinessId> for BusinessId {
    fn from(id: crate::AtBusinessId) -> Self {
        Self {
            country: "AT".into(),
            scheme: IdentifierScheme::AtBusinessId,
            value: id.into(),
        }
    }
}
impl From<crate::BeBusinessId> for BusinessId {
    fn from(id: crate::BeBusinessId) -> Self {
        Self {
            country: "BE".into(),
            scheme: IdentifierScheme::BeBusinessId,
            value: id.into(),
        }
    }
}

impl From<crate::FiBusinessId> for BusinessId {
    fn from(id: crate::FiBusinessId) -> Self {
        Self {
            country: "FI".into(),
            scheme: IdentifierScheme::FiBusinessId,
            value: id.into(),
        }
    }
}

impl From<crate::FrBusinessId> for BusinessId {
    fn from(id: crate::FrBusinessId) -> Self {
        Self {
            country: "FR".into(),
            scheme: IdentifierScheme::FrBusinessId,
            value: id.into(),
        }
    }
}

impl From<crate::IsBusinessId> for BusinessId {
    fn from(id: crate::IsBusinessId) -> Self {
        Self {
            country: "IS".into(),
            scheme: IdentifierScheme::IsBusinessId,
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

impl From<crate::LtBusinessId> for BusinessId {
    fn from(id: crate::LtBusinessId) -> Self {
        Self {
            country: "LT".into(),
            scheme: IdentifierScheme::LtBusinessId,
            value: id.into(),
        }
    }
}

impl From<crate::LvBusinessId> for BusinessId {
    fn from(id: crate::LvBusinessId) -> Self {
        Self {
            country: "LV".into(),
            scheme: IdentifierScheme::LvBusinessId,
            value: id.into(),
        }
    }
}

impl From<crate::SkBusinessId> for BusinessId {
    fn from(id: crate::SkBusinessId) -> Self {
        Self {
            country: "SK".into(),
            scheme: IdentifierScheme::SkBusinessId,
            value: id.into(),
        }
    }
}

impl From<crate::PlBusinessId> for BusinessId {
    fn from(id: crate::PlBusinessId) -> Self {
        Self {
            country: "PL".into(),
            scheme: IdentifierScheme::PlBusinessId,
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

impl From<crate::EeBusinessId> for BusinessId {
    fn from(id: crate::EeBusinessId) -> Self {
        Self {
            country: "EE".into(),
            scheme: IdentifierScheme::EeBusinessId,
            value: id.into(),
        }
    }
}
impl From<crate::GrBusinessId> for BusinessId {
    fn from(id: crate::GrBusinessId) -> Self {
        Self {
            country: "GR".into(),
            scheme: IdentifierScheme::GrBusinessId,
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
/// remain separate facts. Public lookups require resolved lifecycle; pure conversions
/// retain unresolved observations and optional facts remain nullable.
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
    /// Greek GEMI public company details.
    #[non_exhaustive]
    GreekGemi {
        /// Original source status label.
        native_registry_status: Option<String>,
        /// Native tax number, dates and codes.
        #[serde(flatten)]
        details: Box<crate::GrGemiDetails>,
    },
    /// Finnish PRH YTJ company details.
    #[non_exhaustive]
    FinnishPrh {
        /// Original Trade Register status code.
        native_registry_status: Option<String>,
        /// Typed native source fields and contact-address roles.
        #[serde(flatten)]
        details: Box<crate::FiPrhDetails>,
    },
    /// French API Recherche Entreprises company details.
    #[non_exhaustive]
    FrenchRechercheEntreprises {
        /// Original administrative status code.
        native_registry_status: Option<String>,
        /// Typed native source fields and contact-address roles.
        #[serde(flatten)]
        details: Box<crate::FrCompanyDetails>,
    },
    /// Icelandic public company-page fields.
    #[non_exhaustive]
    IcelandicSkatturinn {
        /// Aggregate status is unavailable; selected page facts remain in details.
        native_registry_status: Option<String>,
        /// Typed native source fields and contact-address roles.
        #[serde(flatten)]
        details: Box<crate::IsCompanyDetails>,
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
    /// Lithuanian published legal-entity record.
    #[non_exhaustive]
    LithuanianJar {
        /// Aggregate status is unavailable; opaque source status remains in details.
        native_registry_status: Option<String>,
        /// Typed native source fields and contact-address roles.
        #[serde(flatten)]
        details: Box<crate::LtCompanyDetails>,
    },
    /// Latvian official registry open-data record.
    #[non_exhaustive]
    LatvianOpenData {
        /// Original source closed marker, without inferred lifecycle.
        native_registry_status: Option<String>,
        /// Typed native source fields and contact-address roles.
        #[serde(flatten)]
        details: Box<crate::LvCompanyDetails>,
    },
    /// Slovak public accounting-entity observations.
    #[non_exhaustive]
    SlovakRuz {
        /// Aggregate status is unavailable; source observations remain in details.
        native_registry_status: Option<String>,
        /// Typed native source fields and contact-address roles.
        #[serde(flatten)]
        details: Box<crate::SkCompanyDetails>,
    },
    /// Polish current KRS business extract.
    #[non_exhaustive]
    PolishKrs {
        /// Aggregate status is unavailable; native facts remain in details.
        native_registry_status: Option<String>,
        /// Typed native source fields and contact-address roles.
        #[serde(flatten)]
        details: Box<crate::PlCompanyDetails>,
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
    /// Estonian public portal company details.
    #[non_exhaustive]
    EstonianRik {
        /// Original public-register status label.
        native_registry_status: Option<String>,
        /// Typed native source fields and contact-address roles.
        #[serde(flatten)]
        details: Box<crate::EeCompanyDetails>,
    },
    /// NBB Consult company data derived from CBE.
    #[non_exhaustive]
    BelgianNbb {
        /// Original legal-situation label.
        native_registry_status: Option<String>,
        /// Original structured office, retaining street and box separately.
        registered_office: crate::BeRegistryAddress,
        /// Native codes and dates.
        #[serde(flatten)]
        details: Box<crate::BeNbbDetails>,
    },
    /// Austrian public Firmenbuch source details.
    #[non_exhaustive]
    AustrianFirmenbuch {
        /// Original source status code.
        native_registry_status: Option<String>,
        /// Source record metadata and structured address.
        #[serde(flatten)]
        details: Box<crate::AtFirmenbuchDetails>,
    },
    /// Czech ARES economic-subject source details.
    #[non_exhaustive]
    CzechAres {
        /// No single lifecycle status is reported by this ARES endpoint.
        native_registry_status: Option<String>,
        /// Native codes, dates and source registrations.
        #[serde(flatten)]
        details: Box<crate::CzAresDetails>,
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
            Self::CzechAres { .. } => Provider::CzechAres,
            Self::AustrianFirmenbuch { .. } => Provider::AustrianFirmenbuch,
            Self::BelgianNbb { .. } => Provider::BelgianNbb,
            Self::FinnishPrh { .. } => Provider::FinnishPrh,
            Self::FrenchRechercheEntreprises { .. } => Provider::FrenchRechercheEntreprises,
            Self::IcelandicSkatturinn { .. } => Provider::IcelandicSkatturinn,
            Self::IrishCro { .. } => Provider::IrishCro,
            Self::LithuanianJar { .. } => Provider::LithuanianJar,
            Self::LatvianOpenData { .. } => Provider::LatvianOpenData,
            Self::SlovakRuz { .. } => Provider::SlovakRuz,
            Self::PolishKrs { .. } => Provider::PolishKrs,
            Self::NorwegianBrreg { .. } => Provider::NorwegianBrreg,
            Self::EstonianRik { .. } => Provider::EstonianRik,
            Self::GreekGemi { .. } => Provider::GreekGemi,
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

impl CompanyDetails {
    pub(crate) fn native_status(&self) -> Option<&str> {
        match self {
            Self::BulgarianCommercialRegister {
                native_registry_status,
                ..
            }
            | Self::Bulstat {
                native_registry_status,
                ..
            }
            | Self::GreekGemi {
                native_registry_status,
                ..
            }
            | Self::FinnishPrh {
                native_registry_status,
                ..
            }
            | Self::FrenchRechercheEntreprises {
                native_registry_status,
                ..
            }
            | Self::IcelandicSkatturinn {
                native_registry_status,
                ..
            }
            | Self::IrishCro {
                native_registry_status,
                ..
            }
            | Self::LithuanianJar {
                native_registry_status,
                ..
            }
            | Self::LatvianOpenData {
                native_registry_status,
                ..
            }
            | Self::SlovakRuz {
                native_registry_status,
                ..
            }
            | Self::PolishKrs {
                native_registry_status,
                ..
            }
            | Self::NorwegianBrreg {
                native_registry_status,
                ..
            }
            | Self::EstonianRik {
                native_registry_status,
                ..
            }
            | Self::BelgianNbb {
                native_registry_status,
                ..
            }
            | Self::AustrianFirmenbuch {
                native_registry_status,
                ..
            }
            | Self::CzechAres {
                native_registry_status,
                ..
            }
            | Self::RomanianAnaf {
                native_registry_status,
                ..
            } => native_registry_status.as_deref(),
        }
    }
}
