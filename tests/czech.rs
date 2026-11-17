//! Czech identity, discovery and disabled-operation consumer contracts.
use eu_company_lookup::{
    BusinessId, Client, ClientBuilder, CompanyDetails, IdentifierScheme, LookupError, Operation,
    Provider,
};

#[test]
fn identity_and_offline_discovery_agree() {
    let id = BusinessId::parse("CZ", IdentifierScheme::CzBusinessId, "00123456").unwrap();
    assert_eq!(id.as_str(), "00123456");
    assert!(BusinessId::parse("RO", IdentifierScheme::CzBusinessId, "00123456").is_err());
    assert!(Client::SUPPORTED_COMPANY_COUNTRIES.contains(&"CZ"));
    let catalog = eu_company_lookup::capabilities();
    let provider = catalog
        .iter()
        .find(|c| c.country_code == "CZ")
        .unwrap()
        .providers
        .iter()
        .find(|p| p.provider == Provider::CzechAres)
        .unwrap();
    assert!(provider.supports(Operation::LookupCompany, IdentifierScheme::CzBusinessId));
    let _type_check: Option<CompanyDetails> = None;
}
#[tokio::test]
async fn disabled_czech_lookup_fails_before_validation() {
    let client = ClientBuilder::empty().build().unwrap();
    assert!(matches!(
        client.lookup_company("not-an-id", " cz ").await,
        Err(LookupError::OperationDisabled {
            operation: Operation::LookupCompany,
            identifier_scheme: IdentifierScheme::CzBusinessId
        })
    ));
    assert!(
        !client
            .capabilities()
            .iter()
            .flat_map(|c| &c.providers)
            .any(|p| p.provider == Provider::CzechAres)
    );
}
