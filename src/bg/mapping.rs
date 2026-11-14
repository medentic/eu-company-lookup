//! Pure projection from matched Bulgarian observations into the common contract.
use crate::{BgCompanyLookupResult, CompanyDetails, CompanyLookupResult, RegistryDetails};

impl From<BgCompanyLookupResult> for CompanyLookupResult {
    fn from(mut value: BgCompanyLookupResult) -> Self {
        super::address::country(&mut value.address);
        let registry_details = match value.registry_details {
            RegistryDetails::BulgarianCommercialRegister(details) => {
                CompanyDetails::BulgarianCommercialRegister {
                    native_registry_status: value.native_registry_status,
                    vat_verification: None,
                    details,
                }
            }
            RegistryDetails::Bulstat(details) => CompanyDetails::Bulstat {
                native_registry_status: value.native_registry_status,
                vat_verification: None,
                details,
            },
        };
        Self {
            country: "BG".into(),
            tax_id: value.tax_id.into(),
            vat_registered: None,
            vat_id: None,
            source: registry_details.provider(),
            registry_url: Some(value.registry_url),
            foreign_name: value.company_english_name,
            company_name: value.company_name,
            legal_form: value.legal_form.map(|v| v.as_str().to_owned()),
            registered_office: Some(value.address),
            fiscal_address: None,
            status: value.status,
            registry_details,
        }
    }
}
