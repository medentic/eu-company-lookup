//! Provider-independent facade, configuration and discovery contracts.
use eu_company_lookup::{
    BusinessId, Client, ClientBuilder, ClientConfig, DateInput, Execution, IdentifierScheme,
    LookupError, Operation, Provider, capabilities,
};

#[test]
fn discovery_is_independent_of_client_enablement() {
    let compiled = capabilities(); // No client construction and no runtime required.
    assert_eq!(compiled, Client::new().unwrap().capabilities());
    let disabled = ClientBuilder::empty().build().unwrap();
    assert!(
        disabled
            .capabilities()
            .iter()
            .all(|c| c.providers.is_empty())
    );
    let ro = ClientBuilder::empty()
        .romanian_registry(Default::default())
        .build()
        .unwrap();
    let enabled: Vec<_> = ro
        .capabilities()
        .into_iter()
        .flat_map(|c| c.providers)
        .collect();
    assert_eq!(enabled.len(), 1);
    assert_eq!(enabled[0].provider, Provider::RomanianAnaf);
    assert!(
        compiled
            .iter()
            .flat_map(|c| &c.providers)
            .any(|p| p.provider == Provider::Vies)
    );
}

#[test]
fn named_configuration_validates_only_enabled_adapters() {
    let mut invalid = ClientConfig::default();
    invalid.timeout = std::time::Duration::ZERO;
    assert!(matches!(
        ClientBuilder::empty().vies(invalid.clone()).build(),
        Err(LookupError::InvalidConfiguration(_))
    ));
    assert!(
        ClientBuilder::empty()
            .vies(invalid)
            .without_vies()
            .build()
            .is_ok()
    );
    let client = Client::builder()
        .without_bulgarian_registry()
        .without_romanian_registry()
        .build()
        .unwrap();
    assert!(
        client
            .capabilities()
            .iter()
            .flat_map(|c| &c.providers)
            .all(|p| p.provider == Provider::Vies)
    );
}

#[tokio::test]
async fn disabled_operations_are_local_errors_never_negative_observations() {
    let client = ClientBuilder::empty().build().unwrap();
    for (country, scheme, value) in [
        ("BG", IdentifierScheme::BgBusinessId, "123456789"),
        ("RO", IdentifierScheme::RoBusinessId, "001234"),
    ] {
        let id = BusinessId::parse(country, scheme, value).unwrap();
        let error = client.lookup_company(&id).await.unwrap_err();
        assert!(
            matches!(error, LookupError::OperationDisabled { operation: Operation::LookupCompany, identifier_scheme } if identifier_scheme == scheme)
        );
        assert!(!error.is_service_unavailable());
    }
    assert!(matches!(
        client.lookup_vat("BG123456789").await,
        Err(LookupError::OperationDisabled {
            operation: Operation::LookupVat,
            ..
        })
    ));
    assert!(matches!(
        client.is_vat_valid("BG123456789").await,
        Err(LookupError::OperationDisabled {
            operation: Operation::IsVatValid,
            ..
        })
    ));
    assert!(matches!(
        client.lookup_bg_company("123456789").await,
        Err(LookupError::OperationDisabled {
            operation: Operation::LookupBgCompany,
            ..
        })
    ));
    assert!(matches!(
        client.lookup_ro_company("1234").await,
        Err(LookupError::OperationDisabled {
            operation: Operation::LookupRoCompany,
            ..
        })
    ));
    assert!(matches!(
        client
            .lookup_ro_company_at("1234", chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
            .await,
        Err(LookupError::OperationDisabled {
            operation: Operation::LookupRoCompanyAt,
            ..
        })
    ));
}

#[test]
fn operation_contracts_describe_real_input_and_execution_behavior() {
    let catalog = capabilities();
    for provider in catalog.iter().flat_map(|c| &c.providers) {
        for op in &provider.operations {
            assert!(!op.identifier_schemes.is_empty());
            for scheme in &op.identifier_schemes {
                assert!(provider.supports(op.operation, *scheme));
            }
            if op.operation == Operation::LookupRoCompanyAt {
                assert_eq!(provider.provider, Provider::RomanianAnaf);
                assert_eq!(op.identifier_schemes, [IdentifierScheme::RoBusinessId]);
                assert_eq!(op.date_input, DateInput::FiscalDate);
                assert_eq!(op.execution, Execution::Remote);
            } else {
                assert_eq!(op.date_input, DateInput::None);
            }
            if matches!(
                op.operation,
                Operation::ValidateVatSyntax
                    | Operation::ValidateBgBusinessIdSyntax
                    | Operation::ValidateRoBusinessIdSyntax
            ) {
                assert_eq!(op.execution, Execution::Local);
            } else {
                assert_eq!(op.execution, Execution::Remote);
            }
        }
        assert!(!provider.supports(Operation::LookupVat, IdentifierScheme::RoBusinessId));
    }
    let wire = serde_json::to_value(catalog).unwrap();
    assert!(wire[0]["providers"][0]["operations"][0]["identifierSchemes"].is_array());
}

#[test]
fn unsupported_scheme_is_distinct_from_malformed_national_value() {
    assert!(matches!(
        BusinessId::parse("NO", IdentifierScheme::BgBusinessId, "123456789"),
        Err(LookupError::UnsupportedIdentifierScheme)
    ));
    assert!(matches!(
        BusinessId::parse("BG", IdentifierScheme::BgBusinessId, "bad"),
        Err(LookupError::InvalidInput)
    ));
}
