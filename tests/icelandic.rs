//! Icelandic identity, discovery and disabled-operation consumer contracts.
use eu_company_lookup::{
    BusinessId, Client, ClientBuilder, CompanyDetails, IdentifierScheme, LookupError, Operation,
    Provider,
};

#[test]
fn identity_and_offline_discovery_agree() {
    let id = BusinessId::parse("IS", IdentifierScheme::IsBusinessId, "4101000000").unwrap();
    assert_eq!(id.as_str(), "4101000000");
    assert!(BusinessId::parse("RO", IdentifierScheme::IsBusinessId, "4101000000").is_err());
    assert!(Client::SUPPORTED_COMPANY_COUNTRIES.contains(&"IS"));
    let catalog = eu_company_lookup::capabilities();
    let provider = catalog
        .iter()
        .find(|c| c.country_code == "IS")
        .unwrap()
        .providers
        .iter()
        .find(|p| p.provider == Provider::IcelandicSkatturinn)
        .unwrap();
    assert!(provider.supports(Operation::LookupCompany, IdentifierScheme::IsBusinessId));
    let _type_check: Option<CompanyDetails> = None;
}
#[tokio::test]
async fn disabled_icelandic_lookup_fails_before_validation() {
    let client = ClientBuilder::empty().build().unwrap();
    assert!(matches!(
        client.lookup_company("not-an-id", " is ").await,
        Err(error @ LookupError { id: eu_company_lookup::ErrorId::OperationDisabled, .. }) if error.details.operation == Some(Operation::LookupCompany) && error.details.identifier_scheme == Some(IdentifierScheme::IsBusinessId)
    ));
    assert!(
        !client
            .capabilities()
            .iter()
            .flat_map(|c| &c.providers)
            .any(|p| p.provider == Provider::IcelandicSkatturinn)
    );
}
