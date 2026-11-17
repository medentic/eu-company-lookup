use super::*;
use crate::{LookupError, Provider, RegistryFailure};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

async fn endpoint(status: u16, body: &'static str) -> (String, tokio::task::JoinHandle<String>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut bytes = vec![0; 4096];
        let count = socket.read(&mut bytes).await.unwrap();
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
        .unwrap();
    assert_eq!(result.tax_id.as_str(), "00123456");
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
    assert_eq!(common.source, Provider::CzechAres);
    assert!(common.vat_id.is_none() && common.vat_registered.is_none());
    assert!(common.status.is_none() && common.legal_form.is_none());
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
    let a = common.registered_office.unwrap();
    assert_eq!(a.address.as_deref(), Some("Village 85"));
    assert_eq!(a.city.as_deref(), Some("Town"));
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
