use super::PlCompanyLookupResult;
use crate::{CompanyAddress, CompanyDetails, CompanyLookupResult};
impl From<PlCompanyLookupResult> for CompanyLookupResult {
    fn from(v: PlCompanyLookupResult) -> Self {
        let legal_form = v
            .details
            .dane_podmiotu
            .forma_prawna
            .clone()
            .filter(|s| !s.trim().is_empty());
        let registered_office = v
            .details
            .siedziba_i_adres
            .as_ref()
            .and_then(|a| a.adres.as_ref())
            .map(|a| {
                let delivery: Vec<_> = [&a.ulica, &a.nr_domu]
                    .into_iter()
                    .flatten()
                    .filter(|s| !s.trim().is_empty())
                    .map(String::as_str)
                    .collect();
                let mut fields = crate::locations::Address {
                    address: (!delivery.is_empty()).then(|| delivery.join(" ")),
                    address2: a.nr_lokalu.clone(),
                    city: a.miejscowosc.clone(),
                    postcode: a.kod_pocztowy.clone(),
                    country: a.kraj.clone(),
                    ..Default::default()
                };
                fields.normalize_country();
                CompanyAddress {
                    full_address_by_registry: None,
                    fields,
                }
            });
        let status = if v.details.foreign_branch {
            // Only an unrestricted current branch registration is resolved; proceedings
            // may concern its foreign parent and need separate interpretation.
            (lifecycle(&v.details).is_none() && current_registration(&v.details))
                .then_some(crate::CompanyStatus::Active)
        } else if let Some(removal) = &v.details.full_extract_removal {
            Some(
                // Consumer policy treats an explicit pending removal as closed,
                // while preserving the registry's non-finality marker below.
                if (removal.final_date.is_some() && removal.non_final.is_none())
                    || (removal.final_date.is_none()
                        && removal.non_final.as_deref() == Some("Wpis jest nieprawomocny"))
                {
                    crate::CompanyStatus::Closed
                } else {
                    crate::CompanyStatus::Unknown
                },
            )
        } else {
            lifecycle(&v.details).or_else(|| {
                current_registration(&v.details).then_some(crate::CompanyStatus::Active)
            })
        };
        let registry_details = CompanyDetails::PolishKrs {
            native_registry_status: v
                .details
                .full_extract_removal
                .as_ref()
                .and_then(|r| r.non_final.clone()),
            details: Box::new(v.details),
        };
        Self {
            country: "PL".into(),
            tax_id: v.tax_id.into(),
            company_name: v.company_name,
            legal_form,
            registered_office,
            fiscal_address: None,
            vat_id: None,
            vat_registered: None,
            status,
            registry_url: None,
            foreign_name: None,
            source: registry_details.provider(),
            registry_details,
        }
    }
}

fn lifecycle(details: &super::PlCompanyDetails) -> Option<crate::CompanyStatus> {
    use crate::CompanyStatus::*;
    let section = details.dzial6.as_ref()?;
    let as_of = details.naglowek.stan_z_dnia.as_deref();
    let mut states = Vec::new();
    for entry in section.likwidacja.iter().flatten() {
        if super::liquidation::continued(entry, details) {
            continue;
        }
        if super::liquidation::duplicated_opening(entry, details) {
            states.push(Liquidation);
            continue;
        }
        if entry
            .otwarcie_likwidacji
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
        {
            states.push(if entry.zakonczenie_likwidacji.is_none() {
                Liquidation
            } else {
                Unknown
            });
        } else {
            states.push(Unknown);
        }
    }
    for entry in section.postepowanie_upadlosciowe.iter().flatten() {
        let opened = entry
            .informacja_o_ogloszeniu_upadlosci
            .as_ref()
            .and_then(|o| o.data.as_deref())
            .is_some_and(valid_date);
        // A dated court order in the designated ending field ends this proceeding,
        // not the company. Active still needs independent current registration.
        let discontinued = entry
            .opis_zakonczenia_procesu_upadlosci
            .as_ref()
            .is_some_and(|end| {
                let parse = |s: Option<&str>| {
                    s.filter(|s| valid_date(s))
                        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%d.%m.%Y").ok())
                };
                end.opis_sposobu_zakonczenia_postepowania
                    .as_deref()
                    .is_some_and(|description| {
                        description == "UMORZENIE POSTĘPOWANIA UPADŁOŚCIOWEGO"
                            || (description.starts_with("POSTANOWIENIE SĄDU REJONOWEGO ")
                                && description.split_whitespace().any(|word| word == "GUP"))
                    })
                    && end.additional.is_empty()
                    && parse(
                        entry
                            .informacja_o_ogloszeniu_upadlosci
                            .as_ref()
                            .and_then(|o| o.data.as_deref()),
                    )
                    .zip(parse(end.data_zakonczenia_postepowania.as_deref()))
                    .zip(parse(as_of))
                    .is_some_and(|((start, end), snapshot)| start <= end && end <= snapshot)
            });
        if discontinued {
            continue;
        }
        states.push(
            if opened && unended(entry.opis_zakonczenia_procesu_upadlosci.as_ref()) {
                Bankruptcy
            } else {
                Unknown
            },
        );
    }
    for entry in section.postepowanie_restrukturyzacyjne_naprawcze_przymusowa_restrukturyzacja_uporzadkowana_likwidacja.iter().flatten() {
        let opened = entry.otwarcie_postepowania_restrukturyzacyjnego_naprawczego_przymusowej_restrukturyzacji_uporzadkowanej_likwidacji.as_ref()
            .is_some_and(|o| o.rodzaj_postepowania.as_deref() == Some("POSTĘPOWANIE RESTRUKTURYZACYJNE")
                && o.data_nadania_klauzuli_wykonalnosci.as_deref().is_some_and(valid_date));
        states.push(if opened && unended(entry.zakonczenie_postepowania_restrukturyzacyjnego_naprawczego_przymusowej_restrukturyzacji_uporzadkowanej_likwidacji.as_ref()) { Restructuring } else { Unknown });
    }
    if let Some(entries) = &section.zawieszenie_wznowienie {
        // Compare to the source snapshot, never the replay machine's current date.
        let snapshot = as_of
            .filter(|s| valid_date(s))
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%d.%m.%Y").ok());
        let mut intervals = Vec::new();
        let mut bad = snapshot.is_none();
        for entry in entries {
            let start = entry
                .data_zawieszenia
                .as_deref()
                .filter(|s| valid_date(s))
                .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%d.%m.%Y").ok());
            let end = entry
                .data_wznowienia
                .as_deref()
                .filter(|s| valid_date(s))
                .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%d.%m.%Y").ok());
            if start.is_none()
                || (entry.data_wznowienia.is_some() && end.is_none())
                || end.zip(start).is_some_and(|(e, s)| e < s)
            {
                bad = true;
            }
            if let Some(start) = start {
                intervals.push((start, end));
            }
        }
        intervals.sort();
        if intervals
            .windows(2)
            .any(|w| w[0].1.is_none_or(|end| end >= w[1].0))
        {
            bad = true;
        }
        if bad {
            states.push(Unknown);
        } else if intervals.iter().any(|(start, end)| {
            snapshot.is_some_and(|day| *start <= day && end.is_none_or(|end| day < end))
        }) {
            states.push(Inactive);
        }
        // An ended or future suspension alone does not establish active lifecycle.
    }
    match states.as_slice() {
        [] => None,
        [only] => Some(*only),
        _ => Some(Unknown),
    }
}
fn valid_date(date: &str) -> bool {
    chrono::NaiveDate::parse_from_str(date, "%d.%m.%Y")
        .is_ok_and(|d| d.format("%d.%m.%Y").to_string() == date)
}
fn unended(end: Option<&super::PlProceedingEnd>) -> bool {
    end.is_none_or(|e| {
        e.opis_sposobu_zakonczenia_postepowania.is_none()
            && e.data_zakonczenia_postepowania.is_none()
            && e.additional.is_empty()
    })
}

// A current extract contains current register entries (§7(2), DU 2025/201).
// This means registered without a reported lifecycle restriction, not trading or solvent.
fn current_registration(details: &super::PlCompanyDetails) -> bool {
    let h = &details.naglowek;
    let date = |s: Option<&str>| {
        s.filter(|s| valid_date(s))
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%d.%m.%Y").ok())
    };
    let dates = date(h.data_rejestracji_w_krs.as_deref())
        .zip(date(h.data_ostatniego_wpisu.as_deref()))
        .zip(date(h.stan_z_dnia.as_deref()));
    details.dzial6.is_some()
        && details.unmapped_lifecycle_fields.is_empty()
        && super::liquidation::historical_dissolution(details)
        && !(details.foreign_branch
            && details
                .dzial6
                .as_ref()
                .and_then(|s| s.polaczenie_podzial_przeksztalcenie.as_ref())
                .is_some_and(|e| !e.is_empty()))
        && matches!(
            (h.rejestr.as_str(), h.stan_pozycji),
            ("RejP", Some(1 | 4)) | ("RejS", Some(3 | 4))
        )
        && dates
            .is_some_and(|((registered, last), snapshot)| registered <= last && last <= snapshot)
        && !details.corporate_events.iter().any(|e| {
            e.role == super::PlReorganizationRole::Absorbed
                || (e.role == super::PlReorganizationRole::Unresolved
                    && !(h.rejestr == "RejS"
                        && e.source.okreslenie_okolicznosci.as_deref() == Some("POŁĄCZENIE")
                        && e.source
                            .opis_podzialu_polaczenia_przeksztalcenia
                            .as_deref()
                            .is_some_and(|s| !s.trim().is_empty())
                        && e.source.podmioty_powstale_przejmujace.as_ref().is_some_and(
                            |parties| {
                                !parties.is_empty() && parties.iter().all(|p| p.krs.is_none())
                            },
                        )))
        })
        && !details
            .dane_podmiotu
            .nazwa
            .to_uppercase()
            .contains("W LIKWIDACJI")
        && !details
            .dane_podmiotu
            .nazwa
            .to_uppercase()
            .contains("W UPADŁOŚCI")
}
