//! Slovak identity, discovery and disabled-operation consumer contracts.
use eu_company_lookup::{
    BusinessId, Client, ClientBuilder, CompanyDetails, IdentifierScheme, LookupError, Operation,
    Provider,
};

#[test]
fn identity_and_offline_discovery_agree() {
    let id = BusinessId::parse("SK", IdentifierScheme::SkBusinessId, "00123456").unwrap();
    assert_eq!(id.as_str(), "00123456");
    assert!(BusinessId::parse("RO", IdentifierScheme::SkBusinessId, "00123456").is_err());
    assert!(Client::SUPPORTED_COMPANY_COUNTRIES.contains(&"SK"));
    let catalog = eu_company_lookup::capabilities();
    let provider = catalog
        .iter()
        .find(|c| c.country_code == "SK")
        .unwrap()
        .providers
        .iter()
        .find(|p| p.provider == Provider::SlovakRuz)
        .unwrap();
    assert!(provider.supports(Operation::LookupCompany, IdentifierScheme::SkBusinessId));
    let _type_check: Option<CompanyDetails> = None;
}
#[tokio::test]
async fn disabled_slovak_lookup_fails_before_validation() {
    let client = ClientBuilder::empty().build().unwrap();
    assert!(matches!(
        client.lookup_company("not-an-id", " sk ").await,
        Err(error @ LookupError { id: eu_company_lookup::ErrorId::OperationDisabled, .. }) if error.details.operation == Some(Operation::LookupCompany) && error.details.identifier_scheme == Some(IdentifierScheme::SkBusinessId)
    ));
    assert!(
        !client
            .capabilities()
            .iter()
            .flat_map(|c| &c.providers)
            .any(|p| p.provider == Provider::SlovakRuz)
    );
}
