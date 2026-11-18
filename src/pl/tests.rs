use super::*;
use crate::{CompanyLookupResult, LookupError, Provider, RegistryFailure};
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
    crate::test_support::assert_company_contract(&v);
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
    crate::test_support::assert_company_contract(&v);
    assert!(v.registered_office.is_none());
    for bad in ["PL0001234567", "1234567", "000123456７"] {
        assert!(PlBusinessId::parse(bad).is_err());
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
        assert!(request.starts_with("get /0001234567?rejestr=p&format=json "));
        assert!(!request.contains("authorization:") && !request.contains("cookie:"));
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
    });
    let native = PlRegistryClient::for_test(endpoint, Default::default());
    let client = crate::Client::with_test_polish(native);
    let result = client.lookup_company("0001234567", "PL").await.unwrap_err();
    assert_eq!(result.id, crate::ErrorId::UnresolvedData);
    assert_eq!(result.details.reason, Some("missing_lifecycle"));
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
        Err(LookupError {
            kind: crate::model::LookupFailure::InvalidConfiguration(_),
            ..
        })
    ));
    assert!(matches!(
        PlRegistryClient::new().unwrap().lookup("invalid").await,
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
        for register in ["P", "S"] {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = crate::test_support::http::read_request(&mut socket)
                .await
                .unwrap();
            assert!(
                String::from_utf8(request)
                    .unwrap()
                    .contains(&format!("?rejestr={register}&format=json"))
            );
            socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}").await.unwrap();
        }
    });
    let result = PlRegistryClient::for_test(endpoint, Default::default())
        .lookup("0001234567")
        .await;
    assert!(matches!(
        result,
        Err(LookupError {
            kind: crate::model::LookupFailure::Registry {
                provider: Provider::PolishKrs,
                error: RegistryFailure::Http { status: 404 }
            },
            ..
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
    crate::test_support::assert_company_contract(&v);
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

#[test]
fn sixth_section_distinguishes_ongoing_and_ended_proceedings() {
    use crate::CompanyStatus::*;
    for (section, expected) in [
        (json!({}), None),
        (
            json!({"likwidacja":[{"otwarcieLikwidacji":"Uchwała o otwarciu likwidacji"}]}),
            Some(Liquidation),
        ),
        (
            json!({"likwidacja":[{"otwarcieLikwidacji":"Opening", "zakonczenieLikwidacji":"Ending"}]}),
            Some(Unknown),
        ),
        (json!({"likwidacja":[{}]}), Some(Unknown)),
        (
            json!({"postepowanieUpadlosciowe":[{"informacjaOOgloszeniuUpadlosci":{"data":"06.02.2006"},"opisZakonczeniaProcesuUpadlosci":{}}]}),
            Some(Bankruptcy),
        ),
        (
            json!({"postepowanieUpadlosciowe":[{"informacjaOOgloszeniuUpadlosci":{"data":"31.02.2006"}}]}),
            Some(Unknown),
        ),
        (
            json!({"postepowanieUpadlosciowe":[{"informacjaOOgloszeniuUpadlosci":{"data":"06.02.2006"},"opisZakonczeniaProcesuUpadlosci":{"newEndingField":"ended"}}]}),
            Some(Unknown),
        ),
        (
            json!({"likwidacja":[{"otwarcieLikwidacji":"Opening"}],"postepowanieUpadlosciowe":[{"informacjaOOgloszeniuUpadlosci":{"data":"06.02.2006"}}]}),
            Some(Unknown),
        ),
        (
            json!({"postepowanieRestrukturyzacyjneNaprawczePrzymusowaRestrukturyzacjaUporzadkowanaLikwidacja":[{"otwarciePostepowaniaRestrukturyzacyjnegoNaprawczegoPrzymusowejRestrukturyzacjiUporzadkowanejLikwidacji":{"rodzajPostepowania":"POSTĘPOWANIE RESTRUKTURYZACYJNE","dataNadaniaKlauzuliWykonalnosci":"23.10.2026"},"zakonczeniePostepowaniaRestrukturyzacyjnegoNaprawczegoPrzymusowejRestrukturyzacjiUporzadkowanejLikwidacji":{}}]}),
            Some(Restructuring),
        ),
    ] {
        let mut body = source();
        body["odpis"]["dane"]["dzial6"] = section;
        let result: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&body).unwrap(),
            &PlBusinessId::parse("0001234567").unwrap(),
        )
        .unwrap()
        .into();
        assert_eq!(result.status, expected);
        crate::test_support::assert_company_contract(&result);
    }
}

#[test]
fn documented_company_output_matches_synthetic_source() {
    let mut body = source();
    body["odpis"]["dane"]["dzial6"] =
        serde_json::json!({"likwidacja":[{"otwarcieLikwidacji":"Opening"}]});
    let id = PlBusinessId::parse("0001234567").unwrap();
    let native = decode::decode(&serde_json::to_vec(&body).unwrap(), &id).unwrap();
    crate::test_support::assert_documented_company("pl", native);
}

#[test]
fn suspension_uses_snapshot_and_valid_nonoverlapping_intervals() {
    use crate::CompanyStatus::*;
    for (entries, snapshot, expected) in [
        (
            json!([{"dataZawieszenia":"01.01.2025"}]),
            Some("18.11.2026"),
            Some(Inactive),
        ),
        (
            json!([{"dataZawieszenia":"01.01.2027"}]),
            Some("18.11.2026"),
            None,
        ),
        (
            json!([{"dataZawieszenia":"01.01.2025","dataWznowienia":"01.03.2025"},{"dataZawieszenia":"02.03.2025"}]),
            Some("18.11.2026"),
            Some(Inactive),
        ),
        (
            json!([{"dataZawieszenia":"01.01.2025","dataWznowienia":"01.03.2025"}]),
            Some("01.03.2025"),
            None,
        ),
        (
            json!([{"dataZawieszenia":"01.01.2025"}]),
            None,
            Some(Unknown),
        ),
        (
            json!([{"dataZawieszenia":"31.02.2025"}]),
            Some("18.11.2026"),
            Some(Unknown),
        ),
        (
            json!([{"dataZawieszenia":"01.01.2025","dataWznowienia":""}]),
            Some("18.11.2026"),
            Some(Unknown),
        ),
        (
            json!([{"dataZawieszenia":"01.01.2025","dataWznowienia":"01.01.2024"}]),
            Some("18.11.2026"),
            Some(Unknown),
        ),
        (
            json!([{"dataZawieszenia":"01.01.2025"},{"dataZawieszenia":"01.02.2025"}]),
            Some("18.11.2026"),
            Some(Unknown),
        ),
    ] {
        let mut body = source();
        body["odpis"]["naglowekA"]["stanZDnia"] = json!(snapshot);
        body["odpis"]["dane"]["dzial6"] = json!({"zawieszenieWznowienie":entries});
        let native = decode::decode(
            &serde_json::to_vec(&body).unwrap(),
            &PlBusinessId::parse("0001234567").unwrap(),
        )
        .unwrap();
        let preserved = serde_json::to_value(&native.details).unwrap();
        for (index, entry) in entries.as_array().unwrap().iter().enumerate() {
            for key in ["dataZawieszenia", "dataWznowienia"] {
                assert_eq!(
                    preserved["dzial6"]["zawieszenieWznowienie"][index][key],
                    entry[key]
                );
            }
        }
        let common: CompanyLookupResult = native.into();
        assert_eq!(common.status, expected);
    }
}

#[path = "tests/mergers.rs"]
mod merger_tests;

fn active_source() -> Value {
    let mut raw = source();
    let header = &mut raw["odpis"]["naglowekA"];
    header["dataRejestracjiWKRS"] = json!("01.01.2001");
    header["dataOstatniegoWpisu"] = json!("01.02.2025");
    header["stanZDnia"] = json!("01.03.2025");
    raw["odpis"]["dane"]["dzial6"] = json!({});
    raw
}
#[test]
fn active_registration_requires_complete_consistent_current_evidence() {
    let status = |raw: Value| -> Option<crate::CompanyStatus> {
        let common: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(&raw).unwrap(),
            &PlBusinessId::parse("0001234567").unwrap(),
        )
        .unwrap()
        .into();
        common.status
    };
    assert_eq!(status(active_source()), Some(crate::CompanyStatus::Active));
    let mut association = active_source();
    association["odpis"]["naglowekA"]["stanPozycji"] = json!(4);
    assert_eq!(status(association), Some(crate::CompanyStatus::Active));
    for (path, value) in [
        ("/odpis/naglowekA/stanZDnia", json!("invalid")),
        ("/odpis/naglowekA/dataOstatniegoWpisu", json!("02.03.2025")),
        ("/odpis/naglowekA/dataRejestracjiWKRS", Value::Null),
        ("/odpis/naglowekA/stanPozycji", json!(999)),
        ("/odpis/naglowekA/stanPozycji", json!(2)),
        ("/odpis/dane/dzial6", Value::Null),
        ("/odpis/dane/dzial6", json!({"newRestriction":{}})),
        ("/odpis/dane/dzial6", json!({"likwidacja":[{}]})),
        (
            "/odpis/dane/dzial1/danePodmiotu/nazwa",
            json!("Synthetic W LIKWIDACJI"),
        ),
    ] {
        let mut raw = active_source();
        *raw.pointer_mut(path).unwrap() = value;
        assert_ne!(status(raw), Some(crate::CompanyStatus::Active), "{path}");
    }
    let mut raw = active_source();
    raw["odpis"]["dane"]["dzial6"] = json!({"postepowanieUpadlosciowe":[{
        "informacjaOOgloszeniuUpadlosci":{"data":"01.01.2020"},
        "opisZakonczeniaProcesuUpadlosci":{"opisSposobuZakonczeniaPostepowania":"UMORZENIE POSTĘPOWANIA UPADŁOŚCIOWEGO","dataZakonczeniaPostepowania":"01.01.2021"}
    }]});
    assert_eq!(status(raw.clone()), Some(crate::CompanyStatus::Active));
    raw["odpis"]["dane"]["dzial6"]["postepowanieUpadlosciowe"][0]["opisZakonczeniaProcesuUpadlosci"]
        ["dataZakonczeniaPostepowania"] = json!("01.01.2030");
    assert_eq!(status(raw), Some(crate::CompanyStatus::Unknown));
}
#[tokio::test]
async fn active_current_extract_needs_only_one_request() {
    use tokio::io::AsyncWriteExt;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let body = active_source().to_string();
    let server = crate::test_support::http::spawn_server(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        crate::test_support::http::read_request(&mut socket)
            .await
            .unwrap();
        socket
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        // Listener drops: an attempted supplemental request cannot succeed.
    });
    let client =
        crate::Client::with_test_polish(PlRegistryClient::for_test(endpoint, Default::default()));
    let result = client.lookup_company("0001234567", "PL").await.unwrap();
    assert_eq!(result.status, Some(crate::CompanyStatus::Active));
    server.await.unwrap();
}

#[test]
fn current_registration_handles_history_and_branch_scope_without_inventing_closure() {
    let status = |raw: &Value| -> Option<crate::CompanyStatus> {
        let common: CompanyLookupResult = decode::decode(
            &serde_json::to_vec(raw).unwrap(),
            &PlBusinessId::parse("0001234567").unwrap(),
        )
        .unwrap()
        .into();
        common.status
    };
    for (label, expected) in [
        ("PRZEJĘCIE INNEJ SPÓŁKI", Some(crate::CompanyStatus::Active)),
        (
            "PRZEKSZTAŁCENIE W INNĄ SPÓŁKĘ HANDLOWĄ",
            Some(crate::CompanyStatus::Active),
        ),
        ("PRZEJĘCIE PRZEZ INNĄ SPÓŁKĘ", None),
        ("UNKNOWN", None),
    ] {
        let mut raw = active_source();
        raw["odpis"]["dane"]["dzial6"] =
            json!({"polaczeniePodzialPrzeksztalcenie":[{"okreslenieOkolicznosci":label}]});
        assert_eq!(status(&raw), expected);
    }
    let mut raw = active_source();
    let section = raw["odpis"]["dane"]["dzial1"].as_object_mut().unwrap();
    let subject = section.remove("danePodmiotu").unwrap();
    section.insert("danePodmiotuZagranicznego".into(), subject);
    section.remove("siedzibaIAdres");
    assert_eq!(status(&raw), Some(crate::CompanyStatus::Active));
    raw["odpis"]["dane"]["dzial6"] = json!({"likwidacja":[{"otwarcieLikwidacji":"Source order"}]});
    assert_eq!(status(&raw), None);
}

#[test]
fn documented_active_output_matches_current_projection() {
    let common: CompanyLookupResult = decode::decode(
        &serde_json::to_vec(&active_source()).unwrap(),
        &PlBusinessId::parse("0001234567").unwrap(),
    )
    .unwrap()
    .into();
    assert_eq!(
        serde_json::to_value(common).unwrap(),
        crate::test_support::documentation::example("pl", 3)
    );
}

#[path = "tests/liquidation.rs"]
mod liquidation_tests;

#[path = "tests/association.rs"]
mod association_tests;
