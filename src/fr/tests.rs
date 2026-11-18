use super::*;
use crate::{CompanyLookupResult, LookupError, Provider, RegistryFailure};
use serde_json::{Value, json};
fn source() -> Value {
    json!({"total_results":1,"results":[{"siren":"001234567","nom_complet":"Synthetic SAS","nature_juridique":"5710","etat_administratif":"A","siege":{"est_siege":true,"siret":"00123456700001","adresse":" 1 RUE EXEMPLE\n00100 VILLE ","numero_voie":"1","type_voie":"RUE","libelle_voie":"EXEMPLE","code_postal":"00100","libelle_commune":"VILLE","region":"11"}}]})
}
#[test]
fn source_fields_and_unknown_common_facts_are_preserved() {
    let result = decode::decode(
        &serde_json::to_vec(&source()).unwrap(),
        &FrBusinessId::parse("001234567").unwrap(),
    )
    .unwrap();
    let result: CompanyLookupResult = result.into();
    crate::test_support::assert_company_contract(&result);
    assert_eq!(result.tax_id, "001234567");
    assert_eq!(result.source, Provider::FrenchRechercheEntreprises);
    assert_eq!(result.status, Some(crate::CompanyStatus::Active));
    assert!(result.vat_id.is_none() && result.vat_registered.is_none());
    assert_eq!(
        result.legal_form.as_deref(),
        Some("SAS, société par actions simplifiée")
    );
    let office = result.registered_office.unwrap();
    assert_eq!(
        office.full_address_by_registry.as_deref(),
        Some(" 1 RUE EXEMPLE\n00100 VILLE ")
    );
    assert_eq!(office.address.as_deref(), Some("1 RUE EXEMPLE"));
    assert_eq!(office.postcode.as_deref(), Some("00100"));
    assert!(office.country.is_none() && office.region.is_none());
}
#[test]
fn search_and_head_office_identity_are_checked() {
    let id = FrBusinessId::parse("001234567").unwrap();
    for (pointer, value) in [
        ("/results/0/siren", json!("001234568")),
        ("/results/0/siege/siret", json!("00123456800001")),
        ("/results/0/siege/est_siege", json!(false)),
        ("/results/0/nom_complet", json!(" ")),
        ("/total_results", json!(2)),
    ] {
        let mut row = source();
        *row.pointer_mut(pointer).unwrap() = value;
        assert!(decode::decode(&serde_json::to_vec(&row).unwrap(), &id).is_err());
    }
    assert!(decode::decode(br#"{"total_results":0,"results":[]}"#, &id).is_err());
    let mut row = source();
    row["results"][0]["siege"] = Value::Null;
    let result: CompanyLookupResult = decode::decode(&serde_json::to_vec(&row).unwrap(), &id)
        .unwrap()
        .into();
    crate::test_support::assert_company_contract(&result);
    assert!(result.registered_office.is_none());
    for bad in ["FR001234567", "00123456700001", "12345678", "00123456９"] {
        assert!(FrBusinessId::parse(bad).is_err());
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
        assert!(request.starts_with("get /?q=001234567&minimal=true&include=siege "));
        assert!(!request.contains("authorization:") && !request.contains("cookie:"));
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
    });
    let native = FrRegistryClient::for_test(endpoint, Default::default());
    let client = crate::Client::with_test_french(native);
    let result = client.lookup_company("001234567", "FR").await.unwrap();
    assert_eq!(result.company_name, "Synthetic SAS");
    server.await.unwrap();
}

#[tokio::test]
async fn invalid_configuration_and_identifier_fail_locally() {
    let config = FrRegistryConfig {
        timeout: std::time::Duration::ZERO,
        ..Default::default()
    };
    assert!(matches!(
        FrRegistryClient::with_config(config),
        Err(LookupError {
            kind: crate::model::LookupFailure::InvalidConfiguration(_),
            ..
        })
    ));
    assert!(matches!(
        FrRegistryClient::new().unwrap().lookup("invalid").await,
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
    let result = FrRegistryClient::for_test(endpoint, Default::default())
        .lookup("001234567")
        .await;
    assert!(matches!(
        result,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                provider: Provider::FrenchRechercheEntreprises,
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
        (Some("A"), Some(crate::CompanyStatus::Active)),
        (Some("C"), Some(crate::CompanyStatus::Closed)),
        (Some("F"), Some(crate::CompanyStatus::Unknown)),
        (Some("A inactive"), Some(crate::CompanyStatus::Unknown)),
        (None, None),
    ] {
        let mut value = source();
        *value.pointer_mut("/results/0/etat_administratif").unwrap() = json!(raw);
        let result: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&value).unwrap(),
            &FrBusinessId::parse("001234567").unwrap(),
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
fn unknown_legal_form_codes_do_not_become_common_labels() {
    assert_eq!(super::legal_forms::label("999999"), None);
    assert_eq!(super::legal_forms::label(""), None);
}

#[test]
fn documented_company_output_matches_synthetic_source() {
    let body = source();

    let id = FrBusinessId::parse("001234567").unwrap();
    let native = decode::decode(&serde_json::to_vec(&body).unwrap(), &id).unwrap();
    crate::test_support::assert_documented_company("fr", native);
}
