//! External consumers cannot bypass identifier validation through conversions.
use eu_company_lookup::{BgBusinessId, VatId};

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
    for invalid in ["", "123456789", "GB123456789", "BG123-456-789"] {
        let json = serde_json::to_string(invalid).unwrap();
        assert!(serde_json::from_str::<VatId>(&json).is_err());
    }
    let vat: VatId = serde_json::from_str("\"bg 000123456\"").unwrap();
    let business: BgBusinessId = serde_json::from_str("\"000123456\"").unwrap();
    assert_eq!(vat.as_ref(), "BG000123456");
    assert_eq!(business.as_ref(), "000123456");
}
