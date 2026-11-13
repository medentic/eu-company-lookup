//! Consumer-facing API contracts, without live network access.
use eu_company_lookup::{Client, ClientConfig, LookupError, Provider, VatId, validate_vat_syntax};

#[test]
fn consumer_can_normalize_identifiers_and_discover_supported_sources() {
    let vat: VatId = "bg 123456789".parse().unwrap();
    assert_eq!(vat, validate_vat_syntax("BG123456789").unwrap());
    let client = Client::with_config(ClientConfig::default()).unwrap();
    let catalog = client.capabilities();
    let bg = catalog.iter().find(|c| c.country_code == "BG").unwrap();
    assert_eq!(bg.providers[0].provider, Provider::Vies);
    assert_eq!(bg.providers.len(), 3);
    for provider in &bg.providers[1..] {
        assert!(provider.supports(
            eu_company_lookup::Operation::LookupCompany,
            eu_company_lookup::IdentifierScheme::BgBusinessId
        ));
        assert_eq!(provider.languages.available, ["bg"]);
        assert_eq!(provider.languages.default, Some("bg"));
        assert!(!provider.languages.selectable);
    }
    assert!(serde_json::to_value(catalog).unwrap().is_array());
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Client>();
    send_sync::<eu_company_lookup::BgRegistryClient>();
}

#[tokio::test]
async fn consumer_gets_typed_errors_instead_of_false() {
    let client = Client::new().unwrap();
    assert!(matches!(
        client.is_vat_valid("not-a-vat").await,
        Err(LookupError::InvalidInput)
    ));
    assert!(matches!(
        client.lookup_vat("GB123456789").await,
        Err(LookupError::UnsupportedJurisdiction)
    ));
    assert!(matches!(
        client.lookup_bg_company("BG123456789").await,
        Err(LookupError::InvalidInput)
    ));
    assert!(LookupError::ServiceUnavailable { retry_after: None }.is_service_unavailable());
    assert!(!LookupError::NotFound.is_service_unavailable());
}

#[test]
fn locations_are_independent_and_country_metadata_is_complete() {
    use eu_company_lookup::locations::{self, Country};
    assert_eq!(Country::ALL.iter().filter(|c| c.is_eu()).count(), 27);
    assert_eq!(Country::ALL.iter().filter(|c| c.is_eea()).count(), 30);
    assert_eq!(Country::BG.native_name(), "България");
    assert_eq!(Country::from_iso2("bg"), Some(Country::BG));
    assert_eq!(Country::from_name("БЪЛГАРИЯ"), Some(Country::BG));
    assert_eq!(Country::from_iso2("EL"), None);
    assert_eq!(Country::from_iso2("XI"), None);
    for country in Country::ALL {
        assert_eq!(Country::from_iso2(country.iso2()), Some(*country));
        assert_eq!(Country::from_name(country.native_name()), Some(*country));
        assert_eq!(serde_json::to_value(country).unwrap(), country.iso2());
    }
    let address: locations::Address = locations::parse_address("ул. Тест 1, гр. СОФИЯ 1000", "bg");
    assert_eq!(address.city.as_deref(), Some("София"));
    assert_eq!(address.postcode.as_deref(), Some("1000"));
    assert!(address.country_iso2.is_none()); // Locale selects rules, not country evidence.
    assert!(!locations::supports_address("no"));
    assert!(
        locations::parse_address("Unrecognized", "xx")
            .address
            .is_none()
    );
}

#[test]
fn typed_registry_values_preserve_wire_strings_and_unknown_source_values() {
    use eu_company_lookup::{CompanyStatus, LegalForm, RegistryDocumentType, RegistryRole};
    for (value, expected) in [
        (CompanyStatus::Active, "active"),
        (CompanyStatus::Closed, "closed"),
        (CompanyStatus::Unknown, "unknown"),
    ] {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
    for (value, expected) in [
        (LegalForm::Eood, "ЕООД"),
        (LegalForm::Ood, "ООД"),
        (LegalForm::Ad, "АД"),
        (LegalForm::Ead, "ЕАД"),
        (LegalForm::Other("Сдружение".into()), "Сдружение"),
    ] {
        assert_eq!(serde_json::to_value(value).unwrap(), expected);
    }
    assert_eq!(
        serde_json::to_value(RegistryRole::SoleCapitalOwner).unwrap(),
        "soleCapitalOwner"
    );
    assert_eq!(
        serde_json::to_value(RegistryDocumentType::ConstitutiveDocument).unwrap(),
        "constitutiveDocument"
    );
}

#[test]
fn every_company_registry_result_uses_the_same_status_type() {
    use eu_company_lookup::{BgCompanyLookupResult, CompanyStatus, RoCompanyLookupResult};
    fn bg(value: &BgCompanyLookupResult) -> Option<CompanyStatus> {
        value.status
    }
    fn ro(value: &RoCompanyLookupResult) -> Option<CompanyStatus> {
        value.status
    }
    let _: fn(&BgCompanyLookupResult) -> Option<CompanyStatus> = bg;
    let _: fn(&RoCompanyLookupResult) -> Option<CompanyStatus> = ro;
}
