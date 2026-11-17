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
        assert_eq!(v.vat_registered, flag.as_bool());
        assert!(v.vat_id.is_none());
        assert_eq!(v.legal_form.as_deref(), Some("Aksjeselskap"));
        assert!(v.registered_office.is_none() && v.fiscal_address.is_none() && v.status.is_none());
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
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let body = source().to_string();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0; 4096];
        let n = socket.read(&mut buffer).await.unwrap();
        let request = String::from_utf8_lossy(&buffer[..n]).to_lowercase();
        assert!(request.starts_with("get /001234567 "));
        assert!(!request.contains("authorization:") && !request.contains("cookie:"));
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/vnd.brreg.enhetsregisteret.enhet.v2+json; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
    });
    let native = NoRegistryClient::for_test(endpoint, Default::default());
    let client = crate::Client::with_test_norwegian(native);
    let result = client.lookup_company("001234567", "NO").await.unwrap();
    assert_eq!(result.company_name, "Synthetic AS");
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
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buffer = [0; 4096];
        let n = socket.read(&mut buffer).await.unwrap();
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
