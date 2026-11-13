//! Configuration is extensible and validated at client construction.
use eu_company_lookup::{BgRegistryClient, BgRegistryConfig, Client, ClientConfig, LookupError};
use std::time::Duration;

#[test]
fn clients_accept_customized_defaults_without_struct_literals() {
    let mut vies = ClientConfig::default();
    vies.timeout = Duration::from_secs(20);
    let mut registry = BgRegistryConfig::default();
    registry.timeout = Duration::from_secs(45);
    assert!(Client::with_configs(vies, registry.clone()).is_ok());
    assert!(BgRegistryClient::with_config(registry).is_ok());
}

#[test]
fn invalid_bounds_are_rejected_before_any_lookup() {
    let mut vies = ClientConfig::default();
    vies.max_concurrent_requests = 0;
    assert!(matches!(
        Client::with_config(vies),
        Err(LookupError::InvalidConfiguration(_))
    ));
    let mut registry = BgRegistryConfig::default();
    registry.connect_timeout = registry.timeout + Duration::from_secs(1);
    assert!(matches!(
        BgRegistryClient::with_config(registry.clone()),
        Err(LookupError::InvalidConfiguration(_))
    ));
    assert!(matches!(
        Client::with_configs(ClientConfig::default(), registry),
        Err(LookupError::InvalidConfiguration(_))
    ));
}

#[test]
fn lookup_futures_can_be_used_on_a_multithreaded_runtime() {
    fn send_future(_: impl std::future::Future + Send) {}
    let client = Client::new().unwrap();
    let registry = BgRegistryClient::new().unwrap();
    // Futures are not polled: this verifies the consumer contract without traffic.
    send_future(client.lookup_vat("BG000123456"));
    send_future(client.is_vat_valid("BG000123456"));
    send_future(client.lookup_bg_company("000123456"));
    send_future(registry.lookup("000123456"));
    assert!(format!("{client:?}").starts_with("Client { config:"));
    assert!(format!("{registry:?}").starts_with("BgRegistryClient { config:"));
}
