//! French identity, discovery and disabled-operation consumer contracts.
use eu_company_lookup::{
    BusinessId, Client, ClientBuilder, CompanyDetails, IdentifierScheme, LookupError, Operation,
    Provider,
};

#[test]
fn identity_and_offline_discovery_agree() {
    let id = BusinessId::parse("FR", IdentifierScheme::FrBusinessId, "001234567").unwrap();
    assert_eq!(id.as_str(), "001234567");
    assert!(BusinessId::parse("RO", IdentifierScheme::FrBusinessId, "001234567").is_err());
    assert!(Client::SUPPORTED_COMPANY_COUNTRIES.contains(&"FR"));
    let catalog = eu_company_lookup::capabilities();
    let provider = catalog
        .iter()
        .find(|c| c.country_code == "FR")
        .unwrap()
        .providers
        .iter()
        .find(|p| p.provider == Provider::FrenchRechercheEntreprises)
        .unwrap();
    assert!(provider.supports(Operation::LookupCompany, IdentifierScheme::FrBusinessId));
    let _type_check: Option<CompanyDetails> = None;
}
#[tokio::test]
async fn disabled_french_lookup_fails_before_validation() {
    let client = ClientBuilder::empty().build().unwrap();
    assert!(matches!(
        client.lookup_company("not-an-id", " fr ").await,
        Err(LookupError::OperationDisabled {
            operation: Operation::LookupCompany,
            identifier_scheme: IdentifierScheme::FrBusinessId
        })
    ));
    assert!(
        !client
            .capabilities()
            .iter()
            .flat_map(|c| &c.providers)
            .any(|p| p.provider == Provider::FrenchRechercheEntreprises)
    );
}
