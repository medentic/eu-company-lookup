use super::*;
use crate::{LookupError, RegistryFailure};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn shared_view_preserves_box_and_native_status_without_vat_inference() {
    let id = BeBusinessId::parse("0012345678").unwrap();
    let raw = br#"{"cbe":"0012345678","name":"Example","streetName":"Source","streetNumber":"12A","boxNumber":"003","postalCode":"0010","countryCode":"BE","legalSituation":"Normal situation","legalSituationCode":"000"}"#;
    let common: crate::CompanyLookupResult = decode::decode(raw, &id).unwrap().into();
    assert!(common.vat_id.is_none() && common.vat_registered.is_none() && common.status.is_none());
    assert_eq!(common.source, crate::Provider::BelgianNbb);
    let a = common.registered_office.unwrap();
    assert_eq!(a.address.as_deref(), Some("Source 12A"));
    assert_eq!(a.address2.as_deref(), Some("003"));
    assert!(a.full_address_by_registry.is_none());
    let native = serde_json::to_value(common.registry_details).unwrap();
    assert_eq!(native["data"]["registeredOffice"]["boxNumber"], "003");
    assert_eq!(native["data"]["legalSituationCode"], "000");
}

#[test]
fn syntax_and_exact_identity_preserve_leading_zeroes() {
    let id = BeBusinessId::parse(" 0012345678 ").unwrap();
    assert_eq!(id.as_str(), "0012345678");
    for bad in ["123456789", "BE0012345678", "0012.345.678", "00123456789"] {
        assert!(BeBusinessId::parse(bad).is_err());
    }
    let raw = br#"{"cbe":"0012345678","name":" Example ","streetName":"Source","streetNumber":"1A","postalCode":"0010","legalFormCode":"610","legalSituationCode":"000"}"#;
    let v = decode::decode(raw, &id).unwrap();
    assert_eq!(v.company_name, " Example ");
    assert_eq!(v.registered_office.postal_code.as_deref(), Some("0010"));
    assert_eq!(v.details.legal_situation_code.as_deref(), Some("000"));
    assert!(matches!(
        decode::decode(raw, &BeBusinessId::parse("0012345679").unwrap()),
        Err(RegistryFailure::MismatchedResponse)
    ));
    for raw in [
        br#"{"cbe":"0012345678","name":" "}"#.as_slice(),
        br#"{"cbe":12345678,"name":"Example"}"#.as_slice(),
        b"null",
    ] {
        assert!(decode::decode(raw, &id).is_err());
    }
}

#[tokio::test]
async fn anonymous_get_captures_source_and_404_remains_an_error() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 4096];
        let n = socket.read(&mut request).await.unwrap();
        let request = String::from_utf8_lossy(&request[..n]);
        assert!(request.starts_with("GET /0012345678/EN HTTP/1.1"));
        assert!(!request.to_ascii_lowercase().contains("authorization:"));
        let body = r#"{"cbe":"0012345678","name":"Example"}"#;
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
    });
    let client = BeRegistryClient::for_test(url, Default::default());
    client.begin_capture();
    assert_eq!(
        client.lookup("0012345678").await.unwrap().tax_id.as_str(),
        "0012345678"
    );
    assert!(
        client.take_capture()["body"]
            .as_str()
            .unwrap()
            .contains("Example")
    );
    task.await.unwrap();
    let (url, task) =
        raw_endpoint(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await;
    assert!(matches!(
        BeRegistryClient::for_test(url, Default::default())
            .lookup("0012345678")
            .await,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                error: RegistryFailure::Http { status: 404 },
                ..
            },
            ..
        })
    ));
    task.await.unwrap();
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
    let client = BeRegistryClient::for_test(url, Default::default());
    let clone = client.clone();
    assert!(matches!(
        client.lookup("0012345678").await,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                error: RegistryFailure::RateLimited {
                    retry_after: Some(crate::RetryAfter::Seconds(120))
                },
                ..
            },
            ..
        })
    ));
    task.await.unwrap();
    assert!(matches!(
        clone.lookup("0012345678").await,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                error: RegistryFailure::CoolingDown {
                    retry_after_seconds: 119..=120
                },
                ..
            },
            ..
        })
    ));
}

#[tokio::test]
async fn body_limits_and_non_json_never_become_records() {
    let (url, task) = raw_endpoint(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1024\r\nConnection: close\r\n\r\n").await;
    let config = BeRegistryConfig {
        max_response_bytes: 32,
        raw_debug: false,
        ..Default::default()
    };
    assert!(matches!(
        BeRegistryClient::for_test(url, config)
            .lookup("0012345678")
            .await,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                error: RegistryFailure::ResponseTooLarge,
                ..
            },
            ..
        })
    ));
    task.await.unwrap();
    let (url, task) = raw_endpoint(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
    assert!(matches!(
        BeRegistryClient::for_test(url, Default::default())
            .lookup("0012345678")
            .await,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                error: RegistryFailure::UnexpectedResponse,
                ..
            },
            ..
        })
    ));
    task.await.unwrap();
}
