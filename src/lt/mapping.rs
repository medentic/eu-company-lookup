use super::LtCompanyLookupResult;
use crate::{CompanyDetails, CompanyLookupResult};
impl From<LtCompanyLookupResult> for CompanyLookupResult {
    fn from(v: LtCompanyLookupResult) -> Self {
        let registered_office = v
            .details
            .pilnas_adresas
            .as_ref()
            .filter(|text| !text.trim().is_empty())
            .map(|text| crate::CompanyAddress {
                full_address_by_registry: Some(text.clone()),
                fields: Default::default(),
            });
        let closure = v
            .details
            .isreg_data
            .as_deref()
            .filter(|date| {
                chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                    .is_ok_and(|d| d.format("%Y-%m-%d").to_string() == *date)
                    || chrono::NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S")
                        .is_ok_and(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string() == *date)
            })
            .map(|_| crate::CompanyStatus::Closed);
        let native = v
            .details
            .statusas
            .as_ref()
            .and_then(|r| super::statuses::lookup(&r.id));
        let status = if v.details.isreg_data.is_some() && closure.is_none() {
            Some(crate::CompanyStatus::Unknown)
        } else {
            closure
                .or_else(|| native.map(|(_, state)| state))
                .or_else(|| {
                    v.details
                        .statusas
                        .as_ref()
                        .map(|_| crate::CompanyStatus::Unknown)
                })
        };
        let native_registry_status = native
            .map(|(label, _)| label.to_owned())
            .or_else(|| v.details.statusas.as_ref().map(|r| r.id.clone()));
        let registry_details = CompanyDetails::LithuanianJar {
            native_registry_status,
            details: Box::new(v.details),
        };
        Self {
            country: "LT".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            registry_details,
            source: crate::Provider::LithuanianJar,
            registered_office,
            fiscal_address: None,
            status,
            legal_form: None,
            vat_registered: None,
            vat_id: None,
            foreign_name: None,
            registry_url: None,
        }
    }
}
