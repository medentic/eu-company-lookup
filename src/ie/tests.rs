use super::*;
use crate::{CompanyLookupResult, LookupError, Provider, RegistryFailure};
use serde_json::{Value, json};
fn source() -> Value {
    json!({"success":true,"result":{"resource_id":decode::RESOURCE,"total":1,"records":[{"company_num":"0123456","company_name":"Synthetic Limited","company_type":"Private limited by shares","company_status":"Dissolved","company_address_1":" Original line ","eircode":"A00 A000"}]}})
}
#[test]
fn native_status_dates_and_address_keep_source_roles() {
    let r: CompanyLookupResult = decode::decode(
        &serde_json::to_vec(&source()).unwrap(),
        &IeBusinessId::parse("0123456").unwrap(),
    )
    .unwrap()
    .into();
    assert_eq!(r.status, Some(crate::CompanyStatus::Unknown));
    assert!(r.registered_office.is_none() && r.vat_id.is_none() && r.vat_registered.is_none());
    let j = serde_json::to_value(r).unwrap();
    assert_eq!(
        j["registryDetails"]["data"]["company_address_1"],
        " Original line "
    );
    assert_eq!(
        j["registryDetails"]["data"]["nativeRegistryStatus"],
        "Dissolved"
    );
}
#[test]
fn unique_exact_company_number_and_scope_required() {
    let id = IeBusinessId::parse("0123456").unwrap();
    for (pointer, value) in [
        ("/success", json!(false)),
        ("/result/resource_id", json!("other")),
        ("/result/total", json!(2)),
        ("/result/records", json!([])),
        ("/result/records/0/company_num", json!(123456)),
        ("/result/records/0/company_name", json!(" ")),
    ] {
        let mut s = source();
        *s.pointer_mut(pointer).unwrap() = value;
        assert!(decode::decode(&serde_json::to_vec(&s).unwrap(), &id).is_err());
    }
    assert!(IeBusinessId::parse("1").is_ok());
    for bad in ["IE0123456", "12345678", "１２３４５６"] {
        assert!(IeBusinessId::parse(bad).is_err());
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
            json!({"company_num":"0123456"})
        );
        assert!(!request.contains("authorization:") && !request.contains("cookie:"));
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
    });
    let native = IeRegistryClient::for_test(endpoint, Default::default());
    let client = crate::Client::with_test_irish(native);
    let result = client.lookup_company("0123456", "IE").await.unwrap();
    assert_eq!(result.company_name, "Synthetic Limited");
    server.await.unwrap();
}

#[tokio::test]
async fn invalid_configuration_and_identifier_fail_locally() {
    let config = IeRegistryConfig {
        timeout: std::time::Duration::ZERO,
        ..Default::default()
    };
    assert!(matches!(
        IeRegistryClient::with_config(config),
        Err(LookupError {
            kind: crate::model::LookupFailure::InvalidConfiguration(_),
            ..
        })
    ));
    assert!(matches!(
        IeRegistryClient::new().unwrap().lookup("invalid").await,
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
    let result = IeRegistryClient::for_test(endpoint, Default::default())
        .lookup("0123456")
        .await;
    assert!(matches!(
        result,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                provider: Provider::IrishCro,
                error: RegistryFailure::Http { status: 404 }
            },
            ..
        })
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn shared_transport_config_enables_raw_error_capture() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0; 4096];
        assert!(socket.read(&mut buf).await.unwrap() > 0);
        socket.write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Type: text/plain\r\nContent-Length: 11\r\n\r\nexact error").await.unwrap();
    });
    let config = IeRegistryConfig {
        raw_debug: true,
        ..Default::default()
    };
    let error = IeRegistryClient::for_test(url, config)
        .lookup("0123456")
        .await
        .unwrap_err();
    assert_eq!(error.id, crate::ErrorId::AccessDenied);
    assert_eq!(error.details.country.as_deref(), Some("IE"));
    assert_eq!(error.details.upstream_http_status, Some(403));
    let debug = error.details.debug.unwrap();
    assert_eq!(
        debug.exchanges[0].response.as_ref().unwrap().body.data,
        "exact error"
    );
    assert!(debug.exchanges[0].request.url.contains("0123456"));
    server.await.unwrap();
}
