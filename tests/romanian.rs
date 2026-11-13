//! Offline Romanian consumer contract.
use eu_company_lookup::{
    Client, LookupError, Provider, RoBusinessId, RoRegistryClient, RoRegistryConfig,
};
#[test]
fn cui_is_a_string_and_vat_remains_separate() {
    let id: RoBusinessId = "00 1234".parse().unwrap();
    assert_eq!(id.as_str(), "001234");
    assert_eq!(serde_json::to_string(&id).unwrap(), "\"001234\"");
    assert_eq!(
        serde_json::from_str::<RoBusinessId>("\"001234\"").unwrap(),
        id
    );
    for bad in [
        "",
        "1",
        "RO1234",
        "12345678901",
        "1234567890123",
        "12-34",
        "１２３４",
    ] {
        assert!(RoBusinessId::parse(bad).is_err());
    }
    assert!(eu_company_lookup::VatId::parse("RO1234").is_ok());
    let catalog = Client::new().unwrap().capabilities();
    let ro = catalog.iter().find(|c| c.country_code == "RO").unwrap();
    assert_eq!(
        ro.providers.iter().map(|p| p.provider).collect::<Vec<_>>(),
        vec![Provider::Vies, Provider::RomanianAnaf]
    );
    fn send_sync<T: Send + Sync>() {}
    send_sync::<RoRegistryClient>();
    let mut config = RoRegistryConfig::default();
    config.start_interval = std::time::Duration::ZERO;
    assert!(RoRegistryClient::with_config(config).is_err());
}
#[tokio::test]
async fn invalid_input_fails_before_network() {
    let client = Client::new().unwrap();
    assert!(matches!(
        client.lookup_ro_company("RO1234").await,
        Err(LookupError::InvalidInput)
    ));
    let future =
        client.lookup_ro_company_at("1234", chrono::NaiveDate::from_ymd_opt(9999, 1, 1).unwrap());
    fn send<T: Send>(_: &T) {}
    send(&future);
    assert!(matches!(future.await, Err(LookupError::InvalidInput)));
}
