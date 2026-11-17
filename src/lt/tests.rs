use super::*;
use crate::{CompanyLookupResult, LookupError, Provider, RegistryFailure};
use serde_json::{Value, json};
fn source() -> Value {
    json!({"_data":[{"_type":decode::SOURCE_TYPE,"_id":"synthetic-source-id","ja_kodas":"001234567","ja_pavadinimas":"Synthetic UAB","pilnas_adresas":" Source address ","statusas":{"_id":"opaque-status"},"isreg_data":"2020-01-01"}]})
}
#[test]
fn dates_references_and_address_do_not_invent_common_facts() {
    let r: CompanyLookupResult = decode::decode(
        &serde_json::to_vec(&source()).unwrap(),
        &LtBusinessId::parse("001234567").unwrap(),
    )
    .unwrap()
    .into();
    assert!(
        r.registered_office.is_none()
            && r.status.is_none()
            && r.legal_form.is_none()
            && r.vat_id.is_none()
    );
    let v = serde_json::to_value(r).unwrap();
    assert_eq!(
        v["registryDetails"]["data"]["pilnas_adresas"],
        " Source address "
    );
    assert_eq!(
        v["registryDetails"]["data"]["statusas"]["_id"],
        "opaque-status"
    );
}
#[test]
fn scope_unique_identity_and_published_name_are_required() {
    let id = LtBusinessId::parse("001234567").unwrap();
    for (pointer, value) in [
        ("/_data", json!([])),
        ("/_data/0/_type", json!("other")),
        ("/_data/0/ja_kodas", json!(1234567)),
        ("/_data/0/ja_pavadinimas", json!(" ")),
    ] {
        let mut s = source();
        *s.pointer_mut(pointer).unwrap() = value;
        assert!(decode::decode(&serde_json::to_vec(&s).unwrap(), &id).is_err());
    }
    let mut s = source();
    s["_data"]
        .as_array_mut()
        .unwrap()
        .push(source()["_data"][0].clone());
    assert!(decode::decode(&serde_json::to_vec(&s).unwrap(), &id).is_err());
    for bad in ["LT001234567", "1234567", "00123456９"] {
        assert!(LtBusinessId::parse(bad).is_err());
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
        assert!(request.starts_with("get /?ja_kodas=001234567&limit(2) "));
        assert!(!request.contains("authorization:") && !request.contains("cookie:"));
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
    });
    let native = LtRegistryClient::for_test(endpoint, Default::default());
    let client = crate::Client::with_test_lithuanian(native);
    let result = client.lookup_company("001234567", "LT").await.unwrap();
    assert_eq!(result.company_name, "Synthetic UAB");
    server.await.unwrap();
}

#[tokio::test]
async fn invalid_configuration_and_identifier_fail_locally() {
    let config = LtRegistryConfig {
        timeout: std::time::Duration::ZERO,
        ..Default::default()
    };
    assert!(matches!(
        LtRegistryClient::with_config(config),
        Err(LookupError {
            kind: crate::model::LookupFailure::InvalidConfiguration(_),
            ..
        })
    ));
    assert!(matches!(
        LtRegistryClient::new().unwrap().lookup("invalid").await,
        Err(LookupError {
            kind: crate::model::LookupFailure::InvalidInput,
            ..
        })
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
    let result = LtRegistryClient::for_test(endpoint, Default::default())
        .lookup("001234567")
        .await;
    assert!(matches!(
        result,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                provider: Provider::LithuanianJar,
                error: RegistryFailure::Http { status: 404 }
            },
            ..
        })
    ));
    server.await.unwrap();
}
