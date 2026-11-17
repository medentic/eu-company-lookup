//! Source-only projection; Firmenbuch registration does not establish VAT status.
use super::{AtCompanyLookupResult, AtFirmenbuchDetails};
use crate::{CompanyAddress, CompanyDetails, CompanyLookupResult};

impl From<AtCompanyLookupResult> for CompanyLookupResult {
    fn from(v: AtCompanyLookupResult) -> Self {
        let registered_office = v.registered_office.as_ref().map(|a| CompanyAddress {
            full_address_by_registry: None,
            fields: crate::locations::Address {
                address: a.street.clone(),
                city: a.city.clone(),
                postcode: a.zip_code.clone(),
                ..Default::default()
            },
        });
        let legal_form = v.legal_form.as_ref().and_then(|f| f.name.clone());
        let registry_details = CompanyDetails::AustrianFirmenbuch {
            native_registry_status: Some(v.native_registry_status),
            details: Box::new(AtFirmenbuchDetails {
                record_id: v.record_id,
                domicile: v.domicile,
                source_date: v.source_date,
                legal_form: v.legal_form,
                registered_office: v.registered_office,
            }),
        };
        Self {
            country: "AT".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            vat_registered: None,
            vat_id: None,
            source: registry_details.provider(),
            registry_url: None,
            foreign_name: None,
            legal_form,
            registered_office,
            fiscal_address: None,
            status: None,
            registry_details,
        }
    }
}
