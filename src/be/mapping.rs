//! NBB observations retain their source; no VAT inference.
use super::BeCompanyLookupResult;
use crate::{CompanyAddress, CompanyDetails, CompanyLookupResult};

impl From<BeCompanyLookupResult> for CompanyLookupResult {
    fn from(v: BeCompanyLookupResult) -> Self {
        let a = &v.registered_office;
        let delivery: Vec<&str> = [a.street_name.as_deref(), a.street_number.as_deref()]
            .into_iter()
            .flatten()
            .filter(|s| !s.trim().is_empty())
            .collect();
        let mut fields = crate::locations::Address {
            address: (!delivery.is_empty()).then(|| delivery.join(" ")),
            address2: a.box_number.clone(),
            city: a.town.clone(),
            postcode: a.postal_code.clone(),
            country: a.country.clone(),
            country_iso2: a.country_code.clone(),
            ..Default::default()
        };
        fields.normalize_country();
        let registered_office = Some(CompanyAddress {
            full_address_by_registry: None,
            fields,
        });
        let status = lifecycle(
            v.details.legal_situation_code.as_deref(),
            v.native_registry_status.as_deref(),
        );
        let registry_details = CompanyDetails::BelgianNbb {
            native_registry_status: v
                .native_registry_status
                .or(v.details.legal_situation_code.clone()),
            registered_office: v.registered_office,
            details: Box::new(v.details),
        };
        Self {
            country: "BE".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            legal_form: v.legal_form,
            registered_office,
            vat_registered: None,
            vat_id: None,
            registry_url: None,
            foreign_name: None,
            fiscal_address: None,
            status,
            source: registry_details.provider(),
            registry_details,
        }
    }
}

fn lifecycle(code: Option<&str>, label: Option<&str>) -> Option<crate::CompanyStatus> {
    use crate::CompanyStatus::{Active, Bankruptcy, Closed, Liquidation, Restructuring, Unknown};
    // FPS Economy distinguishes completed liquidation from its opening.
    match code.map(str::trim) {
        Some("000") => Some(Active),
        Some("012" | "013") => Some(Liquidation),
        Some("048" | "049" | "050") => Some(Bankruptcy),
        Some("040" | "041" | "091") => Some(Restructuring),
        Some(
            "014" | "015" | "016" | "017" | "021" | "022" | "023" | "024" | "025" | "026" | "111",
        ) => Some(Closed),
        Some(_) => Some(Unknown),
        None => label.map(|s| match s.trim().to_ascii_lowercase().as_str() {
            "normal situation" => Active,
            _ => Unknown,
        }),
    }
}
