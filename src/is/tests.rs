use super::*;
use crate::{CompanyLookupResult, LookupError, Provider, RegistryFailure};
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
    crate::test_support::assert_company_contract(&r);
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
        assert!(request.starts_with("get /4101000000 "));
        assert!(!request.contains("authorization:") && !request.contains("cookie:"));
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
    });
    let native = IsRegistryClient::for_test(endpoint, Default::default());
    let client = crate::Client::with_test_icelandic(native);
    let result = client.lookup_company("4101000000", "IS").await.unwrap_err();
    assert_eq!(result.id, crate::ErrorId::UnresolvedData);
    assert_eq!(result.details.reason, Some("missing_lifecycle"));
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
        Err(LookupError {
            kind: crate::model::LookupFailure::InvalidConfiguration(_),
            ..
        })
    ));
    assert!(matches!(
        IsRegistryClient::new().unwrap().lookup("invalid").await,
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
        socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await.unwrap();
    });
    let result = IsRegistryClient::for_test(endpoint, Default::default())
        .lookup("4101000000")
        .await;
    assert!(matches!(
        result,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                provider: Provider::IcelandicSkatturinn,
                error: RegistryFailure::Http { status: 404 }
            },
            ..
        })
    ));
    server.await.unwrap();
}

#[test]
fn only_explicit_company_deregistration_resolves_lifecycle() {
    use crate::CompanyStatus::*;
    for (subtitle, expected) in [
        (
            "Stofnað/Skráð: 01.01.2000(Félag afskráð 13.12.2018)",
            Some(Closed),
        ),
        (
            "Stofnað/Skráð: 01.01.2000 (Félag afskráð 31.02.2018)",
            Some(Unknown),
        ),
        (
            "Stofnað/Skráð: 01.01.2000(Félag afskráð 13.12.2018) extra",
            Some(Unknown),
        ),
        ("Stofnað/Skráð: 01.01.2000", None),
        ("Stofnað/Skráð: 01.01.2000 (VSK afskráð 13.12.2018)", None),
    ] {
        let page = source().replace("Stofnað/Skráð: 01.01.2000", subtitle);
        let result: CompanyLookupResult =
            decode::decode(page.as_bytes(), &IsBusinessId::parse("4101000000").unwrap())
                .unwrap()
                .into();
        assert_eq!(result.status, expected, "{subtitle}");
        crate::test_support::assert_company_contract(&result);
    }
    let page = source().replace(
        "</div>",
        "<table><tr><td>Félag afskráð 13.12.2018</td></tr></table></div>",
    );
    let result: CompanyLookupResult =
        decode::decode(page.as_bytes(), &IsBusinessId::parse("4101000000").unwrap())
            .unwrap()
            .into();
    assert_eq!(result.status, None);
}

#[test]
fn documented_company_output_matches_synthetic_source() {
    let body = source().replace(
        "Stofnað/Skráð: 01.01.2000",
        "Stofnað/Skráð: 01.01.2000(Félag afskráð 13.12.2018)",
    );
    let native =
        decode::decode(body.as_bytes(), &IsBusinessId::parse("4101000000").unwrap()).unwrap();
    crate::test_support::assert_documented_company("is", native);
}
