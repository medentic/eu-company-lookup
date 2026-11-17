use super::LvCompanyLookupResult;
use crate::{CompanyAddress, CompanyDetails, CompanyLookupResult, CompanyStatus};
impl From<LvCompanyLookupResult> for CompanyLookupResult {
    fn from(v: LvCompanyLookupResult) -> Self {
        let registered_office = v
            .details
            .address
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .map(|s| CompanyAddress {
                full_address_by_registry: Some(s.clone()),
                fields: Default::default(),
            });
        let legal_form = v.details.type_text.clone().filter(|s| !s.trim().is_empty());
        let native_registry_status = v.details.closed.clone();
        let status = native_registry_status
            .as_ref()
            .map(|_| CompanyStatus::Unknown);
        let registry_details = CompanyDetails::LatvianOpenData {
            native_registry_status,
            details: Box::new(v.details),
        };
        Self {
            country: "LV".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            source: registry_details.provider(),
            registry_details,
            registered_office,
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
