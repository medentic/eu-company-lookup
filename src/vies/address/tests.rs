use super::split;
#[test]
fn country_formats() {
    let cases = [
        ("AT", "Example 1\nAT-2755 Town", "AT-2755", "Town"),
        ("BE", "Example 1\n1000 Town", "1000", "Town"),
        ("CY", "Example 1\n2232 Town", "2232", "Town"),
        ("CZ", "Example 1\nDistrict\n252 02 Town", "252 02", "Town"),
        ("DK", "Example 1\n3460 Town\n", "3460", "Town"),
        ("EE", "Example 1   61602 Town", "61602", "Town"),
        ("EL", "Example 1   16231 - Town", "16231", "Town"),
        ("FI", "Example 1\n04261 Town", "04261", "Town"),
        ("FR", "Building\nExample 1\n78180 Town", "78180", "Town"),
        ("IT", "Example 1\n34170 Town GO\n", "34170", "Town"),
        ("LT", "Example 1, Town, LT-12345", "LT-12345", "Town"),
        ("LU", "Example 1\nL-8232 Town", "L-8232", "Town"),
        ("NL", "\nExample 1\n1234 AB Town\n", "1234 AB", "Town"),
        ("PL", "Example 1\n05-850 Town", "05-850", "Town"),
        (
            "PT",
            "Example 1\nLocality\n1234-567 Town",
            "1234-567",
            "Town",
        ),
        ("SE", "Example 1\n621 85 Town", "621 85", "Town"),
        ("SI", "Example 1, 3257 Town", "3257", "Town"),
        ("SK", "Example 1\n123 45 Town\nSlovensko", "123 45", "Town"),
        ("BG", "ул. ПРИМЕР 1 обл.РУСЕ, гр.РУСЕ 7000", "7000", "РУСЕ"),
    ];
    for (country, raw, postcode, city) in cases {
        let p = split(country, Some(raw.into()));
        assert_eq!(p.full_address_by_vies.as_deref(), Some(raw));
        assert_eq!(p.postcode.as_deref(), Some(postcode));
        assert_eq!(p.city.as_deref(), Some(city));
        assert_eq!(
            p.address.as_deref(),
            Some(match country {
                "BG" => "ул. ПРИМЕР 1",
                "FR" => "Building",
                _ => "Example 1",
            })
        );
        assert_eq!(
            p.address2.as_deref(),
            match country {
                "CZ" => Some("District"),
                "FR" => Some("Example 1"),
                "PT" => Some("Locality"),
                _ => None,
            }
        );
        assert_eq!(
            p.region.as_deref(),
            match country {
                "BG" => Some("РУСЕ"),
                "IT" => Some("GO"),
                _ => None,
            }
        );
        assert_eq!(
            p.country.as_deref(),
            if country == "SK" {
                Some("Slovensko")
            } else {
                None
            }
        );
    }
}

#[test]
fn multiple_delivery_lines_and_locality_are_preserved() {
    let raw = "Building\r\nStreet 1\r\nDistrict\r\n75001 PARIS\r\n";
    let p = split("FR", Some(raw.into()));
    assert_eq!(p.full_address_by_vies.as_deref(), Some(raw));
    assert_eq!(p.address.as_deref(), Some("Building"));
    assert_eq!(p.address2.as_deref(), Some("Street 1\nDistrict"));
    let p = split("PT", Some("Street 1\nLocality\n1234-567 Town".into()));
    assert_eq!(p.address2.as_deref(), Some("Locality"));
}
#[test]
fn missing_unsupported_and_ambiguous_data_never_invent_components() {
    for (prefix, raw) in [
        ("RO", "Street 1\n12345 City"),
        ("BG", "ambiguous"),
        ("BG", "---"),
        ("FR", "N/A"),
        ("BG", ""),
    ] {
        let p = split(prefix, Some(raw.into()));
        assert_eq!(p.full_address_by_vies.as_deref(), Some(raw));
        assert!(
            p.address.is_none()
                && p.address2.is_none()
                && p.city.is_none()
                && p.postcode.is_none()
                && p.region.is_none()
                && p.country.is_none()
        );
    }
    assert_eq!(split("BG", None), crate::VatAddress::default());
    let p = split(
        "BG",
        Some("ул. ПРИМЕР 1 бл.2 вх.3 ет.4 обл.РУСЕ, гр.РУСЕ".into()),
    );
    assert_eq!(p.address.as_deref(), Some("ул. ПРИМЕР 1 бл.2 вх.3 ет.4"));
    assert_eq!(p.city.as_deref(), Some("РУСЕ"));
    assert_eq!(p.region.as_deref(), Some("РУСЕ"));
    assert!(p.postcode.is_none() && p.country.is_none());
    let p = split("SE", Some("\n621 85 VISBY".into()));
    assert!(p.address.is_none());
    assert_eq!(p.city.as_deref(), Some("VISBY"));
}
#[test]
fn oversized_input_and_foreign_addresses_are_not_guessed() {
    let raw = "x".repeat(65537);
    let p = split("BG", Some(raw.clone()));
    assert_eq!(p.full_address_by_vies, Some(raw));
    assert!(p.address.is_none());
    let p = split("BG", Some("Street 1\nDE-12345 Berlin".into()));
    assert!(p.country.is_none() && p.city.is_none());
}

#[test]
fn bulgarian_settlement_variants_preserve_building_details() {
    for raw in [
        "пл. ПРИМЕР № 1 обл.ГАБРОВО ГР.СЕВЛИЕВО 5400",
        "пл. ПРИМЕР № 1 ОБЛ.ГАБРОВО, ГР.СЕВЛИЕВО 5400",
        "пл. ПРИМЕР № 1 обл.ГАБРОВО гр.СЕВЛИЕВО 5400",
    ] {
        let p = split("BG", Some(raw.into()));
        assert_eq!(p.full_address_by_vies.as_deref(), Some(raw));
        assert_eq!(p.address.as_deref(), Some("пл. ПРИМЕР № 1"));
        assert_eq!(p.city.as_deref(), Some("СЕВЛИЕВО"));
        assert_eq!(p.postcode.as_deref(), Some("5400"));
        assert_eq!(p.region.as_deref(), Some("ГАБРОВО"));
        assert!(p.country.is_none() && p.address2.is_none());
    }
    let p = split("BG", Some("ул. ПРИМЕР 1 обл.СОФИЯ С.ПРИМЕРНО СЕЛО".into()));
    assert_eq!(p.city.as_deref(), Some("ПРИМЕРНО СЕЛО"));
    assert!(p.postcode.is_none());
}

#[test]
fn capabilities_do_not_claim_address_rules_for_unsupported_countries() {
    let catalog = crate::Client::new().unwrap().capabilities();
    assert_eq!(
        catalog
            .iter()
            .filter(|c| c.providers[0].address_splitting)
            .count(),
        19
    );
    for prefix in ["DE", "ES", "IE", "RO", "MT", "LV", "HR", "HU", "XI"] {
        let p = split(prefix, Some("Example 1\n12345 City".into()));
        assert!(p.address.is_none() && p.city.is_none() && p.country.is_none());
        assert!(
            !catalog
                .iter()
                .find(|c| c.vat_prefix == prefix)
                .unwrap()
                .providers[0]
                .address_splitting
        );
    }
}
