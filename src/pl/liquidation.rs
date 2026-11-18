//! Narrow reconciliation of registered liquidation descriptions (KSH Arts. 273–276).
use super::{PlCompanyDetails, PlLiquidation};
use chrono::NaiveDate;

fn words(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
}
fn date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text, "%d.%m.%Y")
        .ok()
        .filter(|d| d.format("%d.%m.%Y").to_string() == text)
}
fn dates(text: &str) -> Vec<NaiveDate> {
    text.char_indices()
        .filter_map(|(i, _)| text.get(i..i + 10).and_then(date))
        .collect()
}
fn limited_company(details: &PlCompanyDetails) -> bool {
    !details.foreign_branch
        && details.dane_podmiotu.forma_prawna.as_deref()
            == Some("SPÓŁKA Z OGRANICZONĄ ODPOWIEDZIALNOŚCIĄ")
}

pub(super) fn continued(entry: &PlLiquidation, details: &PlCompanyDetails) -> bool {
    if cancelled(entry, details) {
        return true;
    }
    let Some(opening) = entry.otwarcie_likwidacji.as_deref() else {
        return false;
    };
    let Some(ending) = entry.zakonczenie_likwidacji.as_deref() else {
        return false;
    };
    let ending = words(ending);
    let Some((prefix, suffix)) = ending.rsplit_once("O DALSZYM ISTNIENIU SPÓŁKI.") else {
        return false;
    };
    // Recognize an affirmative registered resolution, not proposals, refusals or
    // a phrase buried in another event. The trailing part contains only dates.
    if !limited_company(details)
        || !prefix.starts_with("UCHWAŁA NR ")
        || prefix.split(|c: char| !c.is_alphabetic()).any(|w| {
            w == "NIE"
                || w.starts_with("UCHYLEN")
                || w.starts_with("ODMOW")
                || w.starts_with("PROJEKT")
                || w.starts_with("NIEWAŻN")
        })
        || !suffix
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_whitespace() || matches!(c, '.' | ','))
        || words(&details.dane_podmiotu.nazwa).contains("W LIKWIDACJI")
    {
        return false;
    }
    let ends = dates(&ending);
    let starts = dates(opening);
    let snapshot = details.naglowek.stan_z_dnia.as_deref().and_then(date);
    starts
        .first()
        .zip(ends.first())
        .zip(snapshot)
        .is_some_and(|((start, end), snapshot)| {
            start < end
                && *end <= snapshot
                && ends.iter().all(|d| d == end)
                && !dates(suffix).is_empty()
        })
}

// Some current extracts withdraw the opening when liquidation is cancelled.
// Require the affirmative resolution to cancel liquidation AND dismiss its officer.
fn cancelled(entry: &PlLiquidation, details: &PlCompanyDetails) -> bool {
    let Some(ending) = entry.zakonczenie_likwidacji.as_deref() else {
        return false;
    };
    let ending = words(ending);
    let Some((prefix, suffix)) = ending.split_once(
        " W SPRAWIE DALSZEGO ISTNIENIA SPÓŁKI, UCHYLENIA LIKWIDACJI I ODWOŁANIA LIKWIDATORA",
    ) else {
        return false;
    };
    let Some((_, number)) = prefix.rsplit_once("UCHWAŁA NR ") else {
        return false;
    };
    let ends = dates(&ending);
    limited_company(details)
        && entry.otwarcie_likwidacji.is_none()
        && entry.liquidator_count.is_none_or(|n| n == 0)
        && !words(&details.dane_podmiotu.nazwa).contains("W LIKWIDACJI")
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit())
        && !prefix.split(|c: char| !c.is_alphabetic()).any(|w| {
            w == "NIE"
                || w.starts_with("PROJEKT")
                || w.starts_with("ODMOW")
                || w.starts_with("NIEWAŻN")
                || w.starts_with("UCHYLEN")
        })
        && suffix
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_whitespace() || matches!(c, '.' | ','))
        && !dates(suffix).is_empty()
        && ends
            .first()
            .zip(details.naglowek.stan_z_dnia.as_deref().and_then(date))
            .is_some_and(|(end, snapshot)| *end <= snapshot && ends.iter().all(|d| d == end))
}

pub(super) fn historical_dissolution(details: &PlCompanyDetails) -> bool {
    let Some(section) = details.dzial6.as_ref() else {
        return false;
    };
    let Some(dissolution) = section.rozwiazanie_uniewaznienie.as_ref() else {
        return true;
    };
    let Some(description) = dissolution
        .informacja_o_rozwiazaniu_uniewaznieniu
        .as_deref()
    else {
        return false;
    };
    let entries = section.likwidacja.as_deref().unwrap_or_default();
    // A dissolution resolution can remain after liquidation is abandoned. Require
    // every period to end in continued existence and the dissolution text to repeat
    // one of those openings, allowing punctuation and an appended source date only.
    let compact = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_uppercase)
            .collect::<String>()
    };
    dissolution.okreslenie_okolicznosci.as_deref() == Some("ROZWIĄZANIE")
        && dissolution.additional.is_empty()
        && !entries.is_empty()
        && entries.iter().all(|e| continued(e, details))
        && entries.iter().any(|e| {
            e.otwarcie_likwidacji.as_deref().is_some_and(|opening| {
                let body = compact(description);
                let opening = compact(opening);
                body.strip_prefix(&opening)
                    .is_some_and(|tail| tail.chars().all(|c| c.is_ascii_digit()))
            })
        })
}

pub(super) fn duplicated_opening(entry: &PlLiquidation, details: &PlCompanyDetails) -> bool {
    let Some(opening) = entry.otwarcie_likwidacji.as_deref() else {
        return false;
    };
    let Some(ending) = entry.zakonczenie_likwidacji.as_deref() else {
        return false;
    };
    // The source duplicated an opening order into its ending field. Require the
    // same appointment text, a current liquidator and the statutory name suffix.
    limited_company(details)
        && words(opening) == words(ending)
        && words(opening).starts_with("SĄD REJESTROWY, POSTANOWIENIEM ")
        && words(opening).contains("ORZEKŁ O JEJ ROZWIĄZANIU I USTANOWIŁ LIKWIDATORA")
        && words(&details.dane_podmiotu.nazwa).ends_with(" W LIKWIDACJI")
        && entry.liquidator_count.is_some_and(|n| n > 0)
        && dates(opening)
            .last()
            .zip(details.naglowek.stan_z_dnia.as_deref().and_then(date))
            .is_some_and(|(opened, snapshot)| *opened <= snapshot)
}
