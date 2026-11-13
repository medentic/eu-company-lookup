use crate::{
    BgBusinessId, BgCompanyLookupResult, CompanyAddress, CompanyStatus, IdentifierScheme,
    LookupError, Provider, RegistryDetails, RoBusinessId, RoCompanyLookupResult, RoRegistryAddress,
    RoTaxStatus,
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
        match (country.to_ascii_uppercase().as_str(), scheme) {
            ("BG", IdentifierScheme::BgBusinessId) => Ok(BgBusinessId::parse(value)?.into()),
            ("RO", IdentifierScheme::RoBusinessId) => Ok(RoBusinessId::parse(value)?.into()),
            _ => Err(LookupError::UnsupportedIdentifierScheme),
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CompanyLookupResult {
    /// Validated national identity matched by the provider.
    pub identifier: BusinessId,
    /// Source of this observation.
    pub source: Provider,
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
    /// Unmodified source lifecycle value, when supplied.
    pub native_registry_status: Option<String>,
    /// Provider-specific facts with an explicit source discriminator.
    pub details: CompanyDetails,
}

/// Typed source-specific company facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "provider", content = "data", rename_all_fields = "camelCase")]
#[non_exhaustive]
pub enum CompanyDetails {
    /// Bulgarian register facts, including source record links and alternate names.
    BulgarianRegistry {
        /// Human-readable record URL.
        registry_url: String,
        /// Registered foreign-script name; not necessarily English.
        foreign_name: Option<String>,
        /// Commercial Register or BULSTAT-specific facts.
        registry_details: RegistryDetails,
    },
    /// Romanian fiscal observation; not an ONRC company extract.
    RomanianAnaf(Box<AnafDetails>),
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

impl From<BgCompanyLookupResult> for CompanyLookupResult {
    fn from(value: BgCompanyLookupResult) -> Self {
        Self {
            identifier: value.tax_id.into(),
            source: value.source,
            company_name: value.company_name,
            legal_form: value.legal_form.map(|v| v.as_str().to_owned()),
            registered_office: Some(value.address),
            fiscal_address: None,
            status: value.status,
            native_registry_status: value.native_registry_status,
            details: CompanyDetails::BulgarianRegistry {
                registry_url: value.registry_url,
                foreign_name: value.company_english_name,
                registry_details: value.registry_details,
            },
        }
    }
}

fn ro_address(value: Option<&RoRegistryAddress>, raw: Option<String>) -> Option<CompanyAddress> {
    if value.is_none() && raw.is_none() {
        return None;
    }
    let mut fields = crate::locations::Address::default();
    if let Some(value) = value {
        let street: Vec<&str> = [value.street.as_deref(), value.street_number.as_deref()]
            .into_iter()
            .flatten()
            .collect();
        fields.address = (!street.is_empty()).then(|| street.join(" "));
        fields.address2 = value.details.clone();
        fields.city = value.locality.clone();
        fields.region = value.county.clone();
        fields.postcode = value.postcode.clone();
        fields.country = value.country.clone();
        fields.country_iso2 = value
            .country
            .as_deref()
            .and_then(crate::locations::Country::from_name)
            .map(|v| v.iso2().to_owned());
    }
    Some(CompanyAddress {
        full_address_by_registry: raw,
        fields,
    })
}

impl From<RoCompanyLookupResult> for CompanyLookupResult {
    fn from(value: RoCompanyLookupResult) -> Self {
        Self {
            identifier: value.tax_id.into(),
            source: value.source,
            company_name: value.company_name,
            legal_form: value.legal_form,
            registered_office: ro_address(value.registered_office.as_ref(), None),
            fiscal_address: ro_address(value.fiscal_address.as_ref(), value.full_fiscal_address),
            status: value.status,
            native_registry_status: value.native_registry_status,
            details: CompanyDetails::RomanianAnaf(Box::new(AnafDetails {
                request_date: value.request_date,
                checked_at: value.checked_at,
                trade_register_number: value.trade_register_number,
                registration_date: value.registration_date,
                registration_status: value.registration_status,
                activity_code: value.activity_code,
                registered_office: value.registered_office,
                fiscal_address: value.fiscal_address,
                tax_status: value.tax_status,
            })),
        }
    }
}
