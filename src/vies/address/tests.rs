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
        ("BG", "ул. ПРИМЕР 1 обл.РУСЕ, гр.РУСЕ 7000", "7000", "Русе"),
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
                "BG" => Some("Русе"),
                "IT" => Some("GO"),
                _ => None,
            }
        );
        assert_eq!(
            p.country.as_deref(),
            if country == "SK" {
                Some("Slovensko")
            } else {
                super::country(country).map(|(_, name)| name)
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
                && p.country.is_some()
        );
    }
    assert_eq!(split("BG", None).country_iso2.as_deref(), Some("BG"));
    let p = split(
        "BG",
        Some("ул. ПРИМЕР 1 бл.2 вх.3 ет.4 обл.РУСЕ, гр.РУСЕ".into()),
    );
    assert_eq!(p.address.as_deref(), Some("ул. ПРИМЕР 1"));
    assert_eq!(p.address2.as_deref(), Some("бл. 2 вх. 3 ет. 4"));
    assert_eq!(p.city.as_deref(), Some("Русе"));
    assert_eq!(p.region.as_deref(), Some("Русе"));
    assert!(p.postcode.is_none() && p.country.is_some());
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
    assert!(p.country.is_some() && p.city.is_none());
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
        assert_eq!(p.city.as_deref(), Some("Севлиево"));
        assert_eq!(p.postcode.as_deref(), Some("5400"));
        assert_eq!(p.region.as_deref(), Some("Габрово"));
        assert!(p.country.is_some() && p.address2.is_none());
    }
    let p = split("BG", Some("ул. ПРИМЕР 1 обл.СОФИЯ С.ПРИМЕРНО СЕЛО".into()));
    assert_eq!(p.city.as_deref(), Some("Примерно Село"));
    assert!(p.postcode.is_none());
}

#[test]
fn capabilities_do_not_claim_address_rules_for_unsupported_countries() {
    let catalog = crate::Client::new().unwrap().capabilities();
    assert_eq!(
        catalog
            .iter()
            .filter(|c| c
                .providers
                .iter()
                .any(|p| p.provider == crate::Provider::Vies && p.address_splitting))
            .count(),
        20
    );
    for prefix in ["DE", "ES", "IE", "MT", "LV", "HR", "HU", "XI"] {
        let p = split(prefix, Some("Example 1\n12345 City".into()));
        assert!(p.address.is_none() && p.city.is_none() && p.country.is_some());
        assert!(
            !catalog
                .iter()
                .find(|c| c.vat_prefix == Some(prefix))
                .unwrap()
                .providers[0]
                .address_splitting
        );
    }
}

#[test]
fn normalize_then_extract_bulgarian_delivery_details() {
    let cases = [
        (
            "КНЯЖЕСКА  №13 бл.ЮТА вх.2 ет.2 обл.РУСЕ, гр.РУСЕ 7000",
            "КНЯЖЕСКА №13",
            "бл. ЮТА вх. 2 ет. 2",
            "Русе",
            "7000",
        ),
        (
            " жк Младост 4ул. Околовръстен път  №257 ет.3 обл.СОФИЯ, гр.СОФИЯ 1715",
            "жк. Младост 4, ул. Околовръстен път №257",
            "ет. 3",
            "София",
            "1715",
        ),
        (
            "  ул.  ПРИМЕР  №13бл.ЮТАвх.2ет.2ап.5 обл.РУСЕ, гр.РУСЕ 7000  ",
            "ул. ПРИМЕР №13",
            "бл. ЮТА вх. 2 ет. 2 ап. 5",
            "Русе",
            "7000",
        ),
    ];
    for (raw, address, address2, city, postcode) in cases {
        let p = split("BG", Some(raw.into()));
        assert_eq!(p.full_address_by_vies.as_deref(), Some(raw));
        assert_eq!(p.address.as_deref(), Some(address));
        assert_eq!(p.address2.as_deref(), Some(address2));
        assert_eq!(p.city.as_deref(), Some(city));
        assert_eq!(p.postcode.as_deref(), Some(postcode));
        assert_eq!(p.region.as_deref(), Some(city));
        assert!(p.country.is_some());
        let once = super::normalize::address("BG", raw);
        assert_eq!(super::normalize::address("BG", &once), once);
    }
}

#[test]
fn normalization_preserves_structural_lines_and_cleans_extracted_fields() {
    let raw = "  Building   A\r\n  Street\t  1\r\n75001   PARIS   CENTRE  \r\n";
    let p = split("FR", Some(raw.into()));
    assert_eq!(p.full_address_by_vies.as_deref(), Some(raw));
    assert_eq!(p.address.as_deref(), Some("Building A"));
    assert_eq!(p.address2.as_deref(), Some("Street 1"));
    assert_eq!(p.city.as_deref(), Some("PARIS CENTRE"));
    for prefix in ["EE", "EL"] {
        let raw = if prefix == "EE" {
            "Street  1   12345 City"
        } else {
            "Street  1   12345 - City"
        };
        let p = split(prefix, Some(raw.into()));
        assert_eq!(p.address.as_deref(), Some("Street 1"));
        assert_eq!(p.postcode.as_deref(), Some("12345"));
    }
}

#[test]
fn marker_cleanup_does_not_split_words_or_duplicate_separators() {
    let raw = "жк Пример 4, ул. Истанбул. №1 ЕТ.3 обл.СОФИЯ, гр.СОФИЯ 1000";
    let p = split("BG", Some(raw.into()));
    assert_eq!(p.address.as_deref(), Some("жк. Пример 4, ул. Истанбул. №1"));
    assert_eq!(p.address2.as_deref(), Some("ЕТ. 3"));
    let p = split("BG", Some("бл.1 вх.А ет.2 обл.РУСЕ, гр.РУСЕ 7000".into()));
    assert!(p.address.is_none());
    assert_eq!(p.address2.as_deref(), Some("бл. 1 вх. А ет. 2"));
}

#[test]
fn bulgarian_place_casing_is_conservative_and_idempotent() {
    for (input, expected) in [
        ("РУСЕ", "Русе"),
        ("СОФИЯ", "София"),
        ("ЛОМ", "Лом"),
        ("ВЕЛИКО ТЪРНОВО", "Велико Търново"),
        ("СОФИЯ-ГРАД", "София-Град"),
        ("Г. ОРЯХОВИЦА", "Г. Оряховица"),
        ("ЖК ПРИМЕР II", "ЖК Пример II"),
        ("София-ГРАД", "София-ГРАД"),
        ("русе", "русе"),
        ("123", "123"),
    ] {
        let raw = format!("ул. ПРИМЕР 1 обл.{input}, гр.{input} 1000");
        let result = split("BG", Some(raw.clone()));
        assert_eq!(result.city.as_deref(), Some(expected));
        assert_eq!(result.region.as_deref(), Some(expected));
        assert_eq!(result.full_address_by_vies.as_deref(), Some(raw.as_str()));
        assert_eq!(result.address.as_deref(), Some("ул. ПРИМЕР 1"));
        assert_eq!(
            super::normalize::place_name("BG", expected.into()),
            expected
        );
    }
    assert_eq!(super::normalize::place_name("IT", "GO".into()), "GO");
    assert_eq!(super::normalize::place_name("FR", "PARIS".into()), "PARIS");
}

#[test]
fn bulgarian_detail_spacing_is_canonical_without_inventing_a_street_marker() {
    for details in [
        "бл.ЮТА вх.2 ет.2 ап.5",
        "бл.  ЮТА вх.  2 ет.  2 ап.  5",
        "бл .ЮТА вх .2 ет .2 ап .5",
        "бл.\tЮТА вх.\u{a0}2 ет.2 ап.5",
    ] {
        let raw = format!("  КНЯЖЕСКА  №13 {details} обл.РУСЕ, гр.РУСЕ 7000  ");
        let result = split("BG", Some(raw.clone()));
        assert_eq!(result.full_address_by_vies.as_deref(), Some(raw.as_str()));
        assert_eq!(result.address.as_deref(), Some("КНЯЖЕСКА №13"));
        assert_eq!(
            result.address2.as_deref(),
            Some("бл. ЮТА вх. 2 ет. 2 ап. 5")
        );
        let normalized = super::normalize::address("BG", &raw);
        assert_eq!(super::normalize::address("BG", &normalized), normalized);
    }
}

#[test]
fn residential_complex_marker_has_one_period() {
    for marker in ["жк", "жк.", "жк .", "ЖК", "ЖК."] {
        let raw = format!("{marker} Младост 4ул. Пример №1 обл.СОФИЯ, гр.СОФИЯ 1000");
        let result = split("BG", Some(raw.clone()));
        let expected = "жк.";
        assert_eq!(
            result.address,
            Some(format!("{expected} Младост 4, ул. Пример №1"))
        );
        assert_eq!(result.full_address_by_vies.as_deref(), Some(raw.as_str()));
        let normalized = super::normalize::address("BG", &raw);
        assert_eq!(super::normalize::address("BG", &normalized), normalized);
    }
    assert_eq!(super::normalize::address("BG", "Дружка 1"), "Дружка 1");
}

#[test]
fn bulgarian_placeholder_details_and_repeated_periods() {
    for (input, expected) in [
        ("бл..11 вх.. ет...2 ап.-", Some("бл. 11 ет. 2")),
        ("бл.- вх.- ет.- ап.-", None),
        ("вх..А-Б ет..1-3", Some("вх. А-Б ет. 1-3")),
        ("вх. -, ет. 2, ап. -", Some("ет. 2")),
    ] {
        let raw = format!("ул.. Пример №7 {input} обл.ПЛЕВЕН, с.ОБНОВА -");
        let result = split("BG", Some(raw.clone()));
        assert_eq!(result.address.as_deref(), Some("ул. Пример №7"));
        assert_eq!(result.address2.as_deref(), expected);
        assert_eq!(result.city.as_deref(), Some("Обнова"));
        assert_eq!(result.full_address_by_vies.as_deref(), Some(raw.as_str()));
        let normalized = super::normalize::address("BG", &raw);
        assert_eq!(super::normalize::address("BG", &normalized), normalized);
    }
    assert_eq!(
        super::normalize::place_name("BG", "Обнова -".into()),
        "Обнова"
    );
    assert_eq!(
        super::normalize::place_name("BG", "София-Град".into()),
        "София-Град"
    );
    assert_eq!(
        super::normalize::place_name("FR", "Paris -".into()),
        "Paris -"
    );
}

#[test]
fn bulgarian_region_or_settlement_without_delivery_line() {
    for (raw, city, region, postcode) in [
        (
            "     обл.ГАБРОВО, с.РАХОВЦИ",
            "Раховци",
            Some("Габрово"),
            None,
        ),
        (
            "обл.СОФИЯ, гр.СОФИЯ 1000",
            "София",
            Some("София"),
            Some("1000"),
        ),
    ] {
        let p = split("BG", Some(raw.into()));
        assert_eq!(p.address, None);
        assert_eq!(p.address2, None);
        assert_eq!(p.city.as_deref(), Some(city));
        assert_eq!(p.region.as_deref(), region);
        assert_eq!(p.postcode.as_deref(), postcode);
        assert_eq!(p.full_address_by_vies.as_deref(), Some(raw));
    }
}

#[test]
fn bulgarian_comma_spacing_and_standalone_place_placeholders() {
    let raw = "с. Априлово , София обл.СОФИЯ, с.АПРИЛОВО 2128";
    let p = split("BG", Some(raw.into()));
    assert_eq!(p.address.as_deref(), Some("с. Априлово, София"));
    assert_eq!(p.full_address_by_vies.as_deref(), Some(raw));
    for tail in [" .", " ...", " -", " . -", " --"] {
        let raw = format!("жк УПИ V-125 обл.ВАРНА, с.КАЛИМАНЦИ{tail}");
        let p = split("BG", Some(raw.clone()));
        assert_eq!(p.city.as_deref(), Some("Калиманци"));
        assert_eq!(p.postcode, None);
        assert_eq!(p.full_address_by_vies, Some(raw));
    }
    for value in ["Г. О.", "София-Град", "с. Къща", "Калиманци."] {
        assert_eq!(super::normalize::place_name("BG", value.into()), value);
    }
    assert_eq!(
        super::normalize::place_name("FR", "Paris .".into()),
        "Paris ."
    );
    assert_eq!(
        super::normalize::address("BG", "ул. Пример 1,5 , офис 2"),
        "ул. Пример 1,5, офис 2"
    );
}

#[test]
fn bulgarian_quarter_marker_has_spacing_without_splitting_words() {
    let raw = "кв.\"Тева\"  №44 ет.3 ап.11 обл.ПЕРНИК, гр.ПЕРНИК 2300";
    let p = split("BG", Some(raw.into()));
    assert_eq!(p.address.as_deref(), Some("кв. Тева №44"));
    assert_eq!(p.address2.as_deref(), Some("ет. 3 ап. 11"));
    assert_eq!(p.full_address_by_vies.as_deref(), Some(raw));
    assert_eq!(
        super::normalize::address("BG", "букв. текст"),
        "букв. текст"
    );
}

#[test]
fn bulgarian_explicit_building_and_room_details_require_component_boundary() {
    let raw = "пл. \"СВОБОДА\", сграда на НТС, стая 407 обл.ЛОВЕЧ, гр.ЛОВЕЧ 5500";
    let parsed = split("BG", Some(raw.into()));
    assert_eq!(parsed.address.as_deref(), Some("пл. СВОБОДА"));
    assert_eq!(parsed.address2.as_deref(), Some("сграда на НТС, стая 407"));
    assert_eq!(parsed.full_address_by_vies.as_deref(), Some(raw));
    assert_eq!(
        crate::locations::split_delivery("ул. Сграда 1", "bg"),
        (Some("ул. Сграда 1".into()), None)
    );
    assert_eq!(
        crate::locations::split_delivery("ул. Нова №2, стая 4", "bg"),
        (Some("ул. Нова №2".into()), Some("стая 4".into()))
    );
    assert_eq!(
        crate::locations::split_delivery("сграда 3", "bg"),
        (None, Some("сграда 3".into()))
    );
    assert_eq!(
        crate::locations::split_delivery("ул. Нова №2, сграда", "bg"),
        (Some("ул. Нова №2, сграда".into()), None)
    );
}

#[test]
fn bulgarian_complex_glued_to_lowercase_street_and_room() {
    let raw = " жк ЛЕВСКИул.ПОДВИС  №29, СТАЯ №203 обл.ВАРНА, гр.ВАРНА 9010";
    let parsed = split("BG", Some(raw.into()));
    assert_eq!(
        parsed.address.as_deref(),
        Some("жк. ЛЕВСКИ, ул. ПОДВИС №29")
    );
    assert_eq!(parsed.address2.as_deref(), Some("СТАЯ №203"));
    assert_eq!(parsed.full_address_by_vies.as_deref(), Some(raw));
    assert_eq!(
        super::normalize::address("BG", "жк. букваул. текст"),
        "жк. букваул. текст"
    );
    assert_eq!(
        super::normalize::address("BG", "ул. ДЖУЛ. 1"),
        "ул. ДЖУЛ. 1"
    );
}

#[test]
fn bulgarian_reviewed_presentation_policy_preserves_original() {
    let raw = " ЖК „Сторгозия\" бл.90 вх.а ет.2 ап.5 обл.ПЛЕВЕН, гр.ПЛЕВЕН 5800";
    let p = split("BG", Some(raw.into()));
    assert_eq!(p.full_address_by_vies.as_deref(), Some(raw));
    assert_eq!(p.address.as_deref(), Some("жк. Сторгозия"));
    assert_eq!(p.address2.as_deref(), Some("бл. 90 вх. А ет. 2 ап. 5"));
    for text in ["ЖК ДЪБНИКА", "ЖК. ДЪБНИКА", "ж.к. ДЪБНИКА"] {
        assert_eq!(super::normalize::address("BG", text), "жк. ДЪБНИКА");
    }
    let value = "ул. «О'Конър», вх. b, ет. 2";
    let expected = "ул. О'Конър, вх. B, ет. 2";
    assert_eq!(super::normalize::address("BG", value), expected);
    assert_eq!(super::normalize::address("BG", expected), expected);
    assert_eq!(
        super::normalize::address("BG", "вх. северен"),
        "вх. северен"
    );
    assert_eq!(super::normalize::address("BG", "вх. 2"), "вх. 2");
    assert_eq!(
        super::normalize::address("FR", "Rue \"Exemple\""),
        "Rue \"Exemple\""
    );
}

#[test]
fn canonical_complex_preserves_compound_registry_street_marker() {
    assert_eq!(
        super::normalize::address("BG", "ж.к. КК СЛЪНЧЕВ БРЯГ-ЗАПАД, бул./ул. ВИВА КЛИНИК"),
        "жк. КК СЛЪНЧЕВ БРЯГ-ЗАПАД, бул./ул. ВИВА КЛИНИК"
    );
}

#[test]
fn bulgarian_settlement_first_requires_matching_repeated_region() {
    for (raw, postcode) in [
        (
            " с. Примерно , Примерна Област обл.ПРИМЕРНА ОБЛАСТ 1234",
            Some("1234"),
        ),
        ("гр.ПРИМЕРНО, Примерна Област обл.ПРИМЕРНА ОБЛАСТ", None),
    ] {
        let p = split("BG", Some(raw.into()));
        assert_eq!(p.full_address_by_vies.as_deref(), Some(raw));
        assert_eq!(p.city.as_deref(), Some("Примерно"));
        assert_eq!(p.region.as_deref(), Some("Примерна Област"));
        assert_eq!(p.postcode.as_deref(), postcode);
        assert!(p.address.is_none() && p.address2.is_none());
    }
    for raw in [
        "с. Примерно, Друга Област обл.ПРИМЕРНА ОБЛАСТ 1234",
        "с. Примерно, Примерна Област обл.ПРИМЕРНА ОБЛАСТ 123",
        "с. Примерно, ул. Примерна 1 обл.ПРИМЕРНА ОБЛАСТ 1234",
    ] {
        let p = split("BG", Some(raw.into()));
        assert_eq!(p.full_address_by_vies.as_deref(), Some(raw));
        assert!(p.city.is_none() && p.region.is_none() && p.postcode.is_none());
    }
}

#[test]
fn bulgarian_empty_primary_and_complex_do_not_invent_delivery() {
    let raw = "- №- бл.- вх.- ет.- ап.- обл.ПРИМЕРНА, с.ПРИМЕРНО 1234";
    let parsed = split("BG", Some(raw.into()));
    assert_eq!(parsed.full_address_by_vies.as_deref(), Some(raw));
    assert!(parsed.address.is_none() && parsed.address2.is_none());
    assert_eq!(parsed.city.as_deref(), Some("Примерно"));
    let parsed = split(
        "BG",
        Some("жк - ул. Примерна №21 обл.ПРИМЕРНА, гр.ПРИМЕРНО -".into()),
    );
    assert_eq!(parsed.address.as_deref(), Some("ул. Примерна №21"));
    assert_eq!(parsed.postcode, None);
    for line in ["№0", "№1-3", "жк. Пример-1", ". №57"] {
        assert_eq!(
            crate::locations::split_delivery(line, "BG").0.as_deref(),
            Some(line)
        );
    }
    assert_eq!(
        crate::locations::split_delivery("- №-, ет. -1", "BG"),
        (None, Some("ет. -1".into()))
    );
}

#[test]
fn bulgarian_star_only_details_need_no_comma_separator() {
    let raw = "ул. Примерна №1 бл.* вх.б ет.4 ап.11 обл.ПРИМЕРНА, гр.ПРИМЕРНО 1234";
    let parsed = split("BG", Some(raw.into()));
    assert_eq!(parsed.full_address_by_vies.as_deref(), Some(raw));
    assert_eq!(parsed.address.as_deref(), Some("ул. Примерна №1"));
    assert_eq!(parsed.address2.as_deref(), Some("вх. Б ет. 4 ап. 11"));
    assert_eq!(
        crate::locations::split_delivery("бл. 2* вх. А* ет. -1 ап. *А", "BG")
            .1
            .as_deref(),
        Some("бл. 2* вх. А* ет. -1 ап. *А")
    );
}

#[test]
fn bulgarian_terminal_ground_floor_annotation_is_delivery_detail() {
    let raw = "ул. Примерна №11 /партерен етаж/ обл.ПРИМЕРНА, гр.ПРИМЕРНО 1234";
    let p = split("BG", Some(raw.into()));
    assert_eq!(p.address.as_deref(), Some("ул. Примерна №11"));
    assert_eq!(p.address2.as_deref(), Some("/партерен етаж/"));
    assert_eq!(p.full_address_by_vies.as_deref(), Some(raw));
    assert_eq!(
        crate::locations::split_delivery("ул. Примерна №11 /ПАРТЕРЕН ЕТАЖ/", "BG")
            .1
            .as_deref(),
        Some("/ПАРТЕРЕН ЕТАЖ/")
    );
    for line in [
        "ул. Пример/Друг №11",
        "ул. Партерен етаж №11",
        "ул. Примерна №11 /партерен етаж/ допълнение",
    ] {
        assert_eq!(
            crate::locations::split_delivery(line, "BG"),
            (Some(line.into()), None)
        );
    }
}
