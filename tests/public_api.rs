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
    assert!(serde_json::to_value(catalog).unwrap().is_array());
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Client>();
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
}
