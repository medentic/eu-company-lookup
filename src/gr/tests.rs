use super::*;
use crate::{LookupError, RegistryError};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn shared_result_uses_official_address_and_registered_latin_name() {
    let id = GrBusinessId::parse("001234567890").unwrap();
    let raw = br#"{"companyInfo":{"payload":{"company":{"id":"001234567890","name":"Source","namei18n":"Registered Latin Name","afm":"012345678","company_address":" Official ","company_address_map":"Map"}}}}"#;
    let common: crate::CompanyLookupResult = decode::decode(raw, &id).unwrap().into();
    assert_eq!(
        common.foreign_name.as_deref(),
        Some("Registered Latin Name")
    );
    assert_eq!(
        common.registry_url.as_deref(),
        Some("https://publicity.businessportal.gr/company/001234567890")
    );
    let a = common.registered_office.unwrap();
    assert_eq!(a.full_address_by_registry.as_deref(), Some(" Official "));
    assert!(a.address.is_none() && a.country_iso2.is_none());
    assert!(common.vat_id.is_none() && common.vat_registered.is_none());
}

#[test]
fn gemi_identity_and_tax_number_are_distinct() {
    let id = GrBusinessId::parse("001234567890").unwrap();
    assert!(GrBusinessId::parse("123456789").is_err());
    assert!(GrBusinessId::parse("EL123456789").is_err());
    let raw = br#"{"companyInfo":{"errorCode":null,"errorMessage":null,"payload":{"company":{"id":"001234567890","name":"Source","afm":"012345678","company_address":" Original ","company_address_map":"Map","isABranch":false,"companyStatus":{"id":"3","status":"Native"}}}}}"#;
    let v = decode::decode(raw, &id).unwrap();
    assert_eq!(v.tax_id.as_str(), "001234567890");
    assert_eq!(v.details.tax_identifier.as_deref(), Some("012345678"));
    assert_eq!(v.full_address_by_registry.as_deref(), Some(" Original "));
    assert_eq!(v.details.map_address.as_deref(), Some("Map"));
    assert_eq!(v.details.is_branch, Some(false));
    assert!(decode::decode(raw, &GrBusinessId::parse("001234567891").unwrap()).is_err());
    for raw in [
        b"null".as_slice(),
        br#"{"companyInfo":{"errorCode":"FAILURE","payload":null}}"#.as_slice(),
    ] {
        assert!(decode::decode(raw, &id).is_err());
    }
}

#[tokio::test]
async fn post_uses_public_null_token_contract() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut buf = [0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&buf[..n]);
            if let Some(split) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..split]);
                let length: usize = headers
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .map(str::to_owned)
                    })
                    .unwrap()
                    .parse()
                    .unwrap();
                if bytes.len() < split + 4 + length {
                    continue;
                }
                assert!(headers.starts_with("POST / HTTP/1.1"));
                assert!(!headers.to_ascii_lowercase().contains("authorization:"));
                let body: serde_json::Value = serde_json::from_slice(&bytes[split + 4..]).unwrap();
                assert_eq!(
                    body,
                    serde_json::json!({"query":{"arGEMI":"001234567890"},"token":null,"language":"el"})
                );
                break;
            }
        }
        let body = r#"{"companyInfo":{"errorCode":null,"errorMessage":null,"payload":{"company":{"id":"001234567890","name":"Example"}}}}"#;
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
    });
    let client = GrRegistryClient::for_test(url, Default::default());
    client.begin_capture();
    assert_eq!(
        client.lookup("001234567890").await.unwrap().company_name,
        "Example"
    );
    assert!(client.take_capture()["body"].as_str().is_some());
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
    let client = GrRegistryClient::for_test(url, Default::default());
    let clone = client.clone();
    assert!(matches!(
        client.lookup("001234567890").await,
        Err(LookupError::Registry {
            error: RegistryError::RateLimited {
                retry_after: Some(crate::RetryAfter::Seconds(120))
            },
            ..
        })
    ));
    task.await.unwrap();
    assert!(matches!(
        clone.lookup("001234567890").await,
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
    let config = GrRegistryConfig {
        max_response_bytes: 32,
        ..Default::default()
    };
    assert!(matches!(
        GrRegistryClient::for_test(url, config)
            .lookup("001234567890")
            .await,
        Err(LookupError::Registry {
            error: RegistryError::ResponseTooLarge,
            ..
        })
    ));
    task.await.unwrap();
    let (url, task) = raw_endpoint(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
    assert!(matches!(
        GrRegistryClient::for_test(url, Default::default())
            .lookup("001234567890")
            .await,
        Err(LookupError::Registry {
            error: RegistryError::UnexpectedResponse,
            ..
        })
    ));
    task.await.unwrap();
}
