use crate::{BgBusinessId, CompanyStatus, LegalForm, Provider, RegistryDocumentType, RegistryRole};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use serde::Serialize;

/// Registered-office address, independent of the VIES response model.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CompanyAddress {
    /// Decoded registry address text, preserving line boundaries. Never rendered HTML.
    pub full_address_by_registry: Option<String>,
    /// Shared normalized address components.
    #[serde(flatten)]
    pub fields: crate::locations::Address,
}

/// Full Bulgarian company lookup result, matched to the requested business identifier.
/// Includes registered identity and provider-specific details; existence does not establish VAT validity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct BgCompanyLookupResult {
    /// Complete requested business identifier, including any branch suffix.
    pub tax_id: BgBusinessId,
    /// Register that supplied the record.
    pub source: Provider,
    /// Public human-readable record URL, without session tokens or credentials.
    pub registry_url: String,
    /// Source legal name; Commercial Register uses its Bulgarian full name.
    pub company_name: String,
    /// Registered foreign-script name, when separately supplied; not guaranteed English.
    pub company_english_name: Option<String>,
    /// Recognized Bulgarian legal-form abbreviation; otherwise the source description.
    pub legal_form: Option<LegalForm>,
    /// Registered office, with unrecognized text retained.
    pub address: CompanyAddress,
    /// Shared normalized lifecycle status; source-specific values remain in registry details.
    pub status: Option<CompanyStatus>,
    /// Original registry status, regardless of normalization; numeric source codes are stringified.
    /// Missing source status remains null. No translated or inferred labels are invented.
    pub native_registry_status: Option<String>,
    /// Provider-specific facts; these do not override the shared identity fields.
    pub registry_details: RegistryDetails,
}

/// Source-specific fields kept separate from the shared business output.
///
/// Match with a fallback arm to allow future providers:
///
/// ```compile_fail
/// use eu_company_lookup::RegistryDetails;
/// fn source(details: &RegistryDetails) -> &str {
///     match details {
///         RegistryDetails::BulgarianCommercialRegister(_) => "commercial",
///         RegistryDetails::Bulstat(_) => "bulstat",
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged, rename_all_fields = "camelCase")]
#[non_exhaustive]
pub enum RegistryDetails {
    /// Commercial Register details; serialized without another source discriminator.
    BulgarianCommercialRegister(Box<CommercialRegisterDetails>),
    /// BULSTAT details.
    Bulstat(BulstatDetails),
}

/// Typed Commercial Register facts. Entry year is derived during serialization.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CommercialRegisterDetails {
    /// Original numeric deed status from the Commercial Register.
    pub status_code: i64,
    /// Registered name without the legal-form suffix.
    pub registered_name: Option<String>,
    /// Current UIC field's entry date, not necessarily original incorporation.
    pub identifier_entry_date: Option<NaiveDate>,
    /// Field 7 managers; null if absent, erased or not interpretable as a list.
    pub managers: Option<Vec<RegistryPerson>>,
    /// Field 23 sole capital owners; other ownership forms are not yet parsed.
    pub owners: Option<Vec<RegistryPerson>>,
    /// Registered and paid-in capital, with original text retained.
    pub capital: RegistryCapital,
    /// Full source business-activity text, not a generated summary.
    pub business_activity: Option<String>,
    /// Current constitutive-document references; no documents are downloaded.
    pub documents: Option<Vec<RegistryDocument>>,
    /// Source indicator only; does not say whether instructions are unresolved.
    pub has_instructions: Option<bool>,
}
impl CommercialRegisterDetails {
    /// Calendar year of the identifier entry, when known.
    pub fn identifier_entry_year(&self) -> Option<i32> {
        self.identifier_entry_date.map(|d| d.year())
    }
}
impl Serialize for CommercialRegisterDetails {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut value = serializer.serialize_struct("CommercialRegisterDetails", 11)?;
        value.serialize_field("statusCode", &self.status_code)?;
        value.serialize_field("registeredName", &self.registered_name)?;
        value.serialize_field("identifierEntryDate", &self.identifier_entry_date)?;
        value.serialize_field("identifierEntryYear", &self.identifier_entry_year())?;
        value.serialize_field("managers", &self.managers)?;
        value.serialize_field("owners", &self.owners)?;
        value.serialize_field("capital", &self.capital)?;
        value.serialize_field("businessActivity", &self.business_activity)?;
        value.serialize_field("documents", &self.documents)?;
        value.serialize_field("hasInstructions", &self.has_instructions)?;
        value.end()
    }
}
/// Source-specific BULSTAT text, preserved intact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct BulstatDetails {
    /// Original decoded subject description.
    pub subject_text: String,
    /// Original decoded BULSTAT status, including when mapped to a typed status.
    pub status_text: Option<String>,
}

/// Person or entity displayed in a registry role. No separate personal-identifier
/// fields are extracted; source descriptions are retained as supplied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct RegistryPerson {
    /// Name, when separable from other source details without guessing.
    pub name: Option<String>,
    /// Source role identified by the selected field.
    pub role: RegistryRole,
    /// Source country name.
    pub country: Option<String>,
    /// Country code only when the source country is recognized.
    #[serde(rename = "countryISO2")]
    pub country_iso2: Option<String>,
    /// Decoded source description, retained when name or country is ambiguous.
    pub source_text: String,
}

/// Source capital fields; unavailable or erased amounts remain null.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct RegistryCapital {
    /// Registered capital (field 31).
    pub registered: Option<RegistryMoney>,
    /// Paid-in capital (field 32).
    pub paid_in: Option<RegistryMoney>,
}

/// Exact decimal money without floating-point conversion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct RegistryMoney {
    /// Decimal amount, or null for an unrecognized representation.
    pub amount: Option<String>,
    /// ISO currency code, only when explicitly recognized in the source.
    pub currency: Option<String>,
    /// Original decoded capital field, including its currency marker.
    pub source_text: String,
}

/// Public document reference from the registry response, not fetched contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct RegistryDocument {
    /// Document category, currently constitutiveDocument.
    #[serde(rename = "type")]
    pub document_type: RegistryDocumentType,
    /// Source link title.
    pub title: String,
    /// Source local announcement time; no timezone is invented.
    pub announced_at_local: Option<NaiveDateTime>,
    /// Absolute official registry URL; access may depend on registry requirements.
    pub url: String,
}

impl std::ops::Deref for CompanyAddress {
    type Target = crate::locations::Address;
    fn deref(&self) -> &Self::Target {
        &self.fields
    }
}
impl std::ops::DerefMut for CompanyAddress {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.fields
    }
}

/// Backward-compatible name for [`BgCompanyLookupResult`]. Prefer the lookup result name in new code.
pub type CompanyRecord = BgCompanyLookupResult;
