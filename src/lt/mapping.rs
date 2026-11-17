use super::LtCompanyLookupResult;
use crate::{CompanyDetails, CompanyLookupResult};
impl From<LtCompanyLookupResult> for CompanyLookupResult {
    fn from(v: LtCompanyLookupResult) -> Self {
        let registry_details = CompanyDetails::LithuanianJar {
            native_registry_status: None,
            details: Box::new(v.details),
        };
        Self {
            country: "LT".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            registry_details,
            source: crate::Provider::LithuanianJar,
            registered_office: None,
            fiscal_address: None,
            status: None,
            legal_form: None,
            vat_registered: None,
            vat_id: None,
            foreign_name: None,
            registry_url: None,
        }
    }
}
