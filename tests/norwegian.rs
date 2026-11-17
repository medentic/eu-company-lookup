//! Norwegian identity, discovery and disabled-operation consumer contracts.
use eu_company_lookup::{
    BusinessId, Client, ClientBuilder, CompanyDetails, IdentifierScheme, LookupError, Operation,
    Provider,
};

#[test]
fn identity_and_offline_discovery_agree() {
    let id = BusinessId::parse("NO", IdentifierScheme::NoBusinessId, "001234567").unwrap();
    assert_eq!(id.as_str(), "001234567");
    assert!(BusinessId::parse("RO", IdentifierScheme::NoBusinessId, "001234567").is_err());
    assert!(Client::SUPPORTED_COMPANY_COUNTRIES.contains(&"NO"));
    let catalog = eu_company_lookup::capabilities();
    let provider = catalog
        .iter()
        .find(|c| c.country_code == "NO")
        .unwrap()
        .providers
        .iter()
        .find(|p| p.provider == Provider::NorwegianBrreg)
        .unwrap();
    assert!(provider.supports(Operation::LookupCompany, IdentifierScheme::NoBusinessId));
    let _type_check: Option<CompanyDetails> = None;
}
#[tokio::test]
async fn disabled_norwegian_lookup_fails_before_validation() {
    let client = ClientBuilder::empty().build().unwrap();
    assert!(matches!(
        client.lookup_company("not-an-id", " no ").await,
        Err(LookupError::OperationDisabled {
            operation: Operation::LookupCompany,
            identifier_scheme: IdentifierScheme::NoBusinessId
        })
    ));
    assert!(
        !client
            .capabilities()
            .iter()
            .flat_map(|c| &c.providers)
            .any(|p| p.provider == Provider::NorwegianBrreg)
    );
}
