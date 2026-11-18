use super::EeCompanyLookupResult;
use crate::{CompanyAddress, CompanyDetails, CompanyLookupResult};
impl From<EeCompanyLookupResult> for CompanyLookupResult {
    fn from(v: EeCompanyLookupResult) -> Self {
        let registered_office = (v.details.legal_address.is_some() || v.details.zip_code.is_some())
            .then(|| CompanyAddress {
                full_address_by_registry: v.details.legal_address.clone(),
                fields: crate::locations::Address {
                    postcode: v.details.zip_code.clone(),
                    ..Default::default()
                },
            });
        let native_registry_status = v.details.status.clone();
        let status = native_registry_status.as_deref().map(lifecycle);
        let legal_form = match v.details.legal_form.as_deref() {
            Some("3") => Some("Füüsilisest isikust ettevõtja".to_owned()),
            Some("5") => Some("Osaühing".to_owned()),
            _ => None,
        };
        let registry_url = v
            .details
            .url
            .as_deref()
            .filter(|raw| {
                reqwest::Url::parse(raw).ok().is_some_and(|url| {
                    url.scheme() == "https"
                        && url.host_str() == Some("ariregister.rik.ee")
                        && url.username().is_empty()
                        && url.password().is_none()
                        && url.port().is_none()
                        && url.query().is_none()
                        && url.fragment().is_none()
                        && url.path_segments().is_some_and(|mut parts| {
                            matches!(parts.next(), Some("eng" | "est"))
                                && parts.next() == Some("company")
                                && parts.next() == Some(v.tax_id.as_str())
                        })
                })
            })
            .map(str::to_owned);
        let registry_details = CompanyDetails::EstonianRik {
            native_registry_status,
            details: Box::new(v.details),
        };
        Self {
            country: "EE".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            registered_office,
            status,
            legal_form,
            fiscal_address: None,
            vat_registered: None,
            vat_id: None,
            registry_url,
            foreign_name: None,
            source: registry_details.provider(),
            registry_details,
        }
    }
}

// Exact provider states; insolvency/proceedings and future values remain distinguishable.
fn lifecycle(value: &str) -> crate::CompanyStatus {
    use crate::CompanyStatus::{Active, Bankruptcy, Closed, Liquidation, Unknown};
    match value.trim().to_uppercase().as_str() {
        "R" => Active,
        "K" => Closed,
        "L" => Liquidation,
        "N" => Bankruptcy,
        _ => Unknown,
    }
}
