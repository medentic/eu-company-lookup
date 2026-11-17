//! Source-only projection. DIČ and source registration flags do not establish VAT status.
use super::{CzAresDetails, CzCompanyLookupResult, CzRegistryAddress};
use crate::{CompanyAddress, CompanyDetails, CompanyLookupResult};

fn address(a: &CzRegistryAddress) -> CompanyAddress {
    let mut fields = crate::locations::Address::default();
    // House-number type 1 is descriptive (č.p.), type 2 evidential (č.ev.).
    // Unknown types remain in the native details instead of being guessed.
    let delivery_name = a.nazev_ulice.as_deref().or_else(|| {
        a.nazev_casti_obce
            .as_deref()
            .filter(|part| Some(*part) != a.nazev_obce.as_deref())
    });
    let number = a.cislo_do_adresy.clone().or_else(|| {
        let house = a.cislo_domovni.filter(|n| *n > 0)?;
        let prefix = match a.typ_cislo_domovni {
            Some(1) if delivery_name.is_some() => "",
            Some(1) => "č.p. ",
            Some(2) => "č.ev. ",
            _ => return None,
        };
        let mut value = format!("{prefix}{house}");
        if let Some(n) = a.cislo_orientacni.filter(|n| *n > 0) {
            value.push_str(&format!("/{n}"));
            if let Some(letter) = &a.cislo_orientacni_pismeno {
                value.push_str(letter);
            }
        }
        Some(value)
    });
    let delivery: Vec<&str> = [delivery_name, number.as_deref()]
        .into_iter()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .collect();
    fields.address = (!delivery.is_empty()).then(|| delivery.join(" "));
    fields.address2 = a.doplnek_adresy.clone();
    fields.city = a.nazev_obce.clone();
    fields.region = a.nazev_kraje.clone();
    fields.postcode = a.psc_txt.clone().or_else(|| a.psc.map(|n| n.to_string()));
    fields.country = a.nazev_statu.clone();
    fields.country_iso2 = a.kod_statu.clone();
    fields.normalize_country();
    CompanyAddress {
        full_address_by_registry: a.textova_adresa.clone(),
        fields,
    }
}
impl From<CzCompanyLookupResult> for CompanyLookupResult {
    fn from(v: CzCompanyLookupResult) -> Self {
        let registered_office = v.registered_office.as_ref().map(address);
        let details = CzAresDetails {
            registered_office: v.registered_office,
            legal_form_code: v.legal_form_code,
            ros_legal_form_code: v.ros_legal_form_code,
            tax_identifier: v.tax_identifier,
            vat_group_identifier: v.vat_group_identifier,
            created_date: v.created_date,
            ceased_date: v.ceased_date,
            updated_date: v.updated_date,
            primary_source: v.primary_source,
            source_registrations: v.source_registrations,
        };
        let registry_details = CompanyDetails::CzechAres {
            native_registry_status: None,
            details: Box::new(details),
        };
        Self {
            country: "CZ".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            vat_registered: None,
            vat_id: None,
            source: registry_details.provider(),
            registry_url: None,
            foreign_name: None,
            legal_form: None,
            registered_office,
            fiscal_address: None,
            status: None,
            registry_details,
        }
    }
}
