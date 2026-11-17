//! Finnish identity, discovery and disabled-operation consumer contracts.
use eu_company_lookup::{
    BusinessId, Client, ClientBuilder, CompanyDetails, IdentifierScheme, LookupError, Operation,
    Provider,
};

#[test]
fn identity_and_offline_discovery_agree() {
    let id = BusinessId::parse("FI", IdentifierScheme::FiBusinessId, "0012345-6").unwrap();
    assert_eq!(id.as_str(), "0012345-6");
    assert!(BusinessId::parse("RO", IdentifierScheme::FiBusinessId, "0012345-6").is_err());
    assert!(Client::SUPPORTED_COMPANY_COUNTRIES.contains(&"FI"));
    let catalog = eu_company_lookup::capabilities();
    let provider = catalog
        .iter()
        .find(|c| c.country_code == "FI")
        .unwrap()
        .providers
        .iter()
        .find(|p| p.provider == Provider::FinnishPrh)
        .unwrap();
    assert!(provider.supports(Operation::LookupCompany, IdentifierScheme::FiBusinessId));
    let _type_check: Option<CompanyDetails> = None;
}
#[tokio::test]
async fn disabled_finnish_lookup_fails_before_validation() {
    let client = ClientBuilder::empty().build().unwrap();
    assert!(matches!(
        client.lookup_company("not-an-id", " fi ").await,
        Err(LookupError::OperationDisabled {
            operation: Operation::LookupCompany,
            identifier_scheme: IdentifierScheme::FiBusinessId
        })
    ));
    assert!(
        !client
            .capabilities()
            .iter()
            .flat_map(|c| &c.providers)
            .any(|p| p.provider == Provider::FinnishPrh)
    );
}
