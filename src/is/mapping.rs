use super::IsCompanyLookupResult;
use crate::{CompanyAddress, CompanyDetails, CompanyLookupResult};
impl From<IsCompanyLookupResult> for CompanyLookupResult {
    fn from(v: IsCompanyLookupResult) -> Self {
        let legal_form = v.details.legal_form_description.clone();
        let registered_office =
            (!v.details.legal_address.trim().is_empty()).then(|| CompanyAddress {
                full_address_by_registry: Some(v.details.legal_address.clone()),
                fields: Default::default(),
            });
        let registry_details = CompanyDetails::IcelandicSkatturinn {
            native_registry_status: None,
            details: Box::new(v.details),
        };
        Self {
            country: "IS".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            registered_office,
            legal_form,
            source: registry_details.provider(),
            registry_details,
            fiscal_address: None,
            vat_registered: None,
            vat_id: None,
            status: None,
            registry_url: None,
            foreign_name: None,
        }
    }
}
