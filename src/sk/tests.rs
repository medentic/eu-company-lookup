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
    assert_eq!(r.tax_id, "00123456");
    assert!(
        r.registered_office.is_none()
            && r.fiscal_address.is_none()
            && r.vat_id.is_none()
            && r.vat_registered.is_none()
            && r.legal_form.is_none()
            && r.status.is_none()
    );
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
async fn two_anonymous_requests_reach_shared_facade() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for (path, body) in [
            (
                "/uctovne-jednotky?zmenene-od=2000-01-01&ico=00123456&max-zaznamov=2",
                json!({"id":[42],"existujeDalsieId":false}).to_string(),
            ),
            ("/uctovna-jednotka?id=42", source().to_string()),
        ] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_lowercase();
            assert!(req.starts_with(&format!("get {path} ")));
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
            .company_name,
        "Synthetic s.r.o."
    );
    server.await.unwrap();
}
#[tokio::test]
async fn source_failure_never_proves_absence() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut s, _) = listener.accept().await.unwrap();
        let mut b = [0; 4096];
        assert!(s.read(&mut b).await.unwrap() > 0);
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
