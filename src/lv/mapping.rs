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
        // The dataset uses a blank marker for no stated closure reason.
        // Preserve it in details.closed, but it is not a lifecycle status.
        let native_registry_status = v.details.closed.clone().filter(|s| !s.trim().is_empty());
        let status = native_registry_status
            .as_ref()
            .map(|_| CompanyStatus::Unknown);
        let closure = v
            .details
            .terminated
            .as_deref()
            .filter(|date| {
                chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                    .is_ok_and(|d| d.format("%Y-%m-%d").to_string() == *date)
                    || chrono::NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S")
                        .is_ok_and(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string() == *date)
            })
            .map(|_| crate::CompanyStatus::Closed);
        let malformed_date =
            (v.details.terminated.is_some() && closure.is_none()).then_some(CompanyStatus::Unknown);
        let resolved = malformed_date
            .or(closure)
            .or_else(|| super::lifecycle::status(&v.details))
            .or(status);
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
            status: resolved,
            fiscal_address: None,
            vat_registered: None,
            vat_id: None,
            registry_url: None,
            foreign_name: None,
        }
    }
}
