//! Preserve the official address independently of the source's map text.
use super::GrCompanyLookupResult;
use crate::{CompanyAddress, CompanyDetails, CompanyLookupResult};

impl From<GrCompanyLookupResult> for CompanyLookupResult {
    fn from(v: GrCompanyLookupResult) -> Self {
        let registered_office = v.full_address_by_registry.map(|raw| CompanyAddress {
            full_address_by_registry: Some(raw),
            fields: Default::default(),
        });
        let registry_url = Some(format!(
            "https://publicity.businessportal.gr/company/{}",
            v.tax_id.as_str()
        ));
        let registry_details = CompanyDetails::GreekGemi {
            native_registry_status: v.native_registry_status,
            details: Box::new(v.details),
        };
        Self {
            country: "GR".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            foreign_name: v.foreign_name,
            legal_form: v.legal_form,
            registered_office,
            registry_url,
            vat_registered: None,
            vat_id: None,
            fiscal_address: None,
            status: None,
            source: registry_details.provider(),
            registry_details,
        }
    }
}
