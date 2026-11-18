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
    crate::test_support::assert_company_contract(&r);
    assert_eq!(r.status, Some(crate::CompanyStatus::Closed));
    assert!(r.vat_id.is_none() && r.vat_registered.is_none());
    let office = r.registered_office.as_ref().unwrap();
    assert_eq!(office.address.as_deref(), Some(" Original line "));
    assert_eq!(office.postcode.as_deref(), Some("A00 A000"));
    assert!(office.full_address_by_registry.is_none() && office.city.is_none());
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
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = crate::test_support::http::spawn_server(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        assert!(
            !crate::test_support::http::read_request(&mut socket)
                .await
                .unwrap()
                .is_empty()
        );
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

#[test]
fn lifecycle_maps_source_states_and_preserves_original_text() {
    for (raw, expected) in [
        (Some("Normal"), Some(crate::CompanyStatus::Active)),
        (Some("Dissolved"), Some(crate::CompanyStatus::Closed)),
        (Some("Struck Off"), Some(crate::CompanyStatus::Closed)),
        (Some("Liquidation"), Some(crate::CompanyStatus::Liquidation)),
        (
            Some("Strike Off Listed"),
            Some(crate::CompanyStatus::PendingClosure),
        ),
        (Some("Not Normal"), Some(crate::CompanyStatus::Unknown)),
        (None, None),
    ] {
        let mut value = source();
        *value
            .pointer_mut("/result/records/0/company_status")
            .unwrap() = json!(raw);
        let result: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&value).unwrap(),
            &IeBusinessId::parse("0123456").unwrap(),
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
fn status_codes_work_without_labels_and_conflicts_stay_unknown() {
    for (code, label, expected) in [
        (1151, None, crate::CompanyStatus::Active),
        (1158, None, crate::CompanyStatus::Closed),
        (1152, None, crate::CompanyStatus::Liquidation),
        (1158, Some("Normal"), crate::CompanyStatus::Unknown),
    ] {
        let mut raw = source();
        raw["result"]["records"][0]["company_status_code"] = json!(code);
        raw["result"]["records"][0]["company_status"] = json!(label);
        let v: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&raw).unwrap(),
            &IeBusinessId::parse("0123456").unwrap(),
        )
        .unwrap()
        .into();
        assert_eq!(v.status, Some(expected));
        crate::test_support::assert_company_contract(&v);
    }
}

#[test]
fn cro_address_lines_preserve_order_without_guessing_city_or_country() {
    let mut raw = source();
    raw["result"]["records"][0]["company_address_2"] = json!(" Second ");
    raw["result"]["records"][0]["company_address_3"] = json!("Third");
    let v: CompanyLookupResult = decode::decode(
        &serde_json::to_vec(&raw).unwrap(),
        &IeBusinessId::parse("0123456").unwrap(),
    )
    .unwrap()
    .into();
    let office = v.registered_office.unwrap();
    assert_eq!(office.address2.as_deref(), Some(" Second \nThird"));
    assert!(office.city.is_none() && office.country.is_none());
    for key in [
        "company_address_1",
        "company_address_2",
        "company_address_3",
        "company_address_4",
        "eircode",
    ] {
        raw["result"]["records"][0][key] = Value::Null;
    }
    let v: CompanyLookupResult = decode::decode(
        &serde_json::to_vec(&raw).unwrap(),
        &IeBusinessId::parse("0123456").unwrap(),
    )
    .unwrap()
    .into();
    assert!(v.registered_office.is_none());
}

#[test]
fn documented_company_output_matches_synthetic_source() {
    let body = source();

    let id = IeBusinessId::parse("0123456").unwrap();
    let native = decode::decode(&serde_json::to_vec(&body).unwrap(), &id).unwrap();
    crate::test_support::assert_documented_company("ie", native);
}
