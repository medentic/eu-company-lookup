use super::*;
use crate::{LookupError, RegistryFailure};
use tokio::io::AsyncWriteExt;

#[test]
fn shared_result_uses_official_address_and_registered_latin_name() {
    let id = GrBusinessId::parse("001234567890").unwrap();
    let raw = br#"{"companyInfo":{"payload":{"company":{"id":"001234567890","name":"Source","namei18n":"Registered Latin Name","afm":"012345678","company_address":" Official ","company_address_map":"Map"}}}}"#;
    let common: crate::CompanyLookupResult = decode::decode(raw, &id).unwrap().into();
    crate::test_support::assert_company_contract(&common);
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
    let task = crate::test_support::http::spawn_server(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let bytes = crate::test_support::http::read_request(&mut socket)
            .await
            .unwrap();
        let split = bytes.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let headers = String::from_utf8_lossy(&bytes[..split]);
        assert!(headers.starts_with("POST / HTTP/1.1"));
        assert!(!headers.to_ascii_lowercase().contains("authorization:"));
        let body: serde_json::Value = serde_json::from_slice(&bytes[split + 4..]).unwrap();
        assert_eq!(
            body,
            serde_json::json!({"query":{"arGEMI":"001234567890"},"token":null,"language":"el"})
        );
        let body = r#"{"companyInfo":{"errorCode":null,"errorMessage":null,"payload":{"company":{"id":"001234567890","name":"Example"}}}}"#;
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
    });
    let client = GrRegistryClient::for_test(url, Default::default());
    client.begin_capture();
    assert_eq!(
        client.lookup("001234567890").await.unwrap_err().id,
        crate::ErrorId::UnresolvedData
    );
    assert!(client.take_capture()["body"].as_str().is_some());
    task.await.unwrap();
}
async fn raw_endpoint(reply: &'static [u8]) -> (String, crate::test_support::http::ServerTask<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = crate::test_support::http::spawn_server(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        assert!(
            !crate::test_support::http::read_request(&mut socket)
                .await
                .unwrap()
                .is_empty()
        );
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
        clone.lookup("001234567890").await,
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
    let config = GrRegistryConfig {
        max_response_bytes: 32,
        raw_debug: false,
        ..Default::default()
    };
    assert!(matches!(
        GrRegistryClient::for_test(url, config)
            .lookup("001234567890")
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
        GrRegistryClient::for_test(url, Default::default())
            .lookup("001234567890")
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

#[test]
fn absent_and_unmapped_statuses_have_distinct_common_meanings() {
    let id = GrBusinessId::parse("001234567890").unwrap();
    for status in [
        None,
        Some("Active source label"),
        Some("Future source status"),
    ] {
        let raw = serde_json::json!({"companyInfo":{"payload":{"company":{
            "id":id.as_str(), "name":"Example", "companyStatus":{"status":status}
        }}}});
        let common: crate::CompanyLookupResult =
            decode::decode(&serde_json::to_vec(&raw).unwrap(), &id)
                .unwrap()
                .into();
        crate::test_support::assert_company_contract(&common);
        assert_eq!(common.status, status.map(|_| crate::CompanyStatus::Unknown));
        assert_eq!(
            serde_json::to_value(common).unwrap()["registryDetails"]["data"]["nativeRegistryStatus"],
            serde_json::json!(status)
        );
    }
}

#[test]
fn greek_labels_are_mapped_exactly_without_translating_source_text() {
    for (label, expected) in [
        ("ΕΝΕΡΓΗ", crate::CompanyStatus::Active),
        ("Ενεργή", crate::CompanyStatus::Active),
        ("Διαγραφή", crate::CompanyStatus::Closed),
        ("Λύση - Εκκαθάριση", crate::CompanyStatus::Liquidation),
        ("ΛΥΣΗ-ΕΚΚΑΘΑΡΙΣΗ", crate::CompanyStatus::Liquidation),
        ("Πτώχευση", crate::CompanyStatus::Bankruptcy),
        ("Υπό Εκκαθάριση", crate::CompanyStatus::Liquidation),
        ("ΜΗ ΕΝΕΡΓΗ", crate::CompanyStatus::Unknown),
    ] {
        let raw = serde_json::json!({"companyInfo":{"payload":{"company":{"id":"001234567890","name":"Synthetic","companyStatus":{"status":label}}}}});
        let v: crate::CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&raw).unwrap(),
            &GrBusinessId::parse("001234567890").unwrap(),
        )
        .unwrap()
        .into();
        assert_eq!(v.status, Some(expected), "{label}");
        crate::test_support::assert_company_contract(&v);
        assert_eq!(
            serde_json::to_value(v).unwrap()["registryDetails"]["data"]["nativeRegistryStatus"],
            label
        );
    }
}

#[test]
fn documented_company_output_matches_synthetic_source() {
    let raw = r#"{"companyInfo":{"payload":{"company":{"id":"001234567890","name":"Source","namei18n":"Registered Latin Name","afm":"012345678","company_address":" Official ","company_address_map":"Map","companyStatus":{"status":"ΕΝΕΡΓΗ"}}}}}"#;
    let id = GrBusinessId::parse("001234567890").unwrap();
    let native = decode::decode(raw.as_bytes(), &id).unwrap();
    crate::test_support::assert_documented_company("gr", native);
}
