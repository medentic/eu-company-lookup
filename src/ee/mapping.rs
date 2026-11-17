use super::EeCompanyLookupResult;
use crate::{CompanyAddress, CompanyDetails, CompanyLookupResult, CompanyStatus};
impl From<EeCompanyLookupResult> for CompanyLookupResult {
    fn from(v: EeCompanyLookupResult) -> Self {
        let registered_office = (v.details.legal_address.is_some() || v.details.zip_code.is_some())
            .then(|| CompanyAddress {
                full_address_by_registry: v.details.legal_address.clone(),
                fields: crate::locations::Address {
                    postcode: v.details.zip_code.clone(),
                    ..Default::default()
                },
            });
        let native_registry_status = v.details.status.clone();
        let status = native_registry_status
            .as_ref()
            .map(|_| CompanyStatus::Unknown);
        let registry_details = CompanyDetails::EstonianRik {
            native_registry_status,
            details: Box::new(v.details),
        };
        Self {
            country: "EE".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            registered_office,
            status,
            legal_form: None,
            fiscal_address: None,
            vat_registered: None,
            vat_id: None,
            registry_url: None,
            foreign_name: None,
            source: registry_details.provider(),
            registry_details,
        }
    }
}
