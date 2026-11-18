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
    crate::test_support::assert_company_contract(&r);
    assert!(
        r.registered_office.is_some()
            && r.status == Some(crate::CompanyStatus::Closed)
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

#[test]
fn only_explicit_valid_removal_dates_establish_closure() {
    for (date, closed) in [
        (Some("2020-01-01"), true),
        (Some("2020-01-01T00:00:00"), true),
        (Some("2020-99-99"), false),
        (Some("2020-1-1"), false),
        (Some("2020-01-01T0:00:00"), false),
        (Some(""), false),
        (None, false),
    ] {
        let mut value = source();
        value.pointer_mut("/_data/0").unwrap()["isreg_data"] = json!(date);
        let result: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&value).unwrap(),
            &LtBusinessId::parse("001234567").unwrap(),
        )
        .unwrap()
        .into();
        assert_eq!(result.status == Some(crate::CompanyStatus::Closed), closed);
        if date.is_some() && !closed {
            assert_eq!(result.status, Some(crate::CompanyStatus::Unknown));
        }
        assert_ne!(result.status, Some(crate::CompanyStatus::Active));
        crate::test_support::assert_company_contract(&result);
    }
}

#[test]
fn registered_address_is_original_text_and_missing_text_stays_null() {
    for address in [Some("  Original\nSecond line  "), Some(" "), None] {
        let mut raw = source();
        raw["_data"][0]["pilnas_adresas"] = json!(address);
        let v: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&raw).unwrap(),
            &LtBusinessId::parse("001234567").unwrap(),
        )
        .unwrap()
        .into();
        if address.is_some_and(|s| !s.trim().is_empty()) {
            let office = v.registered_office.unwrap();
            assert_eq!(office.full_address_by_registry.as_deref(), address);
            assert_eq!(office.city, None);
        } else {
            assert!(v.registered_office.is_none());
        }
    }
}

#[test]
fn official_status_references_are_not_guessed_from_uuid_shape() {
    use crate::CompanyStatus::*;
    for (reference, expected) in [
        ("5ef6b364-a5ff-47fb-8600-ff859214ef85", Active),
        ("04aca49f-d1f9-47f8-af8a-5800eae51e6b", Restructuring),
        ("20a01d01-4e39-4d14-82f3-a9af198de63b", Bankruptcy),
        ("adb14ebc-d6c5-4534-8dd9-b3d14e92b19f", Liquidation),
        ("ff75611d-3e1b-491b-8c85-f2ba085815fe", PendingClosure),
        ("5bcfd61f-7810-4946-9bd3-6de946b56f18", Closed),
        ("28797208-2fa6-47d3-80e4-e4e8842b44c5", Unknown),
    ] {
        assert_eq!(super::statuses::lookup(reference).unwrap().1, expected);
    }
    assert!(super::statuses::lookup("unrecognized-reference").is_none());
}

#[test]
fn mapped_reference_resolves_common_status_but_bad_date_blocks_success() {
    for (date, expected) in [
        (Value::Null, crate::CompanyStatus::Active),
        (json!("2024-02-30"), crate::CompanyStatus::Unknown),
        (json!("2020-01-01"), crate::CompanyStatus::Closed),
    ] {
        let mut body = source();
        body["_data"][0]["isreg_data"] = date;
        body["_data"][0]["statusas"]["_id"] = json!("5ef6b364-a5ff-47fb-8600-ff859214ef85");
        let result: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&body).unwrap(),
            &LtBusinessId::parse("001234567").unwrap(),
        )
        .unwrap()
        .into();
        assert_eq!(result.status, Some(expected));
        crate::test_support::assert_company_contract(&result);
    }
}

#[test]
fn unfamiliar_status_reference_is_supplied_unmapped_evidence() {
    let mut body = source();
    body["_data"][0]["isreg_data"] = json!(null);
    let common: CompanyLookupResult = decode::decode(
        &serde_json::to_vec(&body).unwrap(),
        &LtBusinessId::parse("001234567").unwrap(),
    )
    .unwrap()
    .into();
    assert_eq!(common.status, Some(crate::CompanyStatus::Unknown));
    let error = crate::model::company_validation::ensure_resolved(common).unwrap_err();
    assert_eq!(
        error.details.native_registry_status.as_deref(),
        Some("opaque-status")
    );
    assert_eq!(
        error.details.reason,
        Some("unmapped_or_ambiguous_lifecycle")
    );
}

#[test]
fn documented_company_output_matches_synthetic_source() {
    let body = source();

    let id = LtBusinessId::parse("001234567").unwrap();
    let native = decode::decode(&serde_json::to_vec(&body).unwrap(), &id).unwrap();
    crate::test_support::assert_documented_company("lt", native);
}
