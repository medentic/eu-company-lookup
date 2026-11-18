use super::FiCompanyLookupResult;
use crate::{CompanyDetails, CompanyLookupResult};
impl From<FiCompanyLookupResult> for CompanyLookupResult {
    fn from(v: FiCompanyLookupResult) -> Self {
        let mut status = v.native_registry_status.as_deref().map(lifecycle);
        let mut ongoing = v.details.company_situations.iter().filter(|s| {
            s.end_date
                .as_deref()
                .is_none_or(|end| end.trim().is_empty())
        });
        if let Some(first) = ongoing.next() {
            let situation = situation_lifecycle(&first.r#type);
            status = Some(
                if status == Some(crate::CompanyStatus::Active)
                    && v.details
                        .end_date
                        .as_deref()
                        .is_none_or(|end| end.trim().is_empty())
                    && ongoing.all(|s| situation_lifecycle(&s.r#type) == situation)
                {
                    situation
                } else {
                    crate::CompanyStatus::Unknown
                },
            );
        } else if status == Some(crate::CompanyStatus::Active)
            && v.details
                .end_date
                .as_deref()
                .is_some_and(|end| !end.trim().is_empty())
        {
            status = Some(crate::CompanyStatus::Unknown);
        }
        // An invalid situation end date cannot establish that proceedings ended.
        if v.details.company_situations.iter().any(|s| {
            s.end_date
                .as_deref()
                .filter(|date| !date.trim().is_empty())
                .is_some_and(|date| {
                    !jiff::civil::Date::strptime("%Y-%m-%d", date)
                        .is_ok_and(|d| d.strftime("%Y-%m-%d").to_string() == date)
                })
        }) {
            status = Some(crate::CompanyStatus::Unknown);
        }
        let registry_details = CompanyDetails::FinnishPrh {
            native_registry_status: v.native_registry_status,
            details: Box::new(v.details),
        };
        Self {
            country: "FI".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            legal_form: v.legal_form,
            registered_office: None,
            fiscal_address: None,
            vat_registered: None,
            vat_id: None,
            registry_url: None,
            foreign_name: None,
            status,
            source: registry_details.provider(),
            registry_details,
        }
    }
}

// Exact provider states; insolvency/proceedings and future values remain distinguishable.
fn lifecycle(value: &str) -> crate::CompanyStatus {
    use crate::CompanyStatus::{Active, Closed, Unknown};
    match value.trim().to_uppercase().as_str() {
        "1" => Active,
        "4" => Closed,
        _ => Unknown,
    }
}

fn situation_lifecycle(code: &str) -> crate::CompanyStatus {
    match code {
        "SANE" => crate::CompanyStatus::Restructuring,
        "SELTILA" => crate::CompanyStatus::Liquidation,
        "KONK" => crate::CompanyStatus::Bankruptcy,
        _ => crate::CompanyStatus::Unknown,
    }
}
