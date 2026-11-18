use super::*;
use crate::{LookupError, Provider, RegistryFailure};
use tokio::io::AsyncWriteExt;

#[test]
fn exact_identity_and_required_name() {
    let id = CzBusinessId::parse("00123456").unwrap();
    for raw in [
        r#"{"ico":"00123457","obchodniJmeno":"Example"}"#,
        r#"{"ico":"00123456","obchodniJmeno":" "}"#,
        r#"{"ico":123456,"obchodniJmeno":"Example"}"#,
    ] {
        assert!(decode::decode(raw.as_bytes(), &id).is_err());
    }
    let raw = br#"{"ico":"00123456","obchodniJmeno":"Example","dic":"CZ00123456","sidlo":{"textovaAdresa":"  Source address  ","pscTxt":"00123"},"seznamRegistraci":{"stavZdrojeDph":"AKTIVNI"}}"#;
    let result = decode::decode(raw, &id).unwrap().unwrap();
    assert_eq!(
        result.registered_office.unwrap().textova_adresa.as_deref(),
        Some("  Source address  ")
    );
    assert_eq!(result.tax_identifier.as_deref(), Some("CZ00123456"));
}

async fn endpoint(
    status: u16,
    body: &'static str,
) -> (String, crate::test_support::http::ServerTask<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = crate::test_support::http::spawn_server(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let bytes = crate::test_support::http::read_request(&mut socket)
            .await
            .unwrap();
        let count = bytes.len();
        let response = format!(
            "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.write_all(response.as_bytes()).await.unwrap();
        String::from_utf8(bytes[..count].to_vec()).unwrap()
    });
    (format!("http://{address}"), task)
}

#[tokio::test]
async fn get_preserves_identifier_and_http_failure_is_not_absence() {
    let (url, task) = endpoint(200, r#"{"ico":"00123456","obchodniJmeno":"Example"}"#).await;
    let result = CzRegistryClient::for_test(url, Default::default())
        .lookup("00123456")
        .await
        .unwrap_err();
    assert_eq!(result.id, crate::ErrorId::UnresolvedData);
    assert!(task.await.unwrap().starts_with("GET /00123456 HTTP/1.1"));
    let (url, task) = endpoint(404, r#"{"kod":"NENALEZENO"}"#).await;
    assert!(matches!(
        CzRegistryClient::for_test(url, Default::default())
            .lookup("00123456")
            .await,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                provider: Provider::CzechAres,
                error: RegistryFailure::Http { status: 404 }
            },
            ..
        })
    ));
    task.await.unwrap();
}

#[test]
fn shared_mapping_keeps_tax_flags_and_codes_native() {
    let id = CzBusinessId::parse("00123456").unwrap();
    let raw = br#"{"ico":"00123456","obchodniJmeno":"Example","pravniForma":"112","dic":"CZ00123456","sidlo":{"textovaAdresa":"  Source  ","nazevObce":"Town","nazevStatu":"CESKO","kodStatu":"CZ","pscTxt":"00123"},"seznamRegistraci":{"stavZdrojeDph":"AKTIVNI"}}"#;
    let common: crate::CompanyLookupResult = decode::decode(raw, &id).unwrap().unwrap().into();
    crate::test_support::assert_company_contract(&common);
    assert_eq!(common.source, Provider::CzechAres);
    assert!(common.vat_id.is_none() && common.vat_registered.is_none());
    assert!(common.status.is_none());
    assert_eq!(
        common.legal_form.as_deref(),
        Some("Společnost s ručením omezeným")
    );
    assert!(common.fiscal_address.is_none());
    let a = common.registered_office.unwrap();
    assert_eq!(a.full_address_by_registry.as_deref(), Some("  Source  "));
    assert_eq!(a.postcode.as_deref(), Some("00123"));
    let value = serde_json::to_value(common.registry_details).unwrap();
    assert_eq!(value["provider"], "CzechAres");
    assert_eq!(value["data"]["legalFormCode"], "112");
}

#[test]
fn delivery_number_types_and_orientation_are_preserved() {
    let id = CzBusinessId::parse("00123456").unwrap();
    for (address, expected) in [
        (
            serde_json::json!({"nazevUlice":"Example","cisloDomovni":123,"cisloOrientacni":4,"cisloOrientacniPismeno":"a","typCisloDomovni":1}),
            Some("Example 123/4a"),
        ),
        (
            serde_json::json!({"cisloDomovni":123,"typCisloDomovni":1}),
            Some("č.p. 123"),
        ),
        (
            serde_json::json!({"cisloDomovni":123,"typCisloDomovni":2}),
            Some("č.ev. 123"),
        ),
        (
            serde_json::json!({"cisloDomovni":123,"typCisloDomovni":99}),
            None,
        ),
    ] {
        let raw = serde_json::json!({"ico":"00123456","obchodniJmeno":"Example","sidlo":address});
        let result: crate::CompanyLookupResult =
            decode::decode(&serde_json::to_vec(&raw).unwrap(), &id)
                .unwrap()
                .unwrap()
                .into();
        assert_eq!(
            result.registered_office.unwrap().address.as_deref(),
            expected
        );
    }
}

#[tokio::test]
async fn only_documented_404_envelope_is_a_definitive_miss() {
    let (url, task) = endpoint(
        404,
        r#"{"kod":"NENALEZENO","popis":"Requested economic subject not found"}"#,
    )
    .await;
    assert!(matches!(
        CzRegistryClient::for_test(url, Default::default())
            .lookup("00123456")
            .await,
        Err(LookupError {
            kind: crate::model::LookupFailure::RegistryNotFound {
                provider: Provider::CzechAres
            },
            ..
        })
    ));
    task.await.unwrap();
    for body in [
        r#"{"kod":"OBECNA_CHYBA","popis":"Failure"}"#,
        r#"{"kod":"NENALEZENO","popis":null}"#,
        r#"{}"#,
    ] {
        assert!(!decode::is_definitive_miss(body.as_bytes()));
    }
    assert!(
        decode::decode(
            br#"{"kod":"NENALEZENO","popis":"Not found"}"#,
            &CzBusinessId::parse("00123456").unwrap()
        )
        .is_err()
    );
}

#[test]
fn settlement_delivery_without_street_keeps_the_named_part() {
    let id = CzBusinessId::parse("00123456").unwrap();
    let raw=br#"{"ico":"00123456","obchodniJmeno":"Example","sidlo":{"nazevObce":"Town","nazevCastiObce":"Village","cisloDomovni":85,"typCisloDomovni":1}}"#;
    let common: crate::CompanyLookupResult = decode::decode(raw, &id).unwrap().unwrap().into();
    crate::test_support::assert_company_contract(&common);
    let a = common.registered_office.unwrap();
    assert_eq!(a.address.as_deref(), Some("Village 85"));
    assert_eq!(a.city.as_deref(), Some("Town"));
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
    let client = CzRegistryClient::for_test(url, Default::default());
    let clone = client.clone();
    assert!(matches!(
        client.lookup("00123456").await,
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
        clone.lookup("00123456").await,
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
    let config = CzRegistryConfig {
        max_response_bytes: 32,
        raw_debug: false,
        ..Default::default()
    };
    assert!(matches!(
        CzRegistryClient::for_test(url, config)
            .lookup("00123456")
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
        CzRegistryClient::for_test(url, Default::default())
            .lookup("00123456")
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
fn cessation_date_and_legal_form_are_independent_of_tax_registration() {
    for (date, status) in [
        (Some("2020-01-01"), Some(crate::CompanyStatus::Closed)),
        (Some("invalid"), Some(crate::CompanyStatus::Unknown)),
        (None, None),
    ] {
        let raw = serde_json::json!({"ico":"00123456","obchodniJmeno":"Synthetic","datumZaniku":date,"pravniForma":"112"});
        let result: crate::CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&raw).unwrap(),
            &CzBusinessId::parse("00123456").unwrap(),
        )
        .unwrap()
        .unwrap()
        .into();
        assert_eq!(result.status, status);
        assert_eq!(
            result.legal_form.as_deref(),
            Some("Společnost s ručením omezeným")
        );
        assert_eq!(result.vat_registered, None);
        crate::test_support::assert_company_contract(&result);
    }
    assert_eq!(super::legal_forms::label("106"), None); // Historically ambiguous labels.
    assert_eq!(super::legal_forms::label("000"), None); // Unspecified.
    assert_eq!(super::legal_forms::label("999999"), None);
}

#[test]
fn primary_ros_subject_state_establishes_active_lifecycle() {
    use crate::CompanyStatus::*;
    use serde_json::{Value, json};
    let base = json!({"ico":"00123456","obchodniJmeno":"Synthetic s.r.o.","primarniZdroj":"ros","seznamRegistraci":{"stavZdrojeRos":"AKTIVNI","stavZdrojeIr":"NEEXISTUJICI","stavZdrojeCeu":"NEEXISTUJICI","stavZdrojeDph":"ZANIKLY"}});
    for (path, value, expected) in [
        ("/primarniZdroj", json!("ros"), Some(Active)),
        ("/primarniZdroj", json!("rzp"), None),
        (
            "/seznamRegistraci/stavZdrojeRos",
            json!("ZANIKLY"),
            Some(Closed),
        ),
        (
            "/seznamRegistraci/stavZdrojeRos",
            json!("BUDOUCI"),
            Some(Unknown),
        ),
        (
            "/seznamRegistraci/stavZdrojeRos",
            json!("NEW"),
            Some(Unknown),
        ),
        ("/seznamRegistraci/stavZdrojeRos", Value::Null, None),
        (
            "/seznamRegistraci/stavZdrojeIr",
            json!("AKTIVNI"),
            Some(Unknown),
        ),
        (
            "/seznamRegistraci/stavZdrojeCeu",
            Value::Null,
            Some(Unknown),
        ),
    ] {
        let mut s = base.clone();
        *s.pointer_mut(path).unwrap() = value;
        let result: crate::CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&s).unwrap(),
            &CzBusinessId::parse("00123456").unwrap(),
        )
        .unwrap()
        .unwrap()
        .into();
        assert_eq!(result.status, expected, "{path}");
        crate::test_support::assert_company_contract(&result);
    }
    let mut invalid = base;
    invalid["datumZaniku"] = json!("2024-02-30");
    let result: crate::CompanyLookupResult = decode::decode(
        &serde_json::to_vec(&invalid).unwrap(),
        &CzBusinessId::parse("00123456").unwrap(),
    )
    .unwrap()
    .unwrap()
    .into();
    assert_eq!(result.status, Some(Unknown));
}

#[test]
fn documented_company_output_matches_synthetic_source() {
    let raw = r#"{"ico":"00123456","obchodniJmeno":"Example","dic":"CZ00123456","sidlo":{"textovaAdresa":"  Source address  ","pscTxt":"00123"},"seznamRegistraci":{"stavZdrojeDph":"AKTIVNI"},"datumZaniku":"2020-01-01"}"#;
    let id = CzBusinessId::parse("00123456").unwrap();
    let native = decode::decode(raw.as_bytes(), &id).unwrap().unwrap();
    crate::test_support::assert_documented_company("cz", native);
}

#[test]
fn ros_closure_requires_exact_unique_identity_state_and_date() {
    use serde_json::json;
    let id = CzBusinessId::parse("00123456").unwrap();
    for (record, resolved) in [
        (
            json!({"ico":"00123456","datumZaniku":"2025-10-09","stavSubjektu":"ZANIKLY"}),
            true,
        ),
        (
            json!({"ico":"00123456","datumZaniku":"2025-02-31","stavSubjektu":"ZANIKLY"}),
            false,
        ),
        (
            json!({"ico":"00123456","datumZaniku":"2025-10-09","stavSubjektu":"AKTIVNI"}),
            false,
        ),
        (json!({"ico":"00123456","stavSubjektu":"ZANIKLY"}), false),
    ] {
        let body = json!({"icoId":id.as_str(),"zaznamy":[record]});
        assert_eq!(
            decode::ros_closure(&serde_json::to_vec(&body).unwrap(), &id)
                .unwrap()
                .is_some(),
            resolved
        );
    }
    for body in [
        json!({"icoId":"00123457","zaznamy":[]}),
        json!({"icoId":id.as_str(),"zaznamy":[{"ico":"00123457"}]}),
    ] {
        assert!(matches!(
            decode::ros_closure(&serde_json::to_vec(&body).unwrap(), &id),
            Err(RegistryFailure::MismatchedResponse)
        ));
    }
    let record = json!({"ico":"00123456","datumZaniku":"2025-10-09","stavSubjektu":"ZANIKLY"});
    let multiple = json!({"icoId":id.as_str(),"zaznamy":[record, record]});
    assert!(
        decode::ros_closure(&serde_json::to_vec(&multiple).unwrap(), &id)
            .unwrap()
            .is_none()
    );
    let body = json!({"icoId":id.as_str(),"zaznamy":[]});
    assert!(
        decode::ros_closure(&serde_json::to_vec(&body).unwrap(), &id)
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn ros_supplement_resolves_closure_and_preserves_aggregate() {
    for status in [200, 403, 404, 429] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let task = crate::test_support::http::spawn_server(async move {
            for (path, status, body) in [
                (
                    "/00123456",
                    200,
                    r#"{"ico":"00123456","obchodniJmeno":"Example","primarniZdroj":"rzp","seznamRegistraci":{"stavZdrojeRos":"ZANIKLY","stavZdrojeRzp":"AKTIVNI"}}"#,
                ),
                (
                    "/ros/00123456",
                    status,
                    r#"{"icoId":"00123456","zaznamy":[{"ico":"00123456","datumZaniku":"2025-10-09","stavSubjektu":"ZANIKLY"}]}"#,
                ),
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let request = crate::test_support::http::read_request(&mut socket)
                    .await
                    .unwrap();
                assert!(
                    String::from_utf8(request)
                        .unwrap()
                        .starts_with(&format!("GET {path} HTTP/1.1"))
                );
                socket.write_all(format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            }
        });
        let result = CzRegistryClient::for_test(
            url,
            CzRegistryConfig {
                start_interval: std::time::Duration::from_secs(1),
                ..Default::default()
            },
        )
        .lookup("00123456")
        .await;
        if status == 200 {
            let native = result.unwrap();
            assert!(native.ceased_date.is_none());
            assert_eq!(
                native.ros_closure.as_ref().unwrap().ceased_date,
                "2025-10-09"
            );
            let common: crate::CompanyLookupResult = native.into();
            assert_eq!(common.status, Some(crate::CompanyStatus::Closed));
        } else {
            assert!(matches!(
                result.unwrap_err().kind,
                crate::model::LookupFailure::Registry { .. }
            ));
        }
        task.await.unwrap();
    }
}

#[test]
fn primary_public_register_resolves_without_ros_but_never_overrules_it() {
    use crate::CompanyStatus::{Active, Closed, Unknown};
    use serde_json::json;
    let id = CzBusinessId::parse("00123456").unwrap();
    let base = json!({"ico":"00123456","obchodniJmeno":"Example a.s.","primarniZdroj":"vr",
        "seznamRegistraci":{"stavZdrojeVr":"AKTIVNI","stavZdrojeRos":"NEEXISTUJICI",
            "stavZdrojeIr":"NEEXISTUJICI","stavZdrojeCeu":"NEEXISTUJICI"}});
    let documented: crate::CompanyLookupResult =
        decode::decode(&serde_json::to_vec(&base).unwrap(), &id)
            .unwrap()
            .unwrap()
            .into();
    assert_eq!(
        serde_json::to_value(documented).unwrap(),
        crate::test_support::documentation::example("cz", 2)
    );
    for (field, value, expected) in [
        ("stavZdrojeVr", json!("AKTIVNI"), Some(Active)),
        ("stavZdrojeVr", json!("ZANIKLY"), Some(Closed)),
        ("stavZdrojeVr", json!("BUDOUCI"), Some(Unknown)),
        ("stavZdrojeVr", json!("NEW"), Some(Unknown)),
        ("stavZdrojeVr", json!(null), None),
        ("stavZdrojeRos", json!("ZANIKLY"), Some(Unknown)),
        ("stavZdrojeRos", json!("AKTIVNI"), Some(Unknown)),
        ("stavZdrojeRos", json!(null), Some(Unknown)),
        ("stavZdrojeIr", json!("AKTIVNI"), Some(Unknown)),
        ("stavZdrojeCeu", json!(null), Some(Unknown)),
    ] {
        let mut body = base.clone();
        body["seznamRegistraci"][field] = value;
        let result: crate::CompanyLookupResult =
            decode::decode(&serde_json::to_vec(&body).unwrap(), &id)
                .unwrap()
                .unwrap()
                .into();
        assert_eq!(result.status, expected, "{field}");
        assert_eq!(
            serde_json::to_value(&result).unwrap()["registryDetails"]["data"]["nativeRegistryStatus"],
            body["seznamRegistraci"]["stavZdrojeVr"]
        );
        crate::test_support::assert_company_contract(&result);
    }
}
