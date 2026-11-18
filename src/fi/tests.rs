use super::*;
use crate::{CompanyLookupResult, LookupError, Provider, RegistryFailure};
use serde_json::{Value, json};

fn source() -> Value {
    json!({"totalResults":1,"companies":[{
        "businessId":{"value":"0012345-6","source":"3"},
        "names":[{"name":"Synthetic Oy","type":"1","version":1,"source":"1","endDate":"2020-01-01"}],
        "companyForms":[{"type":"16","version":1,"source":"1","descriptions":[{"languageCode":"1","description":"Osakeyhtiö"}]}],
        "tradeRegisterStatus":"4", "registeredEntries":[],
        "addresses":[{"type":2,"source":"1","postCode":"00100","freeAddressLine":"ORIGINAL_text"}]
    }]})
}

#[test]
fn source_identity_name_history_and_missing_timestamp_are_preserved() {
    let id = FiBusinessId::parse(" 0012345-6 ").unwrap();
    let result = decode::decode(&serde_json::to_vec(&source()).unwrap(), &id).unwrap();
    assert_eq!(result.company_name, "Synthetic Oy");
    assert!(result.details.last_modified.is_none());
    assert_eq!(
        result.details.names[0].end_date.as_deref(),
        Some("2020-01-01")
    );
    let result: CompanyLookupResult = result.into();
    crate::test_support::assert_company_contract(&result);
    assert_eq!(result.status, Some(crate::CompanyStatus::Closed));
    assert_eq!(result.source, Provider::FinnishPrh);
    assert_eq!(result.tax_id, "0012345-6");
    assert_eq!(result.legal_form.as_deref(), Some("Osakeyhtiö"));
    assert!(result.vat_id.is_none() && result.vat_registered.is_none());
    assert!(result.registered_office.is_none() && result.fiscal_address.is_none());
    let json = serde_json::to_value(result).unwrap();
    assert_eq!(
        json["registryDetails"]["data"]["addresses"][0]["freeAddressLine"],
        "ORIGINAL_text"
    );
}

#[test]
fn ambiguous_names_wrong_ids_and_empty_results_are_not_records() {
    let id = FiBusinessId::parse("0012345-6").unwrap();
    let mut value = source();
    value["companies"][0]["businessId"]["value"] = json!("0012345-7");
    assert!(matches!(
        decode::decode(&serde_json::to_vec(&value).unwrap(), &id),
        Err(RegistryFailure::MismatchedResponse)
    ));
    value = source();
    let duplicate = value["companies"][0]["names"][0].clone();
    value["companies"][0]["names"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(decode::decode(&serde_json::to_vec(&value).unwrap(), &id).is_err());
    assert!(matches!(
        decode::decode(br#"{"totalResults":0,"companies":[]}"#, &id),
        Err(RegistryFailure::UnexpectedResponse)
    ));
    for bad in [
        "FI00123456",
        "00123456",
        "123456-7",
        "0012345–6",
        "0012345-６",
    ] {
        assert!(FiBusinessId::parse(bad).is_err());
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
        assert!(request.starts_with("get /?businessid=0012345-6 "));
        assert!(!request.contains("authorization:") && !request.contains("cookie:"));
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
    });
    let native = FiRegistryClient::for_test(endpoint, Default::default());
    let client = crate::Client::with_test_finnish(native);
    let result = client.lookup_company("0012345-6", "FI").await.unwrap();
    assert_eq!(result.company_name, "Synthetic Oy");
    server.await.unwrap();
}

#[tokio::test]
async fn invalid_configuration_and_identifier_fail_locally() {
    let config = FiRegistryConfig {
        timeout: std::time::Duration::ZERO,
        ..Default::default()
    };
    assert!(matches!(
        FiRegistryClient::with_config(config),
        Err(LookupError {
            kind: crate::model::LookupFailure::InvalidConfiguration(_),
            ..
        })
    ));
    assert!(matches!(
        FiRegistryClient::new().unwrap().lookup("invalid").await,
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
    let result = FiRegistryClient::for_test(endpoint, Default::default())
        .lookup("0012345-6")
        .await;
    assert!(matches!(
        result,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                provider: Provider::FinnishPrh,
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
        (Some("1"), Some(crate::CompanyStatus::Active)),
        (Some("4"), Some(crate::CompanyStatus::Closed)),
        (Some("2"), Some(crate::CompanyStatus::Unknown)),
        (Some("0"), Some(crate::CompanyStatus::Unknown)),
        (Some("3"), Some(crate::CompanyStatus::Unknown)),
        (Some("999"), Some(crate::CompanyStatus::Unknown)),
    ] {
        let mut value = source();
        *value
            .pointer_mut("/companies/0/tradeRegisterStatus")
            .unwrap() = json!(raw);
        let result: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&value).unwrap(),
            &FiBusinessId::parse("0012345-6").unwrap(),
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
fn registration_does_not_hide_dissolution_or_ongoing_proceedings() {
    for (field, value) in [
        ("endDate", json!("2020-01-01")),
        (
            "companySituations",
            json!([{"type":"unrecognized","endDate":null}]),
        ),
    ] {
        let mut raw = source();
        raw["companies"][0]["tradeRegisterStatus"] = json!("1");
        raw["companies"][0][field] = value;
        let v: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&raw).unwrap(),
            &FiBusinessId::parse("0012345-6").unwrap(),
        )
        .unwrap()
        .into();
        assert_eq!(v.status, Some(crate::CompanyStatus::Unknown));
    }
}

#[test]
fn current_proceedings_map_but_ended_and_conflicting_situations_do_not_override() {
    use crate::CompanyStatus::*;
    for (situations, expected) in [
        (json!([{"type":"SANE","endDate":null}]), Restructuring),
        (json!([{"type":"SELTILA","endDate":null}]), Liquidation),
        (json!([{"type":"KONK","endDate":null}]), Bankruptcy),
        (json!([{"type":"KONK","endDate":"2020-01-01"}]), Active),
        (json!([{"type":"KONK","endDate":"invalid"}]), Unknown),
        (
            json!([{"type":"KONK","endDate":null},{"type":"SELTILA","endDate":null}]),
            Unknown,
        ),
        (json!([{"type":"future","endDate":null}]), Unknown),
    ] {
        let mut raw = source();
        raw["companies"][0]["tradeRegisterStatus"] = json!("1");
        raw["companies"][0]["companySituations"] = situations;
        let common: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&raw).unwrap(),
            &FiBusinessId::parse("0012345-6").unwrap(),
        )
        .unwrap()
        .into();
        assert_eq!(common.status, Some(expected));
    }
}

#[test]
fn documented_company_output_matches_synthetic_source() {
    let body = source();

    let id = FiBusinessId::parse("0012345-6").unwrap();
    let native = decode::decode(&serde_json::to_vec(&body).unwrap(), &id).unwrap();
    crate::test_support::assert_documented_company("fi", native);
}
