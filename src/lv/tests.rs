use super::*;
use crate::{CompanyLookupResult, LookupError, Provider, RegistryError};
use serde_json::{Value, json};
fn source() -> Value {
    json!({"success":true,"result":{"resource_id":decode::RESOURCE,"total":1,"records":[{"regcode":"00123456789","name":"Synthetic SIA","type_text":"Source legal form","address":" Source legal address ","index":1001,"closed":" ","sepa":"Unrelated creditor code"}]}})
}
#[test]
fn legal_address_and_native_codes_keep_their_roles() {
    let r: CompanyLookupResult = decode::decode(
        &serde_json::to_vec(&source()).unwrap(),
        &LvBusinessId::parse("00123456789").unwrap(),
    )
    .unwrap()
    .into();
    assert_eq!(r.tax_id, "00123456789");
    assert!(r.vat_id.is_none() && r.vat_registered.is_none());
    assert_eq!(r.legal_form.as_deref(), Some("Source legal form"));
    let a = r.registered_office.unwrap();
    assert_eq!(
        a.full_address_by_registry.as_deref(),
        Some(" Source legal address ")
    );
    assert!(a.postcode.is_none() && a.city.is_none());
}
#[test]
fn exact_resource_unique_record_and_identity_required() {
    let id = LvBusinessId::parse("00123456789").unwrap();
    for (pointer, value) in [
        ("/success", json!(false)),
        ("/result/resource_id", json!("other")),
        ("/result/total", json!(2)),
        ("/result/records", json!([])),
        ("/result/records/0/regcode", json!(123456789)),
        ("/result/records/0/name", json!(" ")),
    ] {
        let mut s = source();
        *s.pointer_mut(pointer).unwrap() = value;
        assert!(decode::decode(&serde_json::to_vec(&s).unwrap(), &id).is_err());
    }
    let mut s = source();
    s["result"]["records"][0] = json!({"regcode":40123456789u64,"name":"Numeric"});
    assert!(
        decode::decode(
            &serde_json::to_vec(&s).unwrap(),
            &LvBusinessId::parse("40123456789").unwrap()
        )
        .is_ok()
    );
    for bad in ["LV00123456789", "0012345678", "001234567８９"] {
        assert!(LvBusinessId::parse(bad).is_err());
    }
}
#[tokio::test]
async fn anonymous_exact_query_reaches_shared_facade() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let body = source().to_string();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0; 4096];
        let n = socket.read(&mut buffer).await.unwrap();
        let request = String::from_utf8_lossy(&buffer[..n]).to_lowercase();
        let path = request.split_whitespace().nth(1).unwrap();
        let url = reqwest::Url::parse(&format!("http://localhost{path}")).unwrap();
        let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(query["resource_id"], decode::RESOURCE);
        assert_eq!(query["limit"], "2");
        assert_eq!(
            serde_json::from_str::<Value>(&query["filters"]).unwrap(),
            json!({"regcode":"00123456789"})
        );
        assert!(!request.contains("authorization:") && !request.contains("cookie:"));
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
    });
    let native = LvRegistryClient::for_test(endpoint, Default::default());
    let client = crate::Client::with_test_latvian(native);
    let result = client.lookup_company("00123456789", "LV").await.unwrap();
    assert_eq!(result.company_name, "Synthetic SIA");
    server.await.unwrap();
}

#[tokio::test]
async fn invalid_configuration_and_identifier_fail_locally() {
    let config = LvRegistryConfig {
        timeout: std::time::Duration::ZERO,
        ..Default::default()
    };
    assert!(matches!(
        LvRegistryClient::with_config(config),
        Err(LookupError::InvalidConfiguration(_))
    ));
    assert!(matches!(
        LvRegistryClient::new().unwrap().lookup("invalid").await,
        Err(LookupError::InvalidInput)
    ));
}

#[tokio::test]
async fn endpoint_404_is_an_operational_error_not_registry_absence() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0; 4096];
        let n = socket.read(&mut buffer).await.unwrap();
        assert!(n > 0);
        socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await.unwrap();
    });
    let result = LvRegistryClient::for_test(endpoint, Default::default())
        .lookup("00123456789")
        .await;
    assert!(matches!(
        result,
        Err(LookupError::Registry {
            provider: Provider::LatvianOpenData,
            error: RegistryError::Http { status: 404 }
        })
    ));
    server.await.unwrap();
}
