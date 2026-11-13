//! Conservative Romanian VIES locality-first addresses. Never normalizes source envelopes.
use super::Address;
use std::sync::LazyLock;

pub(super) fn parse(raw: &str) -> Address {
    static FORMAT: LazyLock<regex_lite::Regex> = LazyLock::new(|| {
        regex_lite::Regex::new(r"^(?:MUN\.|MUNICIPIUL|ORŞ\.|ORAŞ|ORS\.|SAT\.|LOC\.|COM\.)\s+(?P<city>.+?)(?:\s+-\s+(?:COM\.|ORŞ\.|MUN\.)\s+(?P<municipality>.+?))?\s+(?P<postcode>[0-9]{4,6})(?:\s+(?P<delivery>.*))?$").unwrap()
    });
    if raw.len() > 65_536 {
        return Address::default();
    }
    let Some(captures) = FORMAT.captures(raw.trim()) else {
        return Address::default();
    };
    let mut result = Address::default();
    let city = captures.name("city").unwrap().as_str().trim();
    // Unrecognized locality labels or numeric data inside a locality remain unknown.
    let bucharest_sector = [
        "BUCUREŞTI, SECTOR ",
        "BUCUREȘTI, SECTOR ",
        "BUCURESTI, SECTOR ",
    ]
    .iter()
    .any(|prefix| {
        city.strip_prefix(prefix)
            .is_some_and(|s| matches!(s, "1" | "2" | "3" | "4" | "5" | "6"))
    });
    if city.is_empty() || (city.chars().any(|c| c.is_ascii_digit()) && !bucharest_sector) {
        return result;
    }
    result.city = Some(city.to_owned());
    result.municipality = captures
        .name("municipality")
        .map(|m| m.as_str().trim().to_owned());
    if raw.trim_start().starts_with("COM.") {
        result.municipality = result.city.take();
    }
    let postcode = captures.name("postcode").unwrap().as_str();
    if postcode.bytes().any(|b| b != b'0') {
        result.postcode = Some(postcode.to_owned());
    }
    if let Some(delivery) = captures
        .name("delivery")
        .map(|m| m.as_str().trim())
        .filter(|s| !s.is_empty())
    {
        static DETAILS: LazyLock<regex_lite::Regex> = LazyLock::new(|| {
            regex_lite::Regex::new(r"(?:^|\s)(?:Bl\.|Sc\.|Et\.|Ap\.)\s*").unwrap()
        });
        if let Some(marker) = DETAILS.find(delivery) {
            let primary = delivery[..marker.start()].trim();
            result.address = (!primary.is_empty()).then(|| primary.to_owned());
            result.address2 = Some(delivery[marker.start()..].trim().to_owned());
        } else {
            result.address = Some(delivery.to_owned());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn marked_locality_postcode_and_delivery_are_separate() {
        let a = parse("MUN. EXEMPLU 012345 STR. TEST Nr. 2");
        assert_eq!(a.city.as_deref(), Some("EXEMPLU"));
        assert_eq!(a.postcode.as_deref(), Some("012345"));
        assert_eq!(a.address.as_deref(), Some("STR. TEST Nr. 2"));
        assert!(a.country.is_none());
        let a = parse("SAT. EXEMPLU - COM. COMUNA EXEMPLU 00000   Nr. 59");
        assert_eq!(a.city.as_deref(), Some("EXEMPLU"));
        assert_eq!(a.municipality.as_deref(), Some("COMUNA EXEMPLU"));
        assert_eq!(a.address.as_deref(), Some("Nr. 59"));
        assert!(a.postcode.is_none());
    }
    #[test]
    fn explicit_building_details_stay_separate_from_street_number() {
        let a = parse("MUN. EXEMPLU 012345 STR. TEST Nr. 3 Bl. D9 Sc. 2A Et. 3 Ap. 14");
        assert_eq!(a.address.as_deref(), Some("STR. TEST Nr. 3"));
        assert_eq!(a.address2.as_deref(), Some("Bl. D9 Sc. 2A Et. 3 Ap. 14"));
        assert_eq!(parse("MUN. EXEMPLU 012345 Bl. TEST").address, None);
    }
    #[test]
    fn unfamiliar_and_incomplete_formats_are_not_guessed() {
        for raw in [
            "",
            "---",
            "EXEMPLU 012345 STR. TEST",
            "MUN. EXEMPLU",
            "MUN. EXEMPLU 1234567 STR. TEST",
        ] {
            assert_eq!(parse(raw), Address::default());
        }
        assert!(parse("MUN. EXEMPLU 000000 STR. TEST").postcode.is_none());
    }
}

#[cfg(test)]
#[test]
fn bucharest_sectors_and_short_source_postcodes_are_preserved_without_padding() {
    let a = parse("MUNICIPIUL  BUCUREŞTI, SECTOR 1 14134 STR. TEST Nr. 2 Et. 4");
    assert_eq!(a.city.as_deref(), Some("BUCUREŞTI, SECTOR 1"));
    assert_eq!(a.postcode.as_deref(), Some("14134"));
    assert_eq!(a.address2.as_deref(), Some("Et. 4"));
    assert_eq!(
        parse("SAT. EXEMPLU - COM. EXEMPLU 87015 STR. TEST")
            .postcode
            .as_deref(),
        Some("87015")
    );
}

#[cfg(test)]
#[test]
fn village_parent_town_and_legacy_source_postcode_are_not_invented() {
    let a = parse("SAT. EXEMPLU - ORŞ. ORAŞ EXEMPLU 75100 CAL. TEST Nr. 224 E");
    assert_eq!(a.city.as_deref(), Some("EXEMPLU"));
    assert_eq!(a.municipality.as_deref(), Some("ORAŞ EXEMPLU"));
    assert_eq!(
        parse("MUN. EXEMPLU 2000 STR. TEST Nr. 17-19")
            .postcode
            .as_deref(),
        Some("2000")
    );
}

#[cfg(test)]
#[test]
fn explicitly_labelled_locality_with_parent_municipality_is_supported() {
    let a = parse("LOC. EXEMPLU - MUN. ORAŞ EXEMPLU 420002 STR. TEST Nr. 131/A");
    assert_eq!(a.city.as_deref(), Some("EXEMPLU"));
    assert_eq!(a.municipality.as_deref(), Some("ORAŞ EXEMPLU"));
}

#[cfg(test)]
#[test]
fn commune_only_source_does_not_invent_a_settlement() {
    let a = parse("COM. EXEMPLU 207205 TARLAUA 65, PARCELA 650");
    assert_eq!(a.city, None);
    assert_eq!(a.municipality.as_deref(), Some("EXEMPLU"));
    assert_eq!(a.postcode.as_deref(), Some("207205"));
    assert_eq!(a.address.as_deref(), Some("TARLAUA 65, PARCELA 650"));
}
