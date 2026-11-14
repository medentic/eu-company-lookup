//! Pure projection of matched ANAF facts; no other provider is queried or merged.
use crate::{
    AnafDetails, CompanyAddress, CompanyDetails, CompanyLookupResult, RoCompanyLookupResult,
    RoRegistryAddress,
};

fn ro_address(value: Option<&RoRegistryAddress>, raw: Option<String>) -> Option<CompanyAddress> {
    if value.is_none() && raw.is_none() {
        return None;
    }
    let mut fields = crate::locations::Address::default();
    if let Some(value) = value {
        let street: Vec<&str> = [value.street.as_deref(), value.street_number.as_deref()]
            .into_iter()
            .flatten()
            .collect();
        fields.address = (!street.is_empty()).then(|| street.join(" "));
        fields.address2 = value.details.clone();
        fields.city = value.locality.clone();
        fields.region = value.county.clone();
        fields.postcode = value.postcode.clone();
        fields.country = value.country.clone();
        fields.country_iso2 = value.country_iso2.clone();
    }
    if fields
        .country
        .as_deref()
        .is_none_or(|name| name.trim().is_empty())
        && fields.country_iso2.is_none()
    {
        fields.country = Some("Romania".into());
    }
    fields.normalize_country();
    Some(CompanyAddress {
        full_address_by_registry: raw,
        fields,
    })
}

impl From<RoCompanyLookupResult> for CompanyLookupResult {
    fn from(value: RoCompanyLookupResult) -> Self {
        let vat_id = value.registered_vat_id();
        let vat_registered = value.tax_status.vat_registered;
        let registered_office = ro_address(value.registered_office.as_ref(), None);
        let fiscal_address = ro_address(value.fiscal_address.as_ref(), value.full_fiscal_address);
        let registry_details = CompanyDetails::RomanianAnaf {
            native_registry_status: value.native_registry_status,
            details: Box::new(AnafDetails {
                request_date: value.request_date,
                checked_at: value.checked_at,
                trade_register_number: value.trade_register_number,
                registration_date: value.registration_date,
                registration_status: value.registration_status,
                activity_code: value.activity_code,
                registered_office: value.registered_office,
                fiscal_address: value.fiscal_address,
                tax_status: value.tax_status,
            }),
        };
        Self {
            country: "RO".into(),
            tax_id: value.tax_id.into(),
            vat_registered,
            vat_id,
            source: registry_details.provider(),
            registry_url: None,
            foreign_name: None,
            company_name: value.company_name,
            legal_form: value.legal_form,
            registered_office,
            fiscal_address,
            status: value.status,
            registry_details,
        }
    }
}
