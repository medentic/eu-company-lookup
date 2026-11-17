//! Latvian identity, discovery and disabled-operation consumer contracts.
use eu_company_lookup::{
    BusinessId, Client, ClientBuilder, CompanyDetails, IdentifierScheme, LookupError, Operation,
    Provider,
};

#[test]
fn identity_and_offline_discovery_agree() {
    let id = BusinessId::parse("LV", IdentifierScheme::LvBusinessId, "00123456789").unwrap();
    assert_eq!(id.as_str(), "00123456789");
    assert!(BusinessId::parse("RO", IdentifierScheme::LvBusinessId, "00123456789").is_err());
    assert!(Client::SUPPORTED_COMPANY_COUNTRIES.contains(&"LV"));
    let catalog = eu_company_lookup::capabilities();
    let provider = catalog
        .iter()
        .find(|c| c.country_code == "LV")
        .unwrap()
        .providers
        .iter()
        .find(|p| p.provider == Provider::LatvianOpenData)
        .unwrap();
    assert!(provider.supports(Operation::LookupCompany, IdentifierScheme::LvBusinessId));
    let _type_check: Option<CompanyDetails> = None;
}
#[tokio::test]
async fn disabled_latvian_lookup_fails_before_validation() {
    let client = ClientBuilder::empty().build().unwrap();
    assert!(matches!(
        client.lookup_company("not-an-id", " lv ").await,
        Err(LookupError::OperationDisabled {
            operation: Operation::LookupCompany,
            identifier_scheme: IdentifierScheme::LvBusinessId
        })
    ));
    assert!(
        !client
            .capabilities()
            .iter()
            .flat_map(|c| &c.providers)
            .any(|p| p.provider == Provider::LatvianOpenData)
    );
}
