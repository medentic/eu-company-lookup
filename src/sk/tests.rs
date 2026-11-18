use super::*;
use crate::{CompanyLookupResult, LookupError, Provider, RegistryFailure};
use serde_json::{Value, json};
fn source() -> Value {
    json!({"id":42,"ico":"00123456","nazovUJ":"Synthetic s.r.o.","dic":"0012345678","pravnaForma":"112","ulica":" Source street ","mesto":"Town","psc":"00100"})
}
#[test]
fn unique_search_and_exact_detail_identity_are_required() {
    assert_eq!(
        decode::select(br#"{"id":[42],"existujeDalsieId":false}"#).unwrap(),
        42
    );
    for raw in [
        r#"{"id":[],"existujeDalsieId":false}"#,
        r#"{"id":[42,43],"existujeDalsieId":false}"#,
        r#"{"id":[42],"existujeDalsieId":true}"#,
        r#"{"id":[42]}"#,
    ] {
        assert!(decode::select(raw.as_bytes()).is_err());
    }
    let id = SkBusinessId::parse("00123456").unwrap();
    for (key, value) in [
        ("id", json!(43)),
        ("ico", json!("00123457")),
        ("nazovUJ", json!(" ")),
        ("stav", json!("ZMAZANÉ")),
    ] {
        let mut s = source();
        s[key] = value;
        assert!(decode::decode(&serde_json::to_vec(&s).unwrap(), &id, 42).is_err());
    }
    assert!(decode::decode(r#"{"id":42,"stav":"ZMAZANÉ"}"#.as_bytes(), &id, 42).is_err());
}
#[test]
fn source_roles_and_leading_zeroes_are_preserved() {
    let id = SkBusinessId::parse(" 0012 3456 ").unwrap();
    let r: CompanyLookupResult = decode::decode(&serde_json::to_vec(&source()).unwrap(), &id, 42)
        .unwrap()
        .into();
    crate::test_support::assert_company_contract(&r);
    assert_eq!(r.tax_id, "00123456");
    assert!(
        r.registered_office.is_none()
            && r.fiscal_address.is_none()
            && r.vat_id.is_none()
            && r.vat_registered.is_none()
            && r.status.is_none()
    );
    assert_eq!(r.legal_form.as_deref(), Some("Spol. s r. o."));
    let j = serde_json::to_value(r).unwrap();
    assert_eq!(j["registryDetails"]["data"]["dic"], "0012345678");
    assert_eq!(j["registryDetails"]["data"]["ulica"], " Source street ");
    for bad in ["SK00123456", "0012345678", "0012345６"] {
        assert!(SkBusinessId::parse(bad).is_err());
    }
    let minimal = json!({"id":42,"ico":"00123456","nazovUJ":"Minimal"});
    assert!(decode::decode(&serde_json::to_vec(&minimal).unwrap(), &id, 42).is_ok());
}
#[tokio::test]
async fn four_anonymous_requests_reach_shared_facade() {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = crate::test_support::http::spawn_server(async move {
        for (path, body) in [
            (
                "/uctovne-jednotky?zmenene-od=2000-01-01&ico=00123456&max-zaznamov=2",
                json!({"id":[42],"existujeDalsieId":false}).to_string(),
            ),
            ("/uctovna-jednotka?id=42", source().to_string()),
            ("/rpo/search?identifier=00123456&onlyActive=true", json!({"results":[{"id":99,"identifiers":[{"value":"00123456"}],"establishment":"2000-01-01"}]}).to_string()),
            ("/rpo/entity/99?showHistoricalData=false&showOrganizationUnits=false", json!({"id":99,"identifiers":[{"value":"00123456"}],"establishment":"2000-01-01"}).to_string()),
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let buf = crate::test_support::http::read_request(&mut socket)
                .await
                .unwrap();
            let n = buf.len();
            let req = String::from_utf8_lossy(&buf[..n]).to_lowercase();
            assert!(req.starts_with(&format!("get {} ",path.to_lowercase())));
            assert!(!req.contains("authorization:") && !req.contains("cookie:"));
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).as_bytes()).await.unwrap();
        }
    });
    let client =
        crate::Client::with_test_slovak(SkRegistryClient::for_test(endpoint, Default::default()));
    assert_eq!(
        client
            .lookup_company("00123456", "SK")
            .await
            .unwrap()
            .status,
        Some(crate::CompanyStatus::Active)
    );
    server.await.unwrap();
}
#[tokio::test]
async fn source_failure_never_proves_absence() {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = crate::test_support::http::spawn_server(async move {
        let (mut s, _) = listener.accept().await.unwrap();
        assert!(
            !crate::test_support::http::read_request(&mut s)
                .await
                .unwrap()
                .is_empty()
        );
        s.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
    });
    assert!(matches!(
        SkRegistryClient::for_test(endpoint, Default::default())
            .lookup("00123456")
            .await,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                provider: Provider::SlovakRuz,
                error: RegistryFailure::Http { status: 404 }
            },
            ..
        })
    ));
    server.await.unwrap();
    assert!(matches!(
        SkRegistryClient::new().unwrap().lookup("invalid").await,
        Err(LookupError {
            kind: crate::model::LookupFailure::InvalidInput,
            ..
        })
    ));
}

#[test]
fn unknown_legal_form_codes_do_not_become_common_labels() {
    assert_eq!(super::legal_forms::label("999999"), None);
    assert_eq!(super::legal_forms::label(""), None);
}

#[test]
fn dissolution_date_is_distinct_from_api_record_availability() {
    use crate::CompanyStatus::*;
    for (date, expected) in [
        (json!("2024-02-29"), Some(Closed)),
        (json!("2023-02-29"), Some(Unknown)),
        (json!("2024-2-9"), Some(Unknown)),
        (json!(""), Some(Unknown)),
        (Value::Null, None),
    ] {
        let mut body = source();
        body["datumZrusenia"] = date;
        let result: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&body).unwrap(),
            &SkBusinessId::parse("00123456").unwrap(),
            42,
        )
        .unwrap()
        .into();
        assert_eq!(result.status, expected);
        crate::test_support::assert_company_contract(&result);
    }
}

#[test]
fn rpo_supplement_requires_current_identity_and_resolves_native_lifecycle() {
    let id = SkBusinessId::parse("00123456").unwrap();
    let base = json!({"id":99,"identifiers":[{"value":"00123456","validFrom":"2000-01-01"}],"establishment":"2000-01-01"});
    let search = |v: serde_json::Value| serde_json::to_vec(&json!({"results":[v]})).unwrap();
    assert_eq!(rpo::select(&search(base.clone()), &id).unwrap(), Some(99));
    let decode = |v: &serde_json::Value| rpo::decode(&serde_json::to_vec(v).unwrap(), &id, 99);
    assert_eq!(
        rpo::status(&decode(&base).unwrap()),
        crate::CompanyStatus::Active
    );
    for (label, expected) in [
        ("V likvidácii", crate::CompanyStatus::Liquidation),
        ("V konkurze", crate::CompanyStatus::Bankruptcy),
        ("V reštrukturalizácii", crate::CompanyStatus::Restructuring),
        ("New state", crate::CompanyStatus::Unknown),
    ] {
        let mut v = base.clone();
        v["legalStatuses"] =
            json!([{"value":{"value":label,"codelistCode":"CL010108"},"validFrom":"2020-01-01"}]);
        assert_eq!(rpo::status(&decode(&v).unwrap()), expected);
        let mut native = decode::decode(&serde_json::to_vec(&source()).unwrap(), &id, 42).unwrap();
        native.details.rpo_lifecycle = Some(decode(&v).unwrap());
        let common: crate::CompanyLookupResult = native.into();
        assert_eq!(common.registry_details.native_status(), Some(label));
        assert_eq!(common.status, Some(expected));
        let details = decode(&v).unwrap();
        assert_eq!(
            serde_json::from_slice::<SkRpoLifecycle>(&serde_json::to_vec(&details).unwrap())
                .unwrap(),
            details
        );
    }
    for (pointer, value) in [
        ("/id", json!(98)),
        ("/identifiers/0/value", json!("87654321")),
        ("/termination", json!("2020-01-01")),
    ] {
        let mut v = base.clone();
        if pointer == "/termination" {
            v["termination"] = value;
        } else {
            *v.pointer_mut(pointer).unwrap() = value;
        }
        assert!(decode(&v).is_err());
    }
    let mut v = base.clone();
    v["establishment"] = json!("invalid");
    assert_eq!(
        rpo::status(&decode(&v).unwrap()),
        crate::CompanyStatus::Unknown
    );
    assert_eq!(rpo::select(br#"{"results":[]}"#, &id).unwrap(), None);
    let mut v = base;
    v["identifiers"][0]["validTo"] = json!("2020-01-01");
    assert!(rpo::select(&search(v), &id).is_err());
}

#[tokio::test]
async fn missing_rpo_match_and_access_failure_remain_distinct() {
    use tokio::io::AsyncWriteExt;
    for rpo_http_status in [200, 503] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let task = crate::test_support::http::spawn_server(async move {
            for (status, body) in [
                (200, json!({"id":[42],"existujeDalsieId":false})),
                (200, source()),
                (rpo_http_status, json!({"results":[]})),
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                crate::test_support::http::read_request(&mut socket)
                    .await
                    .unwrap();
                let body = body.to_string();
                socket.write_all(format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).as_bytes()).await.unwrap();
            }
        });
        let error = SkRegistryClient::for_test(endpoint, Default::default())
            .lookup("00123456")
            .await
            .unwrap_err();
        if rpo_http_status == 200 {
            assert!(matches!(
                error.kind,
                crate::model::LookupFailure::UnresolvedData {
                    reason: "missing_lifecycle",
                    ..
                }
            ));
            assert!(!error.is_service_unavailable);
        } else {
            assert!(matches!(
                error.kind,
                crate::model::LookupFailure::Registry {
                    error: RegistryFailure::ServiceUnavailable { .. },
                    ..
                }
            ));
            assert!(error.is_service_unavailable);
            assert_eq!(error.details.upstream_http_status, Some(503));
        }
        task.await.unwrap();
    }
}

#[test]
fn documented_company_output_matches_synthetic_source() {
    let mut body = source();
    body["datumZrusenia"] = serde_json::json!("2020-01-01");
    let id = SkBusinessId::parse("00123456").unwrap();
    let native = decode::decode(&serde_json::to_vec(&body).unwrap(), &id, 42).unwrap();
    crate::test_support::assert_documented_company("sk", native);
}
