use super::IeCompanyLookupResult;
use crate::{CompanyDetails, CompanyLookupResult, CompanyStatus};
impl From<IeCompanyLookupResult> for CompanyLookupResult {
    fn from(v: IeCompanyLookupResult) -> Self {
        let legal_form = v
            .details
            .company_type
            .clone()
            .filter(|s| !s.trim().is_empty());
        let native_registry_status = v.details.company_status.clone();
        let status = native_registry_status
            .as_ref()
            .map(|_| CompanyStatus::Unknown);
        let registry_details = CompanyDetails::IrishCro {
            native_registry_status,
            details: Box::new(v.details),
        };
        Self {
            country: "IE".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            source: registry_details.provider(),
            registry_details,
            registered_office: None,
            legal_form,
            status,
            fiscal_address: None,
            vat_registered: None,
            vat_id: None,
            registry_url: None,
            foreign_name: None,
        }
    }
}
