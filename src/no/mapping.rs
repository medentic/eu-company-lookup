use super::NoCompanyLookupResult;
use crate::{CompanyDetails, CompanyLookupResult};
impl From<NoCompanyLookupResult> for CompanyLookupResult {
    fn from(v: NoCompanyLookupResult) -> Self {
        let legal_form = v
            .details
            .organisasjonsform
            .as_ref()
            .and_then(|f| f.beskrivelse.clone())
            .filter(|s| !s.trim().is_empty());
        let vat_registered = v.details.registrert_i_mvaregisteret;
        let registry_details = CompanyDetails::NorwegianBrreg {
            native_registry_status: None,
            details: Box::new(v.details),
        };
        Self {
            country: "NO".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            legal_form,
            vat_registered,
            registered_office: None,
            fiscal_address: None,
            vat_id: None,
            status: None,
            registry_url: None,
            foreign_name: None,
            source: registry_details.provider(),
            registry_details,
        }
    }
}
