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
        let status = v
            .native_registry_status
            .as_deref()
            .map(lifecycle)
            .or_else(|| {
                v.details
                    .status_code
                    .as_ref()
                    .map(|_| crate::CompanyStatus::Unknown)
            });
        let registry_details = CompanyDetails::GreekGemi {
            native_registry_status: v.native_registry_status.or(v.details.status_code.clone()),
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
            status,
            source: registry_details.provider(),
            registry_details,
        }
    }
}

// Exact provider states; insolvency/proceedings and future values remain distinguishable.
fn lifecycle(value: &str) -> crate::CompanyStatus {
    use crate::CompanyStatus::{Active, Bankruptcy, Closed, Liquidation, Unknown};
    match value.trim().to_uppercase().as_str() {
        "ΕΝΕΡΓΗ" | "ΕΝΕΡΓΉ" => Active,
        "ΔΙΑΓΡΑΦΗ" | "ΔΙΑΓΡΑΦΉ" => Closed,
        "ΛΥΣΗ - ΕΚΚΑΘΑΡΙΣΗ"
        | "ΛΎΣΗ - ΕΚΚΑΘΆΡΙΣΗ"
        | "ΛΥΣΗ-ΕΚΚΑΘΑΡΙΣΗ"
        | "ΛΎΣΗ-ΕΚΚΑΘΆΡΙΣΗ"
        | "ΥΠΟ ΕΚΚΑΘΑΡΙΣΗ"
        | "ΥΠΌ ΕΚΚΑΘΆΡΙΣΗ"
        | "ΕΚΚΑΘΑΡΙΣΗ"
        | "ΕΚΚΑΘΆΡΙΣΗ" => Liquidation,
        "ΠΤΩΧΕΥΣΗ" | "ΠΤΏΧΕΥΣΗ" => Bankruptcy,
        _ => Unknown,
    }
}
