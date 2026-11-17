//! Polish identity, discovery and disabled-operation consumer contracts.
use eu_company_lookup::{
    BusinessId, Client, ClientBuilder, CompanyDetails, IdentifierScheme, LookupError, Operation,
    Provider,
};

#[test]
fn identity_and_offline_discovery_agree() {
    let id = BusinessId::parse("PL", IdentifierScheme::PlBusinessId, "0001234567").unwrap();
    assert_eq!(id.as_str(), "0001234567");
    assert!(BusinessId::parse("RO", IdentifierScheme::PlBusinessId, "0001234567").is_err());
    assert!(Client::SUPPORTED_COMPANY_COUNTRIES.contains(&"PL"));
    let catalog = eu_company_lookup::capabilities();
    let provider = catalog
        .iter()
        .find(|c| c.country_code == "PL")
        .unwrap()
        .providers
        .iter()
        .find(|p| p.provider == Provider::PolishKrs)
        .unwrap();
    assert!(provider.supports(Operation::LookupCompany, IdentifierScheme::PlBusinessId));
    let _type_check: Option<CompanyDetails> = None;
}
#[tokio::test]
async fn disabled_polish_lookup_fails_before_validation() {
    let client = ClientBuilder::empty().build().unwrap();
    assert!(matches!(
        client.lookup_company("not-an-id", " pl ").await,
        Err(LookupError::OperationDisabled {
            operation: Operation::LookupCompany,
            identifier_scheme: IdentifierScheme::PlBusinessId
        })
    ));
    assert!(
        !client
            .capabilities()
            .iter()
            .flat_map(|c| &c.providers)
            .any(|p| p.provider == Provider::PolishKrs)
    );
}
