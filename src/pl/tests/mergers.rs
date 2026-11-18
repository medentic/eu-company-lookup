use super::*;
fn id() -> PlBusinessId {
    PlBusinessId::parse("0001234567").unwrap()
}
fn party(id: &str) -> Value {
    json!({"nazwa":"Synthetic receiver","krajNazwaRejestru":"POLSKA, KRAJOWY REJESTR SĄDOWY","krs":{"krs":id}})
}
fn event(kind: &str) -> Value {
    json!({"okreslenieOkolicznosci":kind,"opisPolaczeniaPodzialuPrzeksztalcenia":"Synthetic transaction description"})
}
fn current(entry: Value) -> PlCompanyLookupResult {
    let mut s = source();
    s["odpis"]["dane"]["dzial6"] = json!({"polaczeniePodzialPrzeksztalcenie":[entry]});
    decode::decode(&serde_json::to_vec(&s).unwrap(), &id()).unwrap()
}
#[test]
fn structured_direction_does_not_invent_current_lifecycle() {
    use PlReorganizationRole::*;
    for (kind, expected) in [
        ("PRZEJĘCIE INNEJ SPÓŁKI", Acquirer),
        ("PRZEJĘCIE PRZEZ INNĄ SPÓŁKĘ", Absorbed),
        (
            "PRZEJĘCIE CZĘŚCI MAJĄTKU INNEJ SPÓŁKI W WYNIKU PODZIAŁU",
            AssetRecipient,
        ),
        (
            "WYDZIELENIE CZĘŚCI MAJĄTKU SPÓŁKI W WYNIKU PODZIAŁU",
            PartialDivision,
        ),
        ("PRZEKSZTAŁCENIE W INNĄ SPÓŁKĘ HANDLOWĄ", Transformation),
        ("OTHER", Unresolved),
    ] {
        let mut entry = event(kind);
        entry["podmiotyPowstalePrzejmujace"] = json!([
            party("0007654321"),
            party("0000000000"),
            party("123"),
            party("0007654321")
        ]);
        if expected == Acquirer {
            entry["podmiotyPowstalePrzejmujace"] = Value::Null;
        }
        let native = current(entry);
        let e = &native.details.corporate_events[0];
        assert_eq!(e.role, expected);
        assert_eq!(
            e.successor_krs,
            if expected == Absorbed {
                vec!["0007654321"]
            } else {
                vec![]
            }
        );
        if expected != Acquirer {
            assert_eq!(
                e.source.podmioty_powstale_przejmujace.as_ref().unwrap()[1]
                    .krs
                    .as_ref()
                    .unwrap()
                    .krs
                    .as_deref(),
                Some("0000000000")
            );
        }
        let common: CompanyLookupResult = native.into();
        assert_eq!(common.tax_id, "0001234567");
        assert_eq!(common.status, None); // A merger plan or old transaction is not closure or current activity.
    }
}
#[test]
fn resulting_entity_and_contradictory_roles_are_distinct() {
    let mut e = event("POŁĄCZENIE");
    e["podmiotyPowstalePrzejmujace"] = json!([party(id().as_str())]);
    assert_eq!(
        current(e.clone()).details.corporate_events[0].role,
        PlReorganizationRole::ResultingEntity
    );
    e["podmiotyPrzejmowane"] = json!([party(id().as_str())]);
    assert_eq!(
        current(e).details.corporate_events[0].role,
        PlReorganizationRole::Unresolved
    );
    let mut e = event("PRZEJĘCIE PRZEZ INNĄ SPÓŁKĘ");
    e["podmiotyPowstalePrzejmujace"] = json!([party(id().as_str())]);
    assert_eq!(
        current(e).details.corporate_events[0].role,
        PlReorganizationRole::Unresolved
    );
}
fn history(key: &str, value: Value, introduced: &str) -> Value {
    json!([{key:value,"nrWpisuWprow":introduced,"nrWpisuWykr":"3"}])
}
pub(super) fn closed_source() -> Value {
    json!({"odpis":{"rodzaj":"Pełny","naglowekP":{"rejestr":"RejP","numerKRS":"0001234567","stanZDnia":"10.03.2025","stanPozycji":2,"wpis":[
        {"numerWpisu":1,"opis":"REJESTRACJA W KRAJOWYM REJESTRZE SĄDOWYM","dataWpisu":"01.01.2020"},
        {"numerWpisu":2,"opis":"ZMIANA DANYCH W REJESTRZE","dataWpisu":"01.02.2025"},
        {"numerWpisu":3,"opis":"WYKREŚLENIE Z KRAJOWEGO REJESTRU SĄDOWEGO","dataWpisu":"10.02.2025","dataUprawomocnienia":"01.03.2025"}
    ]},"dane":{"dzial1":{"danePodmiotu":{"nazwa":history("nazwa",json!("Synthetic absorbed company"),"1"),"formaPrawna":history("formaPrawna",json!("SPÓŁKA Z OGRANICZONĄ ODPOWIEDZIALNOŚCIĄ"),"1")}},"dzial6":{"polaczeniePodzialPrzeksztalcenie":[{
        "okreslenieOkolicznosci":history("okreslenieOkolicznosci",json!("PRZEJĘCIE PRZEZ INNĄ SPÓŁKĘ"),"2"),
        "opisPolaczeniaPodzialuPrzeksztalcenia":history("opisPolaczeniaPodzialuPrzeksztalcenia",json!("Synthetic merger into the named receiver"),"2"),
        "podmiotyPowstalePrzejmujace":[{"nazwa":history("nazwa",json!("Synthetic surviving company"),"2"),"krajNazwaRejestru":history("krajNazwaRejestru",json!("POLSKA,KRAJOWY REJESTR SĄDOWY"),"2"),"numerWRejestrzeAlboEwidencji":history("numerWRejestrzeAlboEwidencji",json!("0007654321"),"2")}]
    }]}}}})
}
#[test]
fn confirmed_removal_keeps_old_identity_and_named_successor() {
    let native = full::decode(&serde_json::to_vec(&closed_source()).unwrap(), &id()).unwrap();
    assert_eq!(
        native.details.corporate_events[0].successor_krs,
        vec!["0007654321"]
    );
    let common: CompanyLookupResult = native.into();
    assert_eq!(common.status, Some(crate::CompanyStatus::Closed));
    assert_eq!(common.tax_id, "0001234567");
    assert_eq!(common.company_name, "Synthetic absorbed company");
    crate::test_support::assert_company_contract(&common);
}
#[test]
fn incomplete_or_conflicting_history_cannot_prove_closure() {
    for (pointer, value) in [
        ("/odpis/naglowekP/numerKRS", json!("0007654321")),
        ("/odpis/naglowekP/rejestr", json!("RejS")),
        (
            "/odpis/naglowekP/wpis/2/opis",
            json!("ZMIANA DANYCH W REJESTRZE"),
        ),
        ("/odpis/naglowekP/wpis/2/dataWpisu", json!("30.02.2025")),
        ("/odpis/naglowekP/wpis/2/dataUprawomocnienia", Value::Null),
        (
            "/odpis/naglowekP/wpis/2/dataUprawomocnienia",
            json!("01.03.2026"),
        ),
        (
            "/odpis/naglowekP/wpis/2/dataUprawomocnienia",
            json!("01.01.2025"),
        ),
        ("/odpis/naglowekP/wpis/2/numerWpisu", json!(2)),
        (
            "/odpis/dane/dzial1/danePodmiotu/nazwa/0/nrWpisuWykr",
            json!("2"),
        ),
        (
            "/odpis/dane/dzial1/danePodmiotu/nazwa/0/nrWpisuWprow",
            json!("4"),
        ),
    ] {
        let mut raw = closed_source();
        *raw.pointer_mut(pointer).unwrap() = value;
        assert!(
            full::decode(&serde_json::to_vec(&raw).unwrap(), &id()).is_err(),
            "{pointer}"
        );
    }
    let mut raw = closed_source();
    let name = &mut raw["odpis"]["dane"]["dzial1"]["danePodmiotu"]["nazwa"];
    let duplicate = name[0].clone();
    name.as_array_mut().unwrap().push(duplicate);
    assert!(full::decode(&serde_json::to_vec(&raw).unwrap(), &id()).is_err());
}
#[test]
fn withdrawn_successor_and_foreign_number_are_not_current_krs_successors() {
    let mut raw = closed_source();
    raw["odpis"]["dane"]["dzial6"]["polaczeniePodzialPrzeksztalcenie"][0]["podmiotyPowstalePrzejmujace"]
        [0]["krajNazwaRejestru"][0]["krajNazwaRejestru"] = json!("FOREIGN,OTHER REGISTER");
    let native = full::decode(&serde_json::to_vec(&raw).unwrap(), &id()).unwrap();
    assert!(native.details.corporate_events[0].successor_krs.is_empty());
    assert_eq!(
        native.details.corporate_events[0]
            .source
            .podmioty_powstale_przejmujace
            .as_ref()
            .unwrap()[0]
            .numer_w_rejestrze_albo_ewidencji
            .as_deref(),
        Some("0007654321")
    );
}
#[tokio::test]
async fn current_204_requires_full_final_removal_and_keeps_transport_failures() {
    use tokio::io::AsyncWriteExt;
    for status in [200, 403, 404, 429] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let body = closed_source().to_string();
        let server = crate::test_support::http::spawn_server(async move {
            for (path, status, body) in [
                ("/0001234567?rejestr=P&format=json", 204, String::new()),
                ("/full/0001234567?rejestr=P&format=json", status, body),
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
        let client = PlRegistryClient::for_test(
            endpoint,
            PlRegistryConfig {
                start_interval: std::time::Duration::from_secs(1),
                ..Default::default()
            },
        );
        let result = client.lookup(id().as_str()).await;
        if status == 200 {
            let common: CompanyLookupResult = result.unwrap().into();
            assert_eq!(common.status, Some(crate::CompanyStatus::Closed));
        } else {
            assert!(matches!(
                result.unwrap_err().kind,
                crate::model::LookupFailure::Registry { .. }
            ));
        }
        server.await.unwrap();
    }
}
#[test]
fn documented_closed_merger_matches_source() {
    let common: CompanyLookupResult =
        full::decode(&serde_json::to_vec(&closed_source()).unwrap(), &id())
            .unwrap()
            .into();
    assert_eq!(
        serde_json::to_value(common).unwrap(),
        crate::test_support::documentation::example("pl", 2)
    );
}

#[test]
fn merging_into_new_entity_retains_receiver_without_redirecting() {
    let mut e = event("POŁĄCZENIE");
    e["podmiotyPowstalePrzejmujace"] = json!([party("0007654321")]);
    e["podmiotyPrzejmowane"] = json!([party(id().as_str()), party("0001111111")]);
    let native = current(e);
    assert_eq!(
        native.details.corporate_events[0].role,
        PlReorganizationRole::Absorbed
    );
    assert_eq!(
        native.details.corporate_events[0].successor_krs,
        vec!["0007654321"]
    );
    assert_eq!(native.tax_id, id());
    let common: CompanyLookupResult = native.into();
    assert_eq!(common.status, None);
}
#[test]
fn withdrawn_merger_does_not_supply_a_successor() {
    let mut raw = closed_source();
    let event = &mut raw["odpis"]["dane"]["dzial6"]["polaczeniePodzialPrzeksztalcenie"][0];
    for key in [
        "okreslenieOkolicznosci",
        "opisPolaczeniaPodzialuPrzeksztalcenia",
    ] {
        event[key][0]["nrWpisuWykr"] = json!("2");
    }
    let native = full::decode(&serde_json::to_vec(&raw).unwrap(), &id()).unwrap();
    assert!(native.details.corporate_events.is_empty());
    let common: CompanyLookupResult = native.into();
    assert_eq!(common.status, Some(crate::CompanyStatus::Closed));
}
#[test]
fn current_payload_cannot_inject_full_extract_removal() {
    let mut raw = source();
    raw["odpis"]["dane"]["fullExtractRemoval"] = json!({"entryNumber":3,"description":"WYKREŚLENIE Z KRAJOWEGO REJESTRU SĄDOWEGO","entryDate":"10.02.2025","finalDate":"01.03.2025"});
    let native = decode::decode(&serde_json::to_vec(&raw).unwrap(), &id()).unwrap();
    assert!(native.details.full_extract_removal.is_none());
}

#[test]
fn full_extract_address_history_uses_components_and_discards_withdrawn_address() {
    let mut raw = closed_source();
    raw["odpis"]["dane"]["dzial1"]["siedzibaIAdres"] = json!({
        "adres":[{"ulica":"Old street","nrWpisuWprow":"1","nrWpisuWykr":"2"},{"ulica":"Source street","nrDomu":"7","nrWpisuWprow":"2","nrWpisuWykr":"3"}],
        "siedziba":[{"miejscowosc":"Source town","nrWpisuWprow":"1","nrWpisuWykr":"3"}]
    });
    let native = full::decode(&serde_json::to_vec(&raw).unwrap(), &id()).unwrap();
    let common: CompanyLookupResult = native.into();
    assert_eq!(
        common.registered_office.unwrap().address.as_deref(),
        Some("Source street 7")
    );
}

#[test]
fn explicitly_nonfinal_removal_is_closed_with_source_marker() {
    let mut raw = closed_source();
    let last = &mut raw["odpis"]["naglowekP"]["wpis"][2];
    last.as_object_mut().unwrap().remove("dataUprawomocnienia");
    last["nieprawomocny"] = json!("Wpis jest nieprawomocny");
    let native = full::decode(&serde_json::to_vec(&raw).unwrap(), &id()).unwrap();
    let common: CompanyLookupResult = native.clone().into();
    assert_eq!(common.status, Some(crate::CompanyStatus::Closed));
    crate::model::company_validation::ensure_resolved(native).unwrap();
    assert_eq!(
        serde_json::to_value(common).unwrap()["registryDetails"]["data"]["nativeRegistryStatus"],
        "Wpis jest nieprawomocny"
    );
}

#[test]
fn generic_acquisition_and_transformation_preserve_current_identity() {
    let mut raw = active_source();
    raw["odpis"]["naglowekA"]["rejestr"] = json!("RejS");
    raw["odpis"]["naglowekA"]["stanPozycji"] = json!(3);
    raw["odpis"]["dane"]["dzial6"] = json!({"polaczeniePodzialPrzeksztalcenie":[event("PRZEKSZTAŁCENIE"), {"okreslenieOkolicznosci":"POŁĄCZENIE","opisPolaczeniaPodzialuPrzeksztalcenia":"Synthetic hospital acquisition","podmiotyPrzejmowane":[{"nazwa":"Synthetic acquired hospital","identyfikator":{"regon":"123456789"}}]}]});
    let native = decode::decode_for(&serde_json::to_vec(&raw).unwrap(), &id(), "RejS").unwrap();
    assert_eq!(
        native.details.corporate_events[0].role,
        PlReorganizationRole::Transformation
    );
    assert_eq!(
        native.details.corporate_events[1].role,
        PlReorganizationRole::Acquirer
    );
    assert!(native.details.corporate_events[1].successor_krs.is_empty());
    let result: CompanyLookupResult = native.into();
    assert_eq!(result.status, Some(crate::CompanyStatus::Active));
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        crate::test_support::documentation::example("pl", 10)
    );
    let mut e = event("POŁĄCZENIE");
    e["podmiotyPrzejmowane"] = json!([party(id().as_str())]);
    assert_eq!(
        current(e.clone()).details.corporate_events[0].role,
        PlReorganizationRole::Unresolved
    );
    e["podmiotyPrzejmowane"] = json!([party("0007654321")]);
    e["okreslenieOkolicznosci"] = json!("PRZEJĘCIE PRZEZ INNĄ SPÓŁKĘ");
    assert_eq!(
        current(e).details.corporate_events[0].role,
        PlReorganizationRole::Unresolved
    );
}

#[test]
fn empty_acquired_party_does_not_establish_generic_merger_direction() {
    let mut e = event("POŁĄCZENIE");
    e["podmiotyPrzejmowane"] = json!([{}]);
    assert_eq!(
        current(e).details.corporate_events[0].role,
        PlReorganizationRole::Unresolved
    );
}
