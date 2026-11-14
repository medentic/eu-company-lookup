use super::RoBusinessId;
use crate::{CompanyStatus, Provider};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;

/// ANAF taxpayer observation, distinct from a VIES validity check or ONRC extract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct RoCompanyLookupResult {
    /// Requested CUI, retaining leading zeroes.
    pub tax_id: RoBusinessId,
    /// ANAF, the source of this observation.
    pub source: Provider,
    /// Date for which the fiscal status was requested and matched.
    pub request_date: NaiveDate,
    /// UTC time the response was decoded.
    pub checked_at: DateTime<Utc>,
    /// Source company name, without translation.
    pub company_name: String,
    /// Trade register number reported by ANAF; not independently checked at ONRC.
    pub trade_register_number: Option<String>,
    /// Shared normalized lifecycle; unknown source descriptions are `Unknown`.
    pub status: Option<CompanyStatus>,
    /// Original registry status, regardless of normalization; numeric source codes are stringified.
    /// Missing source status remains null. No translated or inferred labels are invented.
    pub native_registry_status: Option<String>,
    /// Raw ANAF registration-status description; not the normalized lifecycle status.
    pub registration_status: Option<String>,
    /// Source registration date text; not assumed to be incorporation date.
    pub registration_date: Option<String>,
    /// Source legal-form description.
    pub legal_form: Option<String>,
    /// Source CAEN activity code.
    pub activity_code: Option<String>,
    /// Original decoded fiscal-address text from ANAF, without normalization.
    pub full_fiscal_address: Option<String>,
    /// Structured registered-office fields supplied by ANAF.
    pub registered_office: Option<RoRegistryAddress>,
    /// Structured fiscal-address fields supplied by ANAF.
    pub fiscal_address: Option<RoRegistryAddress>,
    /// Fiscal flags at the requested date. Missing flags remain null.
    pub tax_status: RoTaxStatus,
}

/// ANAF address components; missing country defaults to Romania (RO).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct RoRegistryAddress {
    /// Source street name.
    pub street: Option<String>,
    /// Source street number, kept as text.
    pub street_number: Option<String>,
    /// Source locality name.
    pub locality: Option<String>,
    /// Source county name.
    pub county: Option<String>,
    /// Title-case country name; defaults to Romania when ANAF supplies none.
    pub country: Option<String>,
    /// Uppercase ISO country code when the supplied country is recognized.
    #[serde(rename = "countryISO2")]
    pub country_iso2: Option<String>,
    /// Source postcode, retaining leading zeroes.
    pub postcode: Option<String>,
    /// Source additional address details.
    pub details: Option<String>,
}

/// ANAF fiscal flags, distinct from VIES validity and legal existence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct RoTaxStatus {
    /// Registration under the domestic VAT regime reported by ANAF.
    pub vat_registered: Option<bool>,
    /// Applies VAT on collection at the requested date.
    pub vat_on_collection: Option<bool>,
    /// Listed as fiscally inactive; not a dissolved-company assertion.
    pub inactive: Option<bool>,
    /// Applies split VAT payment at the requested date.
    pub split_vat: Option<bool>,
    /// Listed in the RO e-Factura register at the requested date.
    pub e_invoice_registered: Option<bool>,
}

impl RoCompanyLookupResult {
    /// The matched ANAF article-316 fiscal registration supplies the numeric ID.
    /// This is not a rule for arbitrary company IDs or special article-317 codes.
    pub(crate) fn registered_vat_id(&self) -> Option<crate::VatId> {
        if self.tax_status.vat_registered != Some(true) {
            return None;
        }
        // ANAF matches CUI numerically. Keep the caller's spelling in tax_id,
        // but use that matched numeric identity without input padding for VAT.
        let number = self.tax_id.as_str().trim_start_matches('0');
        crate::VatId::parse(&format!("RO{number}")).ok()
    }
}
