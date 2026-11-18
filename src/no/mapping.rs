use super::NoCompanyLookupResult;
use crate::{CompanyDetails, CompanyLookupResult};
impl From<NoCompanyLookupResult> for CompanyLookupResult {
    fn from(v: NoCompanyLookupResult) -> Self {
        let legal_form = v
            .details
            .organisasjonsform
            .as_ref()
            .and_then(|f| f.beskrivelse.clone());
        let vat_registered = v.details.registrert_i_mvaregisteret;
        let status = lifecycle(&v.details);
        let registry_details = CompanyDetails::NorwegianBrreg {
            native_registry_status: None,
            details: Box::new(v.details),
        };
        Self {
            country: "NO".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            legal_form,
            vat_registered,
            registered_office: None,
            fiscal_address: None,
            vat_id: None,
            status,
            registry_url: None,
            foreign_name: None,
            source: registry_details.provider(),
            registry_details,
        }
    }
}

fn lifecycle(details: &super::NoCompanyDetails) -> Option<crate::CompanyStatus> {
    use crate::CompanyStatus::*;
    if details
        .response_class
        .as_deref()
        .is_some_and(|s| !matches!(s, "Enhet" | "SlettetEnhet"))
    {
        return Some(Unknown);
    }
    let restructuring = details.under_rekonstruksjonsforhandling_dato.as_deref();
    if details.under_utenlandsk_insolvensbehandling_dato.is_some() {
        // Foreign insolvency does not establish the exact domestic proceeding.
        return Some(Unknown);
    }
    let bankrupt = details.konkurs == Some(true);
    let liquidating = details.under_avvikling == Some(true)
        || details.under_tvangsavvikling_eller_tvangsopplosning == Some(true);
    if let Some(date) = details.slettedato.as_deref() {
        return Some(
            if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .is_ok_and(|d| d.format("%Y-%m-%d").to_string() == date)
                && !bankrupt
                && !liquidating
                && restructuring.is_none()
            {
                Closed
            } else {
                Unknown
            },
        );
    }
    if details.response_class.as_deref() == Some("SlettetEnhet") {
        return Some(Unknown); // Deleted response without the required deletion date.
    }
    if let Some(date) = restructuring {
        return Some(if valid_date(date) && !bankrupt && !liquidating {
            Restructuring
        } else {
            Unknown
        });
    }
    match (bankrupt, liquidating) {
        (true, false) => Some(Bankruptcy),
        (false, true) => Some(Liquidation),
        (true, true) => Some(Unknown),
        (false, false)
            if details.response_class.as_deref() == Some("Enhet")
                && details.konkurs == Some(false)
                && details.under_avvikling == Some(false)
                && details.under_tvangsavvikling_eller_tvangsopplosning == Some(false)
                && details
                    .registreringsdato_enhetsregisteret
                    .as_deref()
                    .is_some_and(valid_date)
                && details.paategninger.as_ref().is_some_and(Vec::is_empty) =>
        {
            Some(Active)
        }
        (false, false) => None,
    }
}

fn valid_date(date: &str) -> bool {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .is_ok_and(|d| d.format("%Y-%m-%d").to_string() == date)
}
