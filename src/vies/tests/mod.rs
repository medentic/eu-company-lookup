mod http;
mod identifiers;
mod probe;
mod soap;

#[tokio::test]
#[ignore = "contacts production VIES; set VIES_LIVE_VAT and run explicitly"]
async fn live_vies() {
    let vat = std::env::var("VIES_LIVE_VAT").expect("set VIES_LIVE_VAT to an authorized VAT ID");
    let client = super::Client::with_config(super::ClientConfig {
        timeout: std::time::Duration::from_secs(20),
        ..Default::default()
    })
    .unwrap();
    let result = client
        .lookup_vat(&vat)
        .await
        .expect("live VIES lookup failed; this is not a passing verification");
    assert_eq!(result.vat_id, super::VatId::parse(&vat).unwrap());
    assert_eq!(result.source, crate::Provider::Vies);
    // The real number, owner and response body are deliberately not printed.
    eprintln!(
        "Live VIES response decoded; valid={}, source date={}, checked at={}",
        result.valid, result.request_date, result.checked_at
    );
}

#[tokio::test]
#[ignore = "live address verification; set VIES_LIVE_VAT and VIES_EXPECTED_ADDRESS_JSON"]
async fn live_address_split() {
    let vat = std::env::var("VIES_LIVE_VAT").expect("set authorized VAT ID");
    let expected: serde_json::Value = serde_json::from_str(
        &std::env::var("VIES_EXPECTED_ADDRESS_JSON")
            .expect("set independently reviewed expected address fields"),
    )
    .unwrap();
    let client = super::Client::with_config(super::ClientConfig {
        timeout: std::time::Duration::from_secs(25),
        ..Default::default()
    })
    .unwrap();
    let result = client
        .lookup_vat(&vat)
        .await
        .expect("live request must succeed");
    assert!(result.valid, "live address case must be registered");
    let actual = serde_json::to_value(&result.address).unwrap();
    eprintln!("{}", serde_json::to_string_pretty(&result).unwrap());
    assert!(
        result
            .address
            .full_address_by_vies
            .as_deref()
            .is_some_and(|s| !s.is_empty())
    );
    assert!(!expected.as_object().unwrap().is_empty());
    for (field, value) in expected.as_object().unwrap() {
        assert_eq!(actual.get(field), Some(value), "address field {field}");
    }
}
