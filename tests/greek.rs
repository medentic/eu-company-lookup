//! Greek identity, discovery and disabled-operation consumer contracts.
use eu_company_lookup::{
    BusinessId, Client, ClientBuilder, CompanyDetails, IdentifierScheme, LookupError, Operation,
    Provider,
};

#[test]
fn identity_and_offline_discovery_agree() {
    let id = BusinessId::parse("GR", IdentifierScheme::GrBusinessId, "001234567890").unwrap();
    assert_eq!(id.as_str(), "001234567890");
    assert!(BusinessId::parse("RO", IdentifierScheme::GrBusinessId, "001234567890").is_err());
    assert!(Client::SUPPORTED_COMPANY_COUNTRIES.contains(&"GR"));
    let catalog = eu_company_lookup::capabilities();
    let provider = catalog
        .iter()
        .find(|c| c.country_code == "GR")
        .unwrap()
        .providers
        .iter()
        .find(|p| p.provider == Provider::GreekGemi)
        .unwrap();
    assert!(provider.supports(Operation::LookupCompany, IdentifierScheme::GrBusinessId));
    let _type_check: Option<CompanyDetails> = None;
}
#[tokio::test]
async fn disabled_greek_lookup_fails_before_validation() {
    let client = ClientBuilder::empty().build().unwrap();
    assert!(matches!(
        client.lookup_company("not-an-id", " gr ").await,
        Err(LookupError::OperationDisabled {
            operation: Operation::LookupCompany,
            identifier_scheme: IdentifierScheme::GrBusinessId
        })
    ));
    assert!(
        !client
            .capabilities()
            .iter()
            .flat_map(|c| &c.providers)
            .any(|p| p.provider == Provider::GreekGemi)
    );
}
