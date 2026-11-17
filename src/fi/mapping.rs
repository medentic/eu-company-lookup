use super::FiCompanyLookupResult;
use crate::{CompanyDetails, CompanyLookupResult};
impl From<FiCompanyLookupResult> for CompanyLookupResult {
    fn from(v: FiCompanyLookupResult) -> Self {
        let registry_details = CompanyDetails::FinnishPrh {
            native_registry_status: v.native_registry_status,
            details: Box::new(v.details),
        };
        Self {
            country: "FI".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            legal_form: v.legal_form,
            registered_office: None,
            fiscal_address: None,
            vat_registered: None,
            vat_id: None,
            registry_url: None,
            foreign_name: None,
            status: None,
            source: registry_details.provider(),
            registry_details,
        }
    }
}
