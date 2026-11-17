//! Public 4.0 error contract; no private enums or live providers.
use eu_company_lookup::{ClientBuilder, ErrorId, LookupError, Operation, validate_vat_syntax};

#[test]
fn fallback_and_invalid_input_share_one_serializable_shape() {
    let fallback = LookupError::default();
    assert_eq!(fallback.id, ErrorId::UnknownError);
    let error = validate_vat_syntax("private-invalid-input").unwrap_err();
    assert_eq!(error.id, ErrorId::InvalidInput);
    assert_eq!(error.title, "Check the identifier");
    assert_eq!(error.to_string(), error.title);
    assert!(!error.is_service_unavailable());
    let value = serde_json::to_value(&error).unwrap();
    assert_eq!(value["id"], "invalid_input");
    assert_eq!(value["isServiceUnavailable"], false);
    assert!(value["details"]["debug"].is_null());
    assert!(value.get("kind").is_none());
    assert!(!value.to_string().contains("private-invalid-input"));
    let keys: Vec<_> = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        [
            "description",
            "details",
            "id",
            "isServiceUnavailable",
            "title"
        ]
    );
    fn error_trait(_: &(dyn std::error::Error + Send + Sync)) {}
    error_trait(&error);
}
#[tokio::test]
async fn disabled_operations_use_common_id_with_structured_details() {
    let client = ClientBuilder::empty().build().unwrap();
    let error = client.is_vat_valid("bad").await.unwrap_err();
    assert_eq!(error.id, ErrorId::OperationDisabled);
    assert_eq!(error.details.operation, Some(Operation::IsVatValid));
    assert!(error.details.debug.is_none());
    assert!(error.details.upstream_http_status.is_none());
}
