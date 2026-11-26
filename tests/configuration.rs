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
        Err(LookupError {
            id: eu_company_lookup::ErrorId::InvalidConfiguration,
            ..
        })
    ));
    let mut registry = BgRegistryConfig::default();
    registry.connect_timeout = registry.timeout + Duration::from_secs(1);
    assert!(matches!(
        BgRegistryClient::with_config(registry.clone()),
        Err(LookupError {
            id: eu_company_lookup::ErrorId::InvalidConfiguration,
            ..
        })
    ));
    assert!(matches!(
        Client::with_configs(ClientConfig::default(), registry),
        Err(LookupError {
            id: eu_company_lookup::ErrorId::InvalidConfiguration,
            ..
        })
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
    send_future(client.lookup_company("000123456", "BG"));
    send_future(registry.lookup("000123456"));
    assert!(format!("{client:?}").starts_with("Client { vies:"));
    assert!(format!("{registry:?}").starts_with("BgRegistryClient { config:"));
}

#[tokio::test]
async fn pdf_requires_enabled_vies_and_two_valid_vat_ids_before_starting_browser() {
    use eu_company_lookup::{ClientBuilder, ClientConfig, ErrorId, Operation, VatId};
    let disabled = ClientBuilder::empty().build().unwrap();
    let error = disabled.vies_result_pdf("bad", "bad").await.unwrap_err();
    assert_eq!(error.id, ErrorId::OperationDisabled);
    assert_eq!(error.details.operation, Some(Operation::ViesResultPdf));
    let client = ClientBuilder::empty()
        .vies(Default::default())
        .build()
        .unwrap();
    let error = client
        .vies_result_pdf("BG123456786", None)
        .await
        .unwrap_err();
    assert_eq!(error.details.reason, Some("requester_vat_required"));
    let mut config = ClientConfig::default();
    config.requester_vat = Some(VatId::parse("FI20774740").unwrap());
    let client = ClientBuilder::empty().vies(config).build().unwrap();
    assert_eq!(
        client
            .vies_result_pdf("BG123456786", "bad")
            .await
            .unwrap_err()
            .id,
        ErrorId::InvalidInput
    );
    assert_eq!(
        client.vies_result_pdf("bad", None).await.unwrap_err().id,
        ErrorId::InvalidInput
    );
}
