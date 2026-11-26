use super::super::{Client, VatId, validate_vat_syntax};
use crate::{ChecksumSupport, LanguageSupport, LookupError};

pub(super) const EXAMPLES: &[&str] = &[
    "ATU12345678",
    "BE0123456789",
    "BG123456789",
    "CY12345678A",
    "CZ12345678",
    "DE123456789",
    "DK12345678",
    "EE123456789",
    "EL123456789",
    "ESA12345678",
    "FI12345678",
    "FRAB123456789",
    "HR12345678901",
    "HU12345678",
    "IE1234567A",
    "IT12345678901",
    "LT123456789",
    "LU12345678",
    "LV12345678901",
    "MT12345678",
    "NL123456789B01",
    "PL1234567890",
    "PT123456789",
    "RO12",
    "SE123456789001",
    "SI12345678",
    "SK1234567890",
    "XI123456789",
];

// One malformed national structure per supported prefix; all values are synthetic.
pub(super) const INVALID_EXAMPLES: &[&str] = &[
    "ATA12345678",
    "BE2123456789",
    "BG12345678",
    "CY123456789",
    "CZ1234567",
    "DE12345678",
    "DK1234567",
    "EE12345678",
    "EL12345678",
    "ES123456789",
    "FI1234567",
    "FRAB12345678",
    "HR1234567890",
    "HU1234567",
    "IE12345678",
    "IT1234567890",
    "LT1234567890",
    "LU1234567",
    "LV1234567890",
    "MT1234567",
    "NL123456789A01",
    "PL123456789",
    "PT12345678",
    "RO1",
    "SE123456789012",
    "SI1234567",
    "SK123456789",
    "XI12345678",
];

#[test]
fn malformed_national_formats_cover_every_supported_prefix() {
    let prefixes: std::collections::BTreeSet<_> = super::super::identifiers::FORMATS
        .iter()
        .map(|(prefix, _)| *prefix)
        .collect();
    assert_eq!(INVALID_EXAMPLES.len(), prefixes.len());
    assert_eq!(
        INVALID_EXAMPLES
            .iter()
            .map(|raw| &raw[..2])
            .collect::<std::collections::BTreeSet<_>>(),
        prefixes
    );
    for raw in INVALID_EXAMPLES {
        assert!(
            matches!(
                VatId::parse(raw),
                Err(LookupError {
                    kind: crate::model::LookupFailure::InvalidInput,
                    ..
                })
            ),
            "{raw}"
        );
    }
}

#[test]
fn swedish_suffix_is_checked_without_losing_leading_zeroes() {
    let vat = VatId::parse(" se 0001234567 01 ").unwrap();
    assert_eq!(vat.as_str(), "SE000123456701");
    assert_eq!(vat.number(), "000123456701");
    for raw in ["SE000123456700", "SE000123456702", "SE000123456799"] {
        assert!(VatId::parse(raw).is_err());
        assert!(serde_json::from_str::<VatId>(&format!("\"{raw}\"")).is_err());
    }
}

#[test]
fn formats_and_capabilities_cover_the_same_jurisdictions() {
    let capabilities = Client::new().unwrap().capabilities();
    assert_eq!(capabilities.len(), 31);
    assert_eq!(
        EXAMPLES.len(),
        capabilities
            .iter()
            .filter(|c| c.vat_prefix.is_some())
            .count()
    );
    for raw in EXAMPLES {
        let vat = validate_vat_syntax(raw).unwrap();
        let capability = capabilities
            .iter()
            .find(|c| c.vat_prefix == Some(vat.prefix()))
            .unwrap();
        let vat_providers: Vec<_> = capability
            .providers
            .iter()
            .filter(|p| p.supports(crate::Operation::LookupVat, crate::IdentifierScheme::Vat))
            .collect();
        assert_eq!(vat_providers.len(), 1);
        let provider = vat_providers[0];
        assert_eq!(provider.operations.len(), 4);
        assert!(provider.supports(
            crate::Operation::ViesResultPdf,
            crate::IdentifierScheme::Vat
        ));
        assert_eq!(
            provider.languages,
            LanguageSupport {
                available: vec![],
                default: None,
                selectable: false
            }
        );
        assert_eq!(provider.checksum, ChecksumSupport::SupportedFormats);
        assert!(!provider.requires_credentials);
        assert_eq!(
            VatId::parse(&format!("{}!", raw)).unwrap_err().to_string(),
            LookupError::from(crate::model::LookupFailure::InvalidInput).to_string()
        );
    }
    let greece = capabilities
        .iter()
        .find(|c| c.vat_prefix == Some("EL"))
        .unwrap();
    assert_eq!(greece.country_code, "GR");
    let ni = capabilities
        .iter()
        .find(|c| c.vat_prefix == Some("XI"))
        .unwrap();
    assert_eq!(
        (ni.country_code, ni.subdivision_code),
        ("GB", Some("GB-NIR"))
    );
    assert!(!capabilities.iter().any(|c| c.country_code == "CH"));
}

#[test]
fn alternate_formats_and_leading_zeroes_are_preserved() {
    for raw in [
        "BE1123456789",
        "BG1234567890",
        "CZ123456789",
        "CZ1234567890",
        "ES12345678A",
        "ESX1234567A",
        "IE1234567AB",
        "IE1A12345B",
        "IE1*12345B",
        "IE1+12345B",
        "LT123456789012",
        "RO1234567890",
        "XI123456789012",
        "XIGD001",
        "XIHA999",
    ] {
        assert_eq!(VatId::parse(raw).unwrap().as_str(), raw);
    }
    let vat = VatId::parse(" \tbe 0123 456 789\n").unwrap();
    assert_eq!(vat.to_string(), "BE0123456789");
    assert_eq!(vat.number(), "0123456789");
}

#[test]
fn bad_inputs_are_rejected_without_guessing_or_panicking() {
    for raw in [
        "",
        "BG",
        "B",
        "1G1234",
        "BG12",
        "BG12345678A",
        "ＢＧ123456789",
        "БГ123456789",
        "BG123-456-789",
        "BG123.456.789",
        "BG123456789\0",
        "BG<123456789>",
        "ES123456789",
        "NL123456789A01",
        "RO1",
        "BE2123456789",
    ] {
        assert!(
            matches!(
                VatId::parse(raw),
                Err(LookupError {
                    kind: crate::model::LookupFailure::InvalidInput,
                    ..
                })
            ),
            "{raw:?}"
        );
    }
    for raw in ["GR123456789", "GB123456789", "CH123456789", "US123456789"] {
        assert!(matches!(
            VatId::parse(raw),
            Err(LookupError {
                kind: crate::model::LookupFailure::UnsupportedJurisdiction,
                ..
            })
        ));
    }
    assert!(VatId::parse(&" ".repeat(65)).is_err());
    for ch in ['é', 'λ', '\0', '💳', '<', '&'] {
        for position in 0..=11 {
            let mut input = "BG123456789".to_string();
            input.insert(position, ch);
            assert!(VatId::parse(&input).is_err());
        }
    }
}

#[test]
fn deserialization_cannot_bypass_the_identifier_invariant() {
    let vat: VatId = serde_json::from_str("\"bg 123456789\"").unwrap();
    assert_eq!(serde_json::to_string(&vat).unwrap(), "\"BG123456789\"");
    assert!(serde_json::from_str::<VatId>("\"<untrusted>\"").is_err());
}
