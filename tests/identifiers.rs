//! External consumers cannot bypass identifier validation through conversions.
use eu_company_lookup::{BgBusinessId, VatChecksumResult, VatId};

#[test]
fn offline_checksum_outcomes_need_no_client_or_async_runtime() {
    for (raw, expected) in [
        ("BG423456789", VatChecksumResult::Passed),
        ("BG423456780", VatChecksumResult::Failed),
        ("FRAB123456789", VatChecksumResult::Unsupported),
        ("XIGD001", VatChecksumResult::Unsupported),
    ] {
        // A failed checksum is still a syntax-valid, serializable identifier.
        let vat = VatId::parse(raw).unwrap();
        assert_eq!(vat.validate_checksum(), expected);
        assert_eq!(vat.as_str(), raw);
        let restored: VatId = serde_json::from_str(&serde_json::to_string(&vat).unwrap()).unwrap();
        assert_eq!(restored.validate_checksum(), expected);
    }
    let vat = VatId::parse(" fi 02345678 ").unwrap();
    assert_eq!(vat.validate_checksum(), VatChecksumResult::Passed);
    assert_eq!(vat.as_str(), "FI02345678");
    // Bad syntax or an unsupported jurisdiction fails parsing, not the checksum.
    assert!(VatId::parse("BG123").is_err());
    assert!(VatId::parse("GB123456789").is_err());
}

#[test]
fn offline_checksum_outcomes_have_explicit_serialized_values() {
    for (result, expected) in [
        (VatChecksumResult::Passed, "\"passed\""),
        (VatChecksumResult::Failed, "\"failed\""),
        (VatChecksumResult::Unsupported, "\"unsupported\""),
    ] {
        assert_eq!(serde_json::to_string(&result).unwrap(), expected);
    }
}

#[test]
fn identifier_conversions_preserve_normalization_and_leading_zeroes() {
    let vat = VatId::try_from("bg 000123456").unwrap();
    assert_eq!(vat.as_ref(), "BG000123456");
    assert_eq!(vat, VatId::try_from(String::from("BG000123456")).unwrap());
    assert_eq!(serde_json::to_string(&vat).unwrap(), "\"BG000123456\"");
    assert_eq!(String::from(vat), "BG000123456");

    let business = BgBusinessId::try_from(" 0001234560001 ").unwrap();
    assert_eq!(business.as_ref(), "0001234560001");
    assert_eq!(
        business,
        BgBusinessId::try_from(String::from("0001234560001")).unwrap()
    );
    assert_eq!(
        serde_json::to_string(&business).unwrap(),
        "\"0001234560001\""
    );
    assert_eq!(String::from(business), "0001234560001");
}

#[test]
fn deserialization_validates_identifiers_instead_of_trusting_wire_values() {
    for invalid in ["", "BG123456789", "1234567890", "１２３４５６７８９"] {
        let json = serde_json::to_string(invalid).unwrap();
        assert!(serde_json::from_str::<BgBusinessId>(&json).is_err());
    }
    for invalid in [
        "",
        "123456789",
        "GB123456789",
        "BG123-456-789",
        "SE123456789012",
    ] {
        let json = serde_json::to_string(invalid).unwrap();
        assert!(serde_json::from_str::<VatId>(&json).is_err());
    }
    let vat: VatId = serde_json::from_str("\"bg 000123456\"").unwrap();
    let business: BgBusinessId = serde_json::from_str("\"000123456\"").unwrap();
    assert_eq!(vat.as_ref(), "BG000123456");
    assert_eq!(business.as_ref(), "000123456");
}
