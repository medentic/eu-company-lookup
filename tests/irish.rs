//! Irish identity, discovery and disabled-operation consumer contracts.
use eu_company_lookup::{
    BusinessId, Client, ClientBuilder, CompanyDetails, IdentifierScheme, LookupError, Operation,
    Provider,
};

#[test]
fn identity_and_offline_discovery_agree() {
    let id = BusinessId::parse("IE", IdentifierScheme::IeBusinessId, "0123456").unwrap();
    assert_eq!(id.as_str(), "0123456");
    assert!(BusinessId::parse("RO", IdentifierScheme::IeBusinessId, "0123456").is_err());
    assert!(Client::SUPPORTED_COMPANY_COUNTRIES.contains(&"IE"));
    let catalog = eu_company_lookup::capabilities();
    let provider = catalog
        .iter()
        .find(|c| c.country_code == "IE")
        .unwrap()
        .providers
        .iter()
        .find(|p| p.provider == Provider::IrishCro)
        .unwrap();
    assert!(provider.supports(Operation::LookupCompany, IdentifierScheme::IeBusinessId));
    let _type_check: Option<CompanyDetails> = None;
}
#[tokio::test]
async fn disabled_irish_lookup_fails_before_validation() {
    let client = ClientBuilder::empty().build().unwrap();
    assert!(matches!(
        client.lookup_company("not-an-id", " ie ").await,
        Err(error @ LookupError { id: eu_company_lookup::ErrorId::OperationDisabled, .. }) if error.details.operation == Some(Operation::LookupCompany) && error.details.identifier_scheme == Some(IdentifierScheme::IeBusinessId)
    ));
    assert!(
        !client
            .capabilities()
            .iter()
            .flat_map(|c| &c.providers)
            .any(|p| p.provider == Provider::IrishCro)
    );
}
