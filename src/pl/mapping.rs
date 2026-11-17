use super::PlCompanyLookupResult;
use crate::{CompanyAddress, CompanyDetails, CompanyLookupResult};
impl From<PlCompanyLookupResult> for CompanyLookupResult {
    fn from(v: PlCompanyLookupResult) -> Self {
        let legal_form = v
            .details
            .dane_podmiotu
            .forma_prawna
            .clone()
            .filter(|s| !s.trim().is_empty());
        let registered_office = v
            .details
            .siedziba_i_adres
            .as_ref()
            .and_then(|a| a.adres.as_ref())
            .map(|a| {
                let delivery: Vec<_> = [&a.ulica, &a.nr_domu]
                    .into_iter()
                    .flatten()
                    .filter(|s| !s.trim().is_empty())
                    .map(String::as_str)
                    .collect();
                let mut fields = crate::locations::Address {
                    address: (!delivery.is_empty()).then(|| delivery.join(" ")),
                    address2: a.nr_lokalu.clone(),
                    city: a.miejscowosc.clone(),
                    postcode: a.kod_pocztowy.clone(),
                    country: a.kraj.clone(),
                    ..Default::default()
                };
                fields.normalize_country();
                CompanyAddress {
                    full_address_by_registry: None,
                    fields,
                }
            });
        let registry_details = CompanyDetails::PolishKrs {
            native_registry_status: None,
            details: Box::new(v.details),
        };
        Self {
            country: "PL".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            legal_form,
            registered_office,
            fiscal_address: None,
            vat_id: None,
            vat_registered: None,
            status: None,
            registry_url: None,
            foreign_name: None,
            source: registry_details.provider(),
            registry_details,
        }
    }
}
