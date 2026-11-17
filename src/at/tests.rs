use super::*;
use crate::{LookupError, Provider, RegistryError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SEARCH: &str =
    r#"{"numResults":1,"state":"ACTIVE","companies":[{"id":"00123a_7","fnr":"00123a"}]}"#;
const DETAIL: &str = r#"{"id":"00123a_7","fnr":"00123a","name":" Example GmbH ","status":"ACTIVE","address":{"zipCode":"0010","street":"Source 1","city":"Town"}}"#;

#[test]
fn shared_mapping_keeps_unsupported_facts_unknown() {
    let id = AtBusinessId::parse("00123A").unwrap();
    let common: crate::CompanyLookupResult = decode::detail(DETAIL.as_bytes(), &id, "00123a_7")
        .unwrap()
        .into();
    assert_eq!(common.source, Provider::AustrianFirmenbuch);
    assert!(common.vat_id.is_none() && common.vat_registered.is_none());
    assert!(common.status.is_none() && common.registry_url.is_none());
    let address = common.registered_office.unwrap();
    assert_eq!(address.postcode.as_deref(), Some("0010"));
    assert!(address.country.is_none() && address.country_iso2.is_none());
    assert!(address.full_address_by_registry.is_none());
    let details = serde_json::to_value(common.registry_details).unwrap();
    assert_eq!(details["data"]["nativeRegistryStatus"], "ACTIVE");
}

#[test]
fn syntax_preserves_zeroes_without_claiming_registration() {
    assert_eq!(AtBusinessId::parse(" 00123 a ").unwrap().as_str(), "00123A");
    for bad in ["", "A", "1234567A", "FN123A", "ATU12345678", "123é", "123"] {
        assert!(AtBusinessId::parse(bad).is_err());
    }
}

#[test]
fn search_requires_exact_identity_and_safe_source_route() {
    let id = AtBusinessId::parse("00123A").unwrap();
    assert_eq!(decode::select(SEARCH.as_bytes(), &id).unwrap(), "00123a_7");
    for bad in [
        SEARCH.replace("00123a", "00124a"),
        SEARCH.replace("00123a_7", "../00123a_7"),
        SEARCH.replace("ACTIVE", "ALL"),
        r#"{"numResults":0,"state":"ACTIVE","companies":[]}"#.into(),
    ] {
        assert!(decode::select(bad.as_bytes(), &id).is_err());
    }
    let result = decode::detail(DETAIL.as_bytes(), &id, "00123a_7").unwrap();
    assert_eq!(result.company_name, " Example GmbH ");
    assert_eq!(
        result.registered_office.unwrap().zip_code.as_deref(),
        Some("0010")
    );
    assert!(decode::detail(DETAIL.as_bytes(), &id, "00123a_8").is_err());
}

#[tokio::test]
async fn anonymous_search_then_detail_and_capture() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for (path, body) in [
            ("/search?term=00123a&size=5&page=0", SEARCH),
            ("/00123a_7", DETAIL),
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]);
            assert!(request.starts_with(&format!("GET {path} HTTP/1.1")));
            assert!(!request.to_ascii_lowercase().contains("authorization:"));
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
        }
    });
    let client = AtRegistryClient::for_test(url, AtRegistryConfig::default());
    client.begin_capture();
    assert_eq!(
        client.lookup("00123A").await.unwrap().tax_id.as_str(),
        "00123A"
    );
    assert_eq!(
        client.take_capture()["responses"].as_array().unwrap().len(),
        2
    );
    server.await.unwrap();
}

#[tokio::test]
async fn access_failure_is_never_absence() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0; 4096];
        let n = socket.read(&mut buf).await.unwrap();
        assert!(n > 0);
        socket
            .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
    });
    assert!(matches!(
        AtRegistryClient::for_test(url, Default::default())
            .lookup("00123A")
            .await,
        Err(LookupError::Registry {
            provider: Provider::AustrianFirmenbuch,
            error: RegistryError::AccessDenied
        })
    ));
    server.await.unwrap();
}

async fn raw_endpoint(reply: &'static [u8]) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        assert!(socket.read(&mut request).await.unwrap() > 0);
        socket.write_all(reply).await.unwrap();
    });
    (url, task)
}

#[tokio::test]
async fn throttle_cooldown_is_shared_by_clones() {
    let (url, task) = raw_endpoint(b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 120\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
    let client = AtRegistryClient::for_test(url, Default::default());
    let clone = client.clone();
    assert!(matches!(
        client.lookup("00123A").await,
        Err(LookupError::Registry {
            error: RegistryError::RateLimited {
                retry_after: Some(crate::RetryAfter::Seconds(120))
            },
            ..
        })
    ));
    task.await.unwrap();
    assert!(matches!(
        clone.lookup("00123A").await,
        Err(LookupError::Registry {
            error: RegistryError::CoolingDown {
                retry_after_seconds: 119..=120
            },
            ..
        })
    ));
}

#[tokio::test]
async fn body_limits_and_non_json_never_become_records() {
    let (url, task) = raw_endpoint(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1024\r\nConnection: close\r\n\r\n").await;
    let config = AtRegistryConfig {
        max_response_bytes: 32,
        ..Default::default()
    };
    assert!(matches!(
        AtRegistryClient::for_test(url, config)
            .lookup("00123A")
            .await,
        Err(LookupError::Registry {
            error: RegistryError::ResponseTooLarge,
            ..
        })
    ));
    task.await.unwrap();
    let (url, task) = raw_endpoint(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
    assert!(matches!(
        AtRegistryClient::for_test(url, Default::default())
            .lookup("00123A")
            .await,
        Err(LookupError::Registry {
            error: RegistryError::UnexpectedResponse,
            ..
        })
    ));
    task.await.unwrap();
}
