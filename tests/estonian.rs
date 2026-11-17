//! Estonian identity, discovery and disabled-operation consumer contracts.
use eu_company_lookup::{
    BusinessId, Client, ClientBuilder, CompanyDetails, IdentifierScheme, LookupError, Operation,
    Provider,
};

#[test]
fn identity_and_offline_discovery_agree() {
    let id = BusinessId::parse("EE", IdentifierScheme::EeBusinessId, "00123456").unwrap();
    assert_eq!(id.as_str(), "00123456");
    assert!(BusinessId::parse("RO", IdentifierScheme::EeBusinessId, "00123456").is_err());
    assert!(Client::SUPPORTED_COMPANY_COUNTRIES.contains(&"EE"));
    let catalog = eu_company_lookup::capabilities();
    let provider = catalog
        .iter()
        .find(|c| c.country_code == "EE")
        .unwrap()
        .providers
        .iter()
        .find(|p| p.provider == Provider::EstonianRik)
        .unwrap();
    assert!(provider.supports(Operation::LookupCompany, IdentifierScheme::EeBusinessId));
    let _type_check: Option<CompanyDetails> = None;
}
#[tokio::test]
async fn disabled_estonian_lookup_fails_before_validation() {
    let client = ClientBuilder::empty().build().unwrap();
    assert!(matches!(
        client.lookup_company("not-an-id", " ee ").await,
        Err(LookupError::OperationDisabled {
            operation: Operation::LookupCompany,
            identifier_scheme: IdentifierScheme::EeBusinessId
        })
    ));
    assert!(
        !client
            .capabilities()
            .iter()
            .flat_map(|c| &c.providers)
            .any(|p| p.provider == Provider::EstonianRik)
    );
}
