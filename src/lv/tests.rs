use super::*;
use crate::{CompanyLookupResult, LookupError, Provider, RegistryFailure};
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
    crate::test_support::assert_company_contract(&r);
    assert_eq!(r.tax_id, "00123456789");
    assert_eq!(r.status, None);
    let native = serde_json::to_value(&r.registry_details).unwrap();
    assert!(native["data"]["nativeRegistryStatus"].is_null());
    assert_eq!(native["data"]["closed"], " ");
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
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let mut base = source();
    base["result"]["records"][0]["registered"] = json!("2000-01-01");
    let mut responses = vec![(decode::RESOURCE, "regcode", base)];
    responses.extend(lifecycle::SOURCES.map(|(_, resource, field)| (resource, field, json!({"success":true,"result":{"resource_id":resource,"filters":{field:"00123456789"},"total":0,"records":[]}}))));
    let server = crate::test_support::http::spawn_server(async move {
        for (resource, field, body) in responses {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = crate::test_support::http::read_request(&mut socket)
                .await
                .unwrap();
            let request = String::from_utf8_lossy(&request);
            let path = request.split_whitespace().nth(1).unwrap();
            let url = reqwest::Url::parse(&format!("http://localhost{path}")).unwrap();
            let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
            assert_eq!(query["resource_id"], resource);
            assert_eq!(
                serde_json::from_str::<Value>(&query["filters"]).unwrap(),
                json!({field:"00123456789"})
            );
            assert!(
                !request.to_lowercase().contains("authorization:")
                    && !request.to_lowercase().contains("cookie:")
            );
            let body = body.to_string();
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).as_bytes()).await.unwrap();
        }
    });
    let native = LvRegistryClient::for_test(endpoint, Default::default());
    let client = crate::Client::with_test_latvian(native.clone());
    let result = client.lookup_company("00123456789", "LV").await.unwrap();
    assert_eq!(result.status, Some(crate::CompanyStatus::Active));
    let capture = native.take_capture();
    assert!(capture["lifecycle"]["insolvency"]["body"].is_string());
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
        Err(LookupError {
            kind: crate::model::LookupFailure::InvalidConfiguration(_),
            ..
        })
    ));
    assert!(matches!(
        LvRegistryClient::new().unwrap().lookup("invalid").await,
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
    let result = LvRegistryClient::for_test(endpoint, Default::default())
        .lookup("00123456789")
        .await;
    assert!(matches!(
        result,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                provider: Provider::LatvianOpenData,
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
        value.pointer_mut("/result/records/0").unwrap()["terminated"] = json!(date);
        let result: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&value).unwrap(),
            &LvBusinessId::parse("00123456789").unwrap(),
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
fn lifecycle_requires_complete_filtered_identity_bound_sources() {
    use crate::CompanyStatus::*;
    let id = LvBusinessId::parse("00123456789").unwrap();
    let empty: Vec<_>=lifecycle::SOURCES.iter().map(|(_,r,f)|json!({"success":true,"result":{"resource_id":r,"filters":{*f:id.as_str()},"total":0,"records":[]}})).collect();
    let decode_sources = |v: &[Value]| {
        lifecycle::decode(
            &serde_json::to_vec(&v[0]).unwrap(),
            &serde_json::to_vec(&v[1]).unwrap(),
            &serde_json::to_vec(&v[2]).unwrap(),
            &id,
        )
    };
    for (pointer, value) in [
        ("/success", json!(false)),
        ("/result/resource_id", json!("wrong")),
        ("/result/total", json!(1)),
        ("/result/filters", json!({})),
    ] {
        let mut values = empty.clone();
        *values[0].pointer_mut(pointer).unwrap() = value;
        assert!(decode_sources(&values).is_err());
    }
    let mut base = source();
    base["result"]["records"][0]["registered"] = json!("2000-01-01");
    for (entry, expected) in [
        (None, Active),
        (
            Some((
                0,
                json!({"id":1,"legal_entity_registration_number":id.as_str(),"liquidation_type":"LIQUIDATION","date_from":"2024-01-01T00:00:00"}),
            )),
            Liquidation,
        ),
        (
            Some((
                1,
                json!({"id":2,"legal_entity_registration_number":id.as_str(),"suspension_code":"ECONOMIC_ACTIVITY_SUSPENDED","date_from":"2024-01-01T00:00:00"}),
            )),
            Inactive,
        ),
        (
            Some((
                1,
                json!({"id":3,"legal_entity_registration_number":id.as_str(),"suspension_code":"NEW","date_from":"2024-01-01"}),
            )),
            Unknown,
        ),
        (
            Some((
                2,
                json!({"proceeding_id":4,"debtor_registration_number":id.as_str(),"proceeding_started_on":"2024-01-01"}),
            )),
            Unknown,
        ),
    ] {
        let mut responses = empty.clone();
        if let Some((index, entry)) = entry {
            responses[index]["result"]["total"] = json!(1);
            responses[index]["result"]["records"] = json!([entry]);
        }
        let mut native = decode::decode(&serde_json::to_vec(&base).unwrap(), &id).unwrap();
        native.details.lifecycle = Some(decode_sources(&responses).unwrap());
        assert_eq!(
            serde_json::from_slice::<LvCompanyDetails>(
                &serde_json::to_vec(&native.details).unwrap()
            )
            .unwrap(),
            native.details
        );
        let result: CompanyLookupResult = native.into();
        assert_eq!(result.status, Some(expected));
        crate::test_support::assert_company_contract(&result);
    }
    let mut wrong = empty;
    wrong[0]["result"]["total"] = json!(1);
    wrong[0]["result"]["records"] =
        json!([{"id":1,"legal_entity_registration_number":"00123456788"}]);
    assert!(decode_sources(&wrong).is_err());
}

#[tokio::test]
async fn missing_lifecycle_source_is_an_operational_failure() {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let body = source().to_string();
    let task = crate::test_support::http::spawn_server(async move {
        for reply in [
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            ),
            "HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            crate::test_support::http::read_request(&mut socket)
                .await
                .unwrap();
            socket.write_all(reply.as_bytes()).await.unwrap();
        }
    });
    let error = LvRegistryClient::for_test(endpoint, Default::default())
        .lookup("00123456789")
        .await
        .unwrap_err();
    assert!(matches!(
        error.kind,
        crate::model::LookupFailure::Registry {
            error: RegistryFailure::ServiceUnavailable {
                reason: crate::UnavailabilityReason::ServerError,
                ..
            },
            ..
        }
    ));
    task.await.unwrap();
}

#[test]
fn documented_company_output_matches_synthetic_source() {
    let mut body = source();
    body["result"]["records"][0]["terminated"] = serde_json::json!("2020-01-01");
    let id = LvBusinessId::parse("00123456789").unwrap();
    let native = decode::decode(&serde_json::to_vec(&body).unwrap(), &id).unwrap();
    crate::test_support::assert_documented_company("lv", native);
}

#[tokio::test]
async fn documented_unresolved_error_matches_complete_supplement_flow() {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let base = source();
    let mut responses = vec![(decode::RESOURCE, "regcode", base)];
    responses.extend(lifecycle::SOURCES.map(|(_, resource, field)| (resource, field, json!({"success":true,"result":{"resource_id":resource,"filters":{field:"00123456789"},"total":0,"records":[]}}))));
    let server = crate::test_support::http::spawn_server(async move {
        for (resource, field, body) in responses {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = crate::test_support::http::read_request(&mut socket)
                .await
                .unwrap();
            let request = String::from_utf8_lossy(&request);
            let path = request.split_whitespace().nth(1).unwrap();
            let url = reqwest::Url::parse(&format!("http://localhost{path}")).unwrap();
            let query: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
            assert_eq!(query["resource_id"], resource);
            assert_eq!(
                serde_json::from_str::<Value>(&query["filters"]).unwrap(),
                json!({field:"00123456789"})
            );
            assert!(
                !request.to_lowercase().contains("authorization:")
                    && !request.to_lowercase().contains("cookie:")
            );
            let body = body.to_string();
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).as_bytes()).await.unwrap();
        }
    });
    let native = LvRegistryClient::for_test(endpoint, Default::default());
    let client = crate::Client::with_test_latvian(native.clone());
    let error = client
        .lookup_company("00123456789", "LV")
        .await
        .unwrap_err();
    assert_eq!(
        serde_json::to_value(error).unwrap(),
        crate::test_support::documentation::example("README", 0)
    );
    let capture = native.take_capture();
    assert!(capture["lifecycle"]["insolvency"]["body"].is_string());
    server.await.unwrap();
}
