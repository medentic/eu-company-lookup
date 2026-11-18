use super::*;
use crate::{CompanyLookupResult, LookupError, Provider, RegistryFailure};
use serde_json::{Value, json};
fn source() -> Value {
    json!({"status":"OK","data":[{"company_id":123,"reg_code":"00123456","name":"Synthetic OÜ","historical_names":["Previous OÜ"],"status":"R","legal_address":"Original address\nSecond line","zip_code":"00100","legal_form":"5","url":"https://ariregister.rik.ee/eng/company/00123456/Synthetic"}]})
}
#[test]
fn exact_match_preserves_original_fields_and_unknowns() {
    let id = EeBusinessId::parse(" 00123456 ").unwrap();
    let v: CompanyLookupResult = decode::decode(&serde_json::to_vec(&source()).unwrap(), &id)
        .unwrap()
        .into();
    crate::test_support::assert_company_contract(&v);
    assert_eq!(v.tax_id, "00123456");
    assert_eq!(v.source, Provider::EstonianRik);
    assert_eq!(v.status, Some(crate::CompanyStatus::Active));
    assert!(v.vat_id.is_none() && v.vat_registered.is_none());
    assert_eq!(v.legal_form.as_deref(), Some("Osaühing"));
    assert!(v.registry_url.is_some());
    let a = v.registered_office.unwrap();
    assert_eq!(
        a.full_address_by_registry.as_deref(),
        Some("Original address\nSecond line")
    );
    assert_eq!(a.postcode.as_deref(), Some("00100"));
    assert!(a.address.is_none() && a.city.is_none() && a.country.is_none());
}
#[test]
fn search_never_uses_first_hit_or_loses_leading_zeroes() {
    let id = EeBusinessId::parse("00123456").unwrap();
    let mut s = source();
    let mut other = s["data"][0].clone();
    other["reg_code"] = json!("99123456");
    s["data"].as_array_mut().unwrap().insert(0, other);
    assert!(decode::decode(&serde_json::to_vec(&s).unwrap(), &id).is_ok());
    let duplicate = s["data"][1].clone();
    s["data"].as_array_mut().unwrap().push(duplicate);
    assert!(decode::decode(&serde_json::to_vec(&s).unwrap(), &id).is_err());
    s = source();
    s["data"][0]["reg_code"] = json!(123456);
    assert!(decode::decode(&serde_json::to_vec(&s).unwrap(), &id).is_err());
    s["data"][0]["reg_code"] = json!(12345678);
    assert!(
        decode::decode(
            &serde_json::to_vec(&s).unwrap(),
            &EeBusinessId::parse("12345678").unwrap()
        )
        .is_ok()
    );
    for bad in ["EE00123456", "123456789", "1234567", "0012345６"] {
        assert!(EeBusinessId::parse(bad).is_err());
    }
    for s in [
        json!({"status":"OK","data":[]}),
        json!({"status":"ERROR","data":[]}),
    ] {
        assert!(decode::decode(&serde_json::to_vec(&s).unwrap(), &id).is_err());
    }
}
#[test]
fn missing_optional_address_stays_missing() {
    let mut s = source();
    s["data"][0]["legal_address"] = Value::Null;
    s["data"][0]["zip_code"] = Value::Null;
    s["data"][0]["status"] = Value::Null;
    let v: CompanyLookupResult = decode::decode(
        &serde_json::to_vec(&s).unwrap(),
        &EeBusinessId::parse("00123456").unwrap(),
    )
    .unwrap()
    .into();
    crate::test_support::assert_company_contract(&v);
    assert!(v.registered_office.is_none() && v.status.is_none());
}
#[tokio::test]
async fn anonymous_exact_query_reaches_shared_facade() {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let body = source().to_string();
    let server = crate::test_support::http::spawn_server(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let buffer = crate::test_support::http::read_request(&mut socket)
            .await
            .unwrap();
        let n = buffer.len();
        let request = String::from_utf8_lossy(&buffer[..n]).to_lowercase();
        assert!(request.starts_with("get /?q=00123456&results=10 "));
        assert!(!request.contains("authorization:") && !request.contains("cookie:"));
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
    });
    let native = EeRegistryClient::for_test(endpoint, Default::default());
    let client = crate::Client::with_test_estonian(native);
    let result = client.lookup_company("00123456", "EE").await.unwrap();
    assert_eq!(result.company_name, "Synthetic OÜ");
    server.await.unwrap();
}

#[tokio::test]
async fn invalid_configuration_and_identifier_fail_locally() {
    let config = EeRegistryConfig {
        timeout: std::time::Duration::ZERO,
        ..Default::default()
    };
    assert!(matches!(
        EeRegistryClient::with_config(config),
        Err(LookupError {
            kind: crate::model::LookupFailure::InvalidConfiguration(_),
            ..
        })
    ));
    assert!(matches!(
        EeRegistryClient::new().unwrap().lookup("invalid").await,
        Err(LookupError {
            kind: crate::model::LookupFailure::InvalidInput,
            ..
        })
    ));
}

#[tokio::test]
async fn endpoint_404_is_an_operational_error_not_registry_absence() {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = crate::test_support::http::spawn_server(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let buffer = crate::test_support::http::read_request(&mut socket)
            .await
            .unwrap();
        let n = buffer.len();
        assert!(n > 0);
        socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await.unwrap();
    });
    let result = EeRegistryClient::for_test(endpoint, Default::default())
        .lookup("00123456")
        .await;
    assert!(matches!(
        result,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                provider: Provider::EstonianRik,
                error: RegistryFailure::Http { status: 404 }
            },
            ..
        })
    ));
    server.await.unwrap();
}

#[test]
fn lifecycle_maps_source_states_and_preserves_original_text() {
    for (raw, expected) in [
        (Some("R"), Some(crate::CompanyStatus::Active)),
        (Some("K"), Some(crate::CompanyStatus::Closed)),
        (Some("L"), Some(crate::CompanyStatus::Liquidation)),
        (Some("N"), Some(crate::CompanyStatus::Bankruptcy)),
        (Some("R extra"), Some(crate::CompanyStatus::Unknown)),
        (None, None),
    ] {
        let mut value = source();
        *value.pointer_mut("/data/0/status").unwrap() = json!(raw);
        let result: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&value).unwrap(),
            &EeBusinessId::parse("00123456").unwrap(),
        )
        .unwrap()
        .into();
        crate::test_support::assert_company_contract(&result);
        assert_eq!(result.status, expected, "{raw:?}");
        let wire = serde_json::to_value(result).unwrap();
        assert_eq!(
            wire["registryDetails"]["data"]["nativeRegistryStatus"],
            json!(raw)
        );
    }
}

#[test]
fn registry_links_must_be_official_and_identity_bound() {
    for raw in [
        "https://example.org/eng/company/00123456",
        "https://ariregister.rik.ee/eng/company/99999999",
        "https://ariregister.rik.ee/eng/company/00123456?token=secret",
        "https://user@ariregister.rik.ee/eng/company/00123456",
        "http://ariregister.rik.ee/eng/company/00123456",
    ] {
        let mut value = source();
        value["data"][0]["url"] = json!(raw);
        let v: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&value).unwrap(),
            &EeBusinessId::parse("00123456").unwrap(),
        )
        .unwrap()
        .into();
        assert!(v.registry_url.is_none(), "{raw}");
    }
}

#[test]
fn documented_company_output_matches_synthetic_source() {
    let body = source();

    let id = EeBusinessId::parse("00123456").unwrap();
    let native = decode::decode(&serde_json::to_vec(&body).unwrap(), &id).unwrap();
    crate::test_support::assert_documented_company("ee", native);
}
