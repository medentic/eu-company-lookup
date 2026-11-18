use super::*;
use crate::{CompanyLookupResult, LookupError, Provider, RegistryFailure};
use serde_json::{Value, json};
fn source() -> Value {
    json!({"organisasjonsnummer":"001234567","navn":"Synthetic AS","organisasjonsform":{"kode":"AS","beskrivelse":"Aksjeselskap"},"registrertIMvaregisteret":true,"forretningsadresse":{"adresse":["Original line"],"postnummer":"00100"},"postadresse":{"adresse":["Postal line"]},"konkurs":true})
}
#[test]
fn native_roles_and_domestic_vat_are_distinct_from_vies() {
    let id = NoBusinessId::parse("001234567").unwrap();
    for flag in [Value::Bool(true), Value::Bool(false), Value::Null] {
        let mut s = source();
        s["registrertIMvaregisteret"] = flag.clone();
        let v: CompanyLookupResult = decode::decode(&serde_json::to_vec(&s).unwrap(), &id)
            .unwrap()
            .into();
        crate::test_support::assert_company_contract(&v);
        assert_eq!(v.vat_registered, flag.as_bool());
        assert!(v.vat_id.is_none());
        assert_eq!(v.legal_form.as_deref(), Some("Aksjeselskap"));
        assert!(v.registered_office.is_none() && v.fiscal_address.is_none());
        assert_eq!(v.status, Some(crate::CompanyStatus::Bankruptcy));
        let j = serde_json::to_value(v).unwrap();
        assert_eq!(
            j["registryDetails"]["data"]["forretningsadresse"]["adresse"][0],
            "Original line"
        );
        assert_eq!(
            j["registryDetails"]["data"]["postadresse"]["adresse"][0],
            "Postal line"
        );
    }
}
#[test]
fn identity_and_required_name_are_checked() {
    let id = NoBusinessId::parse("001234567").unwrap();
    for (k, v) in [
        ("organisasjonsnummer", json!("001234568")),
        ("navn", json!(" ")),
    ] {
        let mut s = source();
        s[k] = v;
        assert!(decode::decode(&serde_json::to_vec(&s).unwrap(), &id).is_err());
    }
    for bad in ["NO001234567", "001234567MVA", "12345678", "00123456９"] {
        assert!(NoBusinessId::parse(bad).is_err());
    }
    assert!(decode::decode(b"{}", &id).is_err());
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
        assert!(request.starts_with("get /001234567 "));
        assert!(!request.contains("authorization:") && !request.contains("cookie:"));
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/vnd.brreg.enhetsregisteret.enhet.v2+json; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
    });
    let native = NoRegistryClient::for_test(endpoint, Default::default());
    let client = crate::Client::with_test_norwegian(native);
    let result = client.lookup_company("001234567", "NO").await.unwrap();
    assert_eq!(result.status, Some(crate::CompanyStatus::Bankruptcy));
    server.await.unwrap();
}

#[tokio::test]
async fn invalid_configuration_and_identifier_fail_locally() {
    let config = NoRegistryConfig {
        timeout: std::time::Duration::ZERO,
        ..Default::default()
    };
    assert!(matches!(
        NoRegistryClient::with_config(config),
        Err(LookupError {
            kind: crate::model::LookupFailure::InvalidConfiguration(_),
            ..
        })
    ));
    assert!(matches!(
        NoRegistryClient::new().unwrap().lookup("invalid").await,
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
        socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Type: application/vnd.brreg.enhetsregisteret.enhet.v2+json; charset=UTF-8\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await.unwrap();
    });
    let result = NoRegistryClient::for_test(endpoint, Default::default())
        .lookup("001234567")
        .await;
    assert!(matches!(
        result,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                provider: Provider::NorwegianBrreg,
                error: RegistryFailure::Http { status: 404 }
            },
            ..
        })
    ));
    server.await.unwrap();
}

#[test]
fn lifecycle_flags_and_deletion_dates_do_not_invent_activity() {
    use crate::CompanyStatus::*;
    for (bankrupt, liquidating, deletion, expected) in [
        (false, false, None, None),
        (true, false, None, Some(Bankruptcy)),
        (false, true, None, Some(Liquidation)),
        (true, true, None, Some(Unknown)),
        (false, false, Some("2020-02-29"), Some(Closed)),
        (false, false, Some("2020-02-30"), Some(Unknown)),
        (true, false, Some("2020-02-29"), Some(Unknown)),
    ] {
        let mut raw = source();
        raw["konkurs"] = json!(bankrupt);
        raw["underAvvikling"] = json!(liquidating);
        raw["slettedato"] = json!(deletion);
        let common: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&raw).unwrap(),
            &NoBusinessId::parse("001234567").unwrap(),
        )
        .unwrap()
        .into();
        assert_eq!(common.status, expected);
        let result = crate::model::company_validation::ensure_resolved(common);
        assert_eq!(result.is_ok(), expected.is_some_and(|s| s != Unknown));
        if let Err(error) = result {
            assert_eq!(error.id, crate::ErrorId::UnresolvedData);
        }
    }
}

#[test]
fn current_entity_discriminator_and_complete_flags_resolve_active() {
    use crate::CompanyStatus::*;
    let base = json!({"organisasjonsnummer":"001234567","navn":"Synthetic AS","respons_klasse":"Enhet","registreringsdatoEnhetsregisteret":"2000-01-01","konkurs":false,"underAvvikling":false,"underTvangsavviklingEllerTvangsopplosning":false,"paategninger":[]});
    let project = |s: &Value| -> CompanyLookupResult {
        decode::decode(
            &serde_json::to_vec(s).unwrap(),
            &NoBusinessId::parse("001234567").unwrap(),
        )
        .unwrap()
        .into()
    };
    let result = project(&base);
    assert_eq!(result.status, Some(Active));
    crate::test_support::assert_company_contract(&result);
    for (key, value, expected) in [
        ("respons_klasse", json!("SlettetEnhet"), Some(Unknown)),
        ("respons_klasse", json!("FutureClass"), Some(Unknown)),
        ("respons_klasse", Value::Null, None),
        ("konkurs", Value::Null, None),
        (
            "registreringsdatoEnhetsregisteret",
            json!("2000-02-30"),
            None,
        ),
        (
            "paategninger",
            json!([{"infotype":"NEW","tekst":"Unreviewed annotation"}]),
            None,
        ),
        ("slettedato", json!(""), Some(Unknown)),
        ("slettedato", json!("2024-01-01"), Some(Closed)),
        (
            "underRekonstruksjonsforhandlingDato",
            json!("2024-01-01"),
            Some(Restructuring),
        ),
        (
            "underRekonstruksjonsforhandlingDato",
            json!("2024-02-30"),
            Some(Unknown),
        ),
        (
            "underUtenlandskInsolvensbehandlingDato",
            json!("2024-01-01"),
            Some(Unknown),
        ),
    ] {
        let mut s = base.clone();
        s[key] = value;
        let result = project(&s);
        assert_eq!(result.status, expected, "{key}");
        crate::test_support::assert_company_contract(&result);
    }
}

#[test]
fn documented_company_output_matches_synthetic_source() {
    let mut body = source();
    body["konkurs"] = serde_json::json!(true);
    let id = NoBusinessId::parse("001234567").unwrap();
    let native = decode::decode(&serde_json::to_vec(&body).unwrap(), &id).unwrap();
    crate::test_support::assert_documented_company("no", native);
}
