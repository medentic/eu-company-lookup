use super::FrCompanyLookupResult;
use crate::{CompanyAddress, CompanyDetails, CompanyLookupResult};
impl From<FrCompanyLookupResult> for CompanyLookupResult {
    fn from(v: FrCompanyLookupResult) -> Self {
        let registered_office = v.details.siege.as_ref().map(|s| {
            let delivery: Vec<_> = [
                &s.numero_voie,
                &s.indice_repetition,
                &s.type_voie,
                &s.libelle_voie,
            ]
            .into_iter()
            .flatten()
            .filter(|v| !v.trim().is_empty())
            .map(String::as_str)
            .collect();
            let secondary: Vec<_> = [&s.complement_adresse, &s.distribution_speciale]
                .into_iter()
                .flatten()
                .filter(|v| !v.trim().is_empty())
                .map(String::as_str)
                .collect();
            let mut fields = crate::locations::Address {
                address: (!delivery.is_empty()).then(|| delivery.join(" ")),
                address2: (!secondary.is_empty()).then(|| secondary.join("\n")),
                city: if s.code_pays_etranger.is_some() {
                    s.libelle_commune_etranger.clone()
                } else {
                    s.libelle_commune.clone()
                },
                postcode: s.code_postal.clone(),
                country: s.libelle_pays_etranger.clone(),
                ..Default::default()
            };
            fields.normalize_country();
            CompanyAddress {
                full_address_by_registry: s.adresse.clone(),
                fields,
            }
        });
        let native_registry_status = v.details.etat_administratif.clone();
        let status = native_registry_status.as_deref().map(lifecycle);
        let legal_form = v
            .details
            .nature_juridique
            .as_deref()
            .and_then(super::legal_forms::label)
            .map(str::to_owned);
        let registry_details = CompanyDetails::FrenchRechercheEntreprises {
            native_registry_status,
            details: Box::new(v.details),
        };
        Self {
            country: "FR".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            legal_form,
            registered_office,
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
        "A" => Active,
        "C" => Closed,
        _ => Unknown,
    }
}
