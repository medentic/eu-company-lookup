use super::*;
use crate::{CompanyLookupResult, LookupError, Provider, RegistryError};
fn source() -> String {
    r#"<html><div class="company box"><h1>Synthetic ehf. (4101000000)</h1><h2 class="subtitle">Stofnað/Skráð: 01.01.2000</h2><table><thead><tr><th>Póstfang</th><th>Lögheimili</th><th>Sveitarfélag</th><th>Rekstrarform</th></tr></thead><tbody><tr><td> Postal <br> 100 City </td><td> Legal &amp; road <br> 200 Town </td><td>1000<br>Town</td><td>E1 <br>Einkahlutafélag (ehf)</td></tr></tbody></table><h2 class="subtitle">b.t. aðili: Do not expose contact</h2><h3>Forráðamaður</h3><ul><li>Do not expose officer</li></ul></div></html>"#.into()
}
#[test]
fn legal_and_postal_addresses_remain_distinct_and_lossless() {
    let r: CompanyLookupResult = decode::decode(
        source().as_bytes(),
        &IsBusinessId::parse("4101000000").unwrap(),
    )
    .unwrap()
    .into();
    assert_eq!(r.company_name, "Synthetic ehf.");
    assert_eq!(
        r.registered_office
            .as_ref()
            .unwrap()
            .full_address_by_registry
            .as_deref(),
        Some(" Legal & road  200 Town ")
    );
    assert_eq!(r.legal_form.as_deref(), Some("Einkahlutafélag (ehf)"));
    assert!(r.vat_registered.is_none() && r.vat_id.is_none() && r.status.is_none());
    let j = serde_json::to_string(&r).unwrap();
    assert!(!j.contains("Do not expose officer") && !j.contains("Do not expose contact"));
    assert!(j.contains(" Postal  100 City "));
}
#[test]
fn identity_table_labels_and_legal_entity_syntax_are_required() {
    let id = IsBusinessId::parse("4101000000").unwrap();
    for s in [
        source().replace("4101000000", "4201000000"),
        source().replace("Lögheimili", "Other address"),
        source().replace("<h1>", "<h1>wrong</h1><h1>"),
        "<html>Login challenge</html>".into(),
    ] {
        assert!(decode::decode(s.as_bytes(), &id).is_err());
    }
    for bad in [
        "0101000000",
        "4001000000",
        "7201000000",
        "IS4101000000",
        "410100-0000",
        "410100000０",
    ] {
        assert!(IsBusinessId::parse(bad).is_err());
    }
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
        assert!(request.starts_with("get /4101000000 "));
        assert!(!request.contains("authorization:") && !request.contains("cookie:"));
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
    });
    let native = IsRegistryClient::for_test(endpoint, Default::default());
    let client = crate::Client::with_test_icelandic(native);
    let result = client.lookup_company("4101000000", "IS").await.unwrap();
    assert_eq!(result.company_name, "Synthetic ehf.");
    server.await.unwrap();
}

#[tokio::test]
async fn invalid_configuration_and_identifier_fail_locally() {
    let config = IsRegistryConfig {
        timeout: std::time::Duration::ZERO,
        ..Default::default()
    };
    assert!(matches!(
        IsRegistryClient::with_config(config),
        Err(LookupError::InvalidConfiguration(_))
    ));
    assert!(matches!(
        IsRegistryClient::new().unwrap().lookup("invalid").await,
        Err(LookupError::InvalidInput)
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
        socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await.unwrap();
    });
    let result = IsRegistryClient::for_test(endpoint, Default::default())
        .lookup("4101000000")
        .await;
    assert!(matches!(
        result,
        Err(LookupError::Registry {
            provider: Provider::IcelandicSkatturinn,
            error: RegistryError::Http { status: 404 }
        })
    ));
    server.await.unwrap();
}
