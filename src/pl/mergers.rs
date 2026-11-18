//! Direction comes from structured KRS roles, never a name or free-text mention.
use super::{PlCorporateEvent, PlMergerParty, PlReorganization, PlReorganizationRole as Role};

pub(super) fn events(entries: &[PlReorganization], own: &str) -> Vec<PlCorporateEvent> {
    entries
        .iter()
        .map(|entry| {
            let receivers = entry
                .podmioty_powstale_przejmujace
                .as_deref()
                .unwrap_or_default();
            let acquired = entry.podmioty_przejmowane.as_deref().unwrap_or_default();
            let receiver_ids = ids(receivers);
            let acquired_ids = ids(acquired);
            let role = match entry.okreslenie_okolicznosci.as_deref() {
                Some("PRZEJĘCIE INNEJ SPÓŁKI")
                    if !acquired_ids.iter().any(|id| id == own)
                        && receiver_ids.iter().all(|id| id == own) =>
                {
                    Role::Acquirer
                }
                Some("PRZEJĘCIE PRZEZ INNĄ SPÓŁKĘ")
                    if acquired.is_empty() && !receiver_ids.iter().any(|id| id == own) =>
                {
                    Role::Absorbed
                }
                Some("PRZEJĘCIE CZĘŚCI MAJĄTKU INNEJ SPÓŁKI W WYNIKU PODZIAŁU") => {
                    Role::AssetRecipient
                }
                Some(
                    "WYDZIELENIE CZĘŚCI MAJĄTKU SPÓŁKI W WYNIKU PODZIAŁU"
                    | "WYODRĘBNIENIE CZĘŚCI MAJĄTKU SPÓŁKI W WYNIKU PODZIAŁU",
                ) => Role::PartialDivision,
                Some("PRZEKSZTAŁCENIE W INNĄ SPÓŁKĘ HANDLOWĄ" | "PRZEKSZTAŁCENIE") => {
                    Role::Transformation
                }
                Some("POŁĄCZENIE")
                    if acquired_ids.iter().any(|id| id == own)
                        && !receiver_ids.is_empty()
                        && receiver_ids.iter().all(|id| id != own) =>
                {
                    Role::Absorbed
                }
                Some("POŁĄCZENIE")
                    if receiver_ids.iter().any(|id| id == own)
                        && !acquired_ids.iter().any(|id| id == own) =>
                {
                    Role::ResultingEntity
                }
                Some("POŁĄCZENIE")
                    if !acquired.is_empty()
                        && acquired
                            .iter()
                            .all(|p| p.nazwa.as_deref().is_some_and(|n| !n.trim().is_empty()))
                        && receivers.is_empty()
                        && !acquired_ids.iter().any(|id| id == own) =>
                {
                    Role::Acquirer
                }
                _ => Role::Unresolved,
            };
            // Partial asset transfers and transformations are not merger succession.
            let successor_krs = if role == Role::Absorbed {
                receiver_ids
            } else {
                Vec::new()
            };
            PlCorporateEvent {
                role,
                successor_krs,
                source: entry.clone(),
            }
        })
        .collect()
}

fn ids(parties: &[PlMergerParty]) -> Vec<String> {
    let mut ids = Vec::new();
    for party in parties {
        if let Some(id) = party.krs.as_ref().and_then(|k| k.krs.as_deref())
            && id.len() == 10
            && id.bytes().all(|c| c.is_ascii_digit())
            && id != "0000000000"
            && !ids.iter().any(|existing| existing == id)
        {
            ids.push(id.to_owned());
        }
    }
    ids
}
