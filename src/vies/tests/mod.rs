mod http;
mod identifiers;
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
