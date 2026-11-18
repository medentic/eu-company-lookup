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
        let status = lifecycle(v.details.registration_text.as_deref());
        let native_registry_status = status.and(v.details.registration_text.clone());
        let registry_details = CompanyDetails::IcelandicSkatturinn {
            native_registry_status,
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
            status,
            registry_url: None,
            foreign_name: None,
        }
    }
}

fn lifecycle(text: Option<&str>) -> Option<crate::CompanyStatus> {
    let text = text?.trim().strip_prefix("Stofnað/Skráð:")?.trim();
    let (registered, notice) = text.split_once('(')?;
    let closed = notice.strip_prefix("Félag afskráð ")?.strip_suffix(')');
    let valid = |s: &str| {
        chrono::NaiveDate::parse_from_str(s, "%d.%m.%Y")
            .is_ok_and(|d| d.format("%d.%m.%Y").to_string() == s)
    };
    Some(if valid(registered.trim()) && closed.is_some_and(valid) {
        crate::CompanyStatus::Closed
    } else {
        crate::CompanyStatus::Unknown
    })
}
