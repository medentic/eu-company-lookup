//! Belgian identity, discovery and disabled-operation consumer contracts.
use eu_company_lookup::{
    BusinessId, Client, ClientBuilder, CompanyDetails, IdentifierScheme, LookupError, Operation,
    Provider,
};

#[test]
fn identity_and_offline_discovery_agree() {
    let id = BusinessId::parse("BE", IdentifierScheme::BeBusinessId, "0012345678").unwrap();
    assert_eq!(id.as_str(), "0012345678");
    assert!(BusinessId::parse("RO", IdentifierScheme::BeBusinessId, "0012345678").is_err());
    assert!(Client::SUPPORTED_COMPANY_COUNTRIES.contains(&"BE"));
    let catalog = eu_company_lookup::capabilities();
    let provider = catalog
        .iter()
        .find(|c| c.country_code == "BE")
        .unwrap()
        .providers
        .iter()
        .find(|p| p.provider == Provider::BelgianNbb)
        .unwrap();
    assert!(provider.supports(Operation::LookupCompany, IdentifierScheme::BeBusinessId));
    let _type_check: Option<CompanyDetails> = None;
}
#[tokio::test]
async fn disabled_belgian_lookup_fails_before_validation() {
    let client = ClientBuilder::empty().build().unwrap();
    assert!(matches!(
        client.lookup_company("not-an-id", " be ").await,
        Err(LookupError::OperationDisabled {
            operation: Operation::LookupCompany,
            identifier_scheme: IdentifierScheme::BeBusinessId
        })
    ));
    assert!(
        !client
            .capabilities()
            .iter()
            .flat_map(|c| &c.providers)
            .any(|p| p.provider == Provider::BelgianNbb)
    );
}
