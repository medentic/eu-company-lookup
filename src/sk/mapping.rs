use super::SkCompanyLookupResult;
use crate::{CompanyDetails, CompanyLookupResult};
impl From<SkCompanyLookupResult> for CompanyLookupResult {
    fn from(v: SkCompanyLookupResult) -> Self {
        let legal_form = v
            .details
            .pravna_forma
            .as_deref()
            .and_then(super::legal_forms::label)
            .map(str::to_owned);
        // RÚZ documents this as dissolution of the accounting entity, not deletion
        // of the API record (`stav`). Missing dates do not establish active status.
        let status = v.details.datum_zrusenia.as_deref().map(|date| {
            if jiff::civil::Date::strptime("%Y-%m-%d", date)
                .is_ok_and(|d| d.strftime("%Y-%m-%d").to_string() == date)
            {
                crate::CompanyStatus::Closed
            } else {
                crate::CompanyStatus::Unknown
            }
        });
        let status = status.or_else(|| v.details.rpo_lifecycle.as_ref().map(super::rpo::status));
        let native_registry_status = v
            .details
            .rpo_lifecycle
            .as_ref()
            .and_then(|r| r.legal_statuses.as_deref())
            .and_then(|states| match states {
                [state] => Some(state.value.value.clone()),
                _ => None,
            });
        let registry_details = CompanyDetails::SlovakRuz {
            native_registry_status,
            details: Box::new(v.details),
        };
        Self {
            country: "SK".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            registry_details,
            source: crate::Provider::SlovakRuz,
            registered_office: None,
            fiscal_address: None,
            status,
            legal_form,
            vat_registered: None,
            vat_id: None,
            foreign_name: None,
            registry_url: None,
        }
    }
}
