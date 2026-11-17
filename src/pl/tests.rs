use super::*;
use crate::{CompanyLookupResult, LookupError, Provider, RegistryError};
use serde_json::{Value, json};
fn source() -> Value {
    json!({"odpis":{"rodzaj":"Aktualny","naglowekA":{"rejestr":"RejP","numerKRS":"0001234567","stanPozycji":1},"dane":{"dzial1":{"danePodmiotu":{"nazwa":"Synthetic sp. z o.o.","formaPrawna":"Source form","identyfikatory":{"nip":"0012345678","regon":"00123456700000"}},"siedzibaIAdres":{"adres":{"ulica":"Source street","nrDomu":"1","nrLokalu":"2","miejscowosc":"Town","kodPocztowy":"00-001","kraj":"POLSKA"}}}}}})
}
#[test]
fn identifier_schemes_and_source_components_are_preserved() {
    let v: CompanyLookupResult = decode::decode(
        &serde_json::to_vec(&source()).unwrap(),
        &PlBusinessId::parse("0001234567").unwrap(),
    )
    .unwrap()
    .into();
    assert_eq!(v.tax_id, "0001234567");
    assert!(v.vat_id.is_none() && v.vat_registered.is_none());
    assert_eq!(v.legal_form.as_deref(), Some("Source form"));
    let a = v.registered_office.as_ref().unwrap();
    assert!(a.full_address_by_registry.is_none());
    assert_eq!(a.address.as_deref(), Some("Source street 1"));
    assert_eq!(a.country.as_deref(), Some("Polska"));
    assert_eq!(a.postcode.as_deref(), Some("00-001"));
    let j = serde_json::to_value(v).unwrap();
    assert_eq!(
        j["registryDetails"]["data"]["danePodmiotu"]["identyfikatory"]["regon"],
        "00123456700000"
    );
}
#[test]
fn header_scope_and_exact_identity_are_required() {
    let id = PlBusinessId::parse("0001234567").unwrap();
    for (p, v) in [
        ("/odpis/rodzaj", json!("Pelny")),
        ("/odpis/naglowekA/rejestr", json!("RejS")),
        ("/odpis/naglowekA/numerKRS", json!("0001234568")),
        ("/odpis/dane/dzial1/danePodmiotu/nazwa", json!(" ")),
    ] {
        let mut s = source();
        *s.pointer_mut(p).unwrap() = v;
        assert!(decode::decode(&serde_json::to_vec(&s).unwrap(), &id).is_err());
    }
    let mut s = source();
    s["odpis"]["dane"]["dzial1"]["siedzibaIAdres"] = Value::Null;
    let v: CompanyLookupResult = decode::decode(&serde_json::to_vec(&s).unwrap(), &id)
        .unwrap()
        .into();
    assert!(v.registered_office.is_none());
    for bad in ["PL0001234567", "1234567", "000123456７"] {
        assert!(PlBusinessId::parse(bad).is_err());
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
        assert!(request.starts_with("get /0001234567?rejestr=p&format=json "));
        assert!(!request.contains("authorization:") && !request.contains("cookie:"));
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
    });
    let native = PlRegistryClient::for_test(endpoint, Default::default());
    let client = crate::Client::with_test_polish(native);
    let result = client.lookup_company("0001234567", "PL").await.unwrap();
    assert_eq!(result.company_name, "Synthetic sp. z o.o.");
    server.await.unwrap();
}

#[tokio::test]
async fn invalid_configuration_and_identifier_fail_locally() {
    let config = PlRegistryConfig {
        timeout: std::time::Duration::ZERO,
        ..Default::default()
    };
    assert!(matches!(
        PlRegistryClient::with_config(config),
        Err(LookupError::InvalidConfiguration(_))
    ));
    assert!(matches!(
        PlRegistryClient::new().unwrap().lookup("invalid").await,
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
        socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await.unwrap();
    });
    let result = PlRegistryClient::for_test(endpoint, Default::default())
        .lookup("0001234567")
        .await;
    assert!(matches!(
        result,
        Err(LookupError::Registry {
            provider: Provider::PolishKrs,
            error: RegistryError::Http { status: 404 }
        })
    ));
    server.await.unwrap();
}

#[test]
fn foreign_branch_keeps_polish_identity_and_foreign_head_office_separate() {
    let id = PlBusinessId::parse("0001234567").unwrap();
    let mut s = source();
    let section = s["odpis"]["dane"]["dzial1"].as_object_mut().unwrap();
    let mut subject = section.remove("danePodmiotu").unwrap();
    subject["nazwaFormaPrawnaPrzedsiebiorcyZagranicznego"] = json!("Foreign parent");
    section.insert("danePodmiotuZagranicznego".into(), subject.clone());
    let mut office = section.remove("siedzibaIAdres").unwrap();
    office["siedzibaIAdresZakladuGlownego"] =
        json!({"kraj":"WIELKA BRYTANIA", "miejscowosc":"LONDON"});
    section.insert("siedzibaIAdresPodmiotuZagranicznego".into(), office);
    let v: CompanyLookupResult = decode::decode(&serde_json::to_vec(&s).unwrap(), &id)
        .unwrap()
        .into();
    assert_eq!(v.tax_id, "0001234567");
    assert!(v.foreign_name.is_none());
    assert_eq!(
        v.registered_office.as_ref().unwrap().city.as_deref(),
        Some("Town")
    );
    let j = serde_json::to_value(v).unwrap();
    assert_eq!(j["registryDetails"]["data"]["foreignBranch"], true);
    assert_eq!(
        j["registryDetails"]["data"]["siedzibaIAdres"]["siedzibaIAdresZakladuGlownego"]["miejscowosc"],
        "LONDON"
    );
    s["odpis"]["dane"]["dzial1"]["danePodmiotu"] = subject;
    assert!(decode::decode(&serde_json::to_vec(&s).unwrap(), &id).is_err());
}
