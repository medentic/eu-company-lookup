use super::*;
use tokio::io::AsyncWriteExt;
fn association_source() -> Value {
    let mut raw = active_source();
    raw["odpis"]["naglowekA"]["rejestr"] = json!("RejS");
    raw["odpis"]["naglowekA"]["stanPozycji"] = json!(3);
    raw["odpis"]["dane"]["dzial1"]["danePodmiotu"]["nazwa"] = json!("Synthetic association");
    raw["odpis"]["dane"]["dzial1"]["danePodmiotu"]["formaPrawna"] = json!("STOWARZYSZENIE");
    raw
}
async fn lookup(
    responses: Vec<(&'static str, u16, String)>,
) -> Result<PlCompanyLookupResult, LookupError> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = crate::test_support::http::spawn_server(async move {
        for (query, status, body) in responses {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = crate::test_support::http::read_request(&mut socket)
                .await
                .unwrap();
            assert!(
                String::from_utf8(request)
                    .unwrap()
                    .starts_with(&format!("GET {query} HTTP/1.1"))
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
    let result = client.lookup("0001234567").await;
    server.await.unwrap();
    result
}
const P: &str = "/0001234567?rejestr=P&format=json";
const S: &str = "/0001234567?rejestr=S&format=json";
const FULL_S: &str = "/full/0001234567?rejestr=S&format=json";
#[tokio::test]
async fn association_fallback_requires_p_404_and_matching_s_header() {
    let common: CompanyLookupResult = lookup(vec![
        (P, 404, String::new()),
        (S, 200, association_source().to_string()),
    ])
    .await
    .unwrap()
    .into();
    assert_eq!(common.status, Some(crate::CompanyStatus::Active));
    assert_eq!(
        serde_json::to_value(common).unwrap(),
        crate::test_support::documentation::example("pl", 6)
    );
    for response in [source(), {
        let mut s = association_source();
        s["odpis"]["naglowekA"]["numerKRS"] = json!("0007654321");
        s
    }] {
        assert!(
            lookup(vec![
                (P, 404, String::new()),
                (S, 200, response.to_string())
            ])
            .await
            .is_err()
        );
    }
    for status in [403, 429, 500] {
        let error = lookup(vec![(P, status, String::new())]).await.unwrap_err();
        assert_eq!(error.details.upstream_http_status, Some(status));
    }
    assert!(lookup(vec![(P, 200, "{}".into())]).await.is_err());
}
#[tokio::test]
async fn missing_current_s_uses_full_s_and_never_infers_closure_from_http_status() {
    let mut closed = super::merger_tests::closed_source();
    closed["odpis"]["naglowekP"]["rejestr"] = json!("RejS");
    closed["odpis"]["naglowekP"]["stanPozycji"] = json!(7);
    let common: CompanyLookupResult = lookup(vec![
        (P, 404, String::new()),
        (S, 204, String::new()),
        (FULL_S, 200, closed.to_string()),
    ])
    .await
    .unwrap()
    .into();
    assert_eq!(common.status, Some(crate::CompanyStatus::Closed));
    assert_eq!(
        serde_json::to_value(common).unwrap(),
        crate::test_support::documentation::example("pl", 7)
    );
    for status in [404, 500] {
        assert!(
            lookup(vec![
                (P, 404, String::new()),
                (S, 204, String::new()),
                (FULL_S, status, String::new())
            ])
            .await
            .is_err()
        );
    }
    closed["odpis"]["naglowekP"]["rejestr"] = json!("RejP");
    assert!(
        lookup(vec![
            (P, 404, String::new()),
            (S, 204, String::new()),
            (FULL_S, 200, closed.to_string())
        ])
        .await
        .is_err()
    );
}
#[test]
fn association_merger_history_preserves_native_description_without_guessing_successor_ids() {
    let mut raw = association_source();
    raw["odpis"]["dane"]["dzial6"] = json!({"polaczeniePodzialPrzeksztalcenie":[{"okreslenieOkolicznosci":"POŁĄCZENIE","opisPodzialuPolaczeniaPrzeksztalcenia":"Source historical merger description","podmiotyPowstalePrzejmujace":[{"nazwa":"Source receiver","identyfikator":{"regon":"00123456700000"}}]}]});
    let id = PlBusinessId::parse("0001234567").unwrap();
    let native = decode::decode_for(&serde_json::to_vec(&raw).unwrap(), &id, "RejS").unwrap();
    assert_eq!(
        native.details.corporate_events[0].role,
        PlReorganizationRole::Unresolved
    );
    assert!(native.details.corporate_events[0].successor_krs.is_empty());
    let common: CompanyLookupResult = native.into();
    assert_eq!(common.status, Some(crate::CompanyStatus::Active));
    raw["odpis"]["dane"]["dzial6"]["polaczeniePodzialPrzeksztalcenie"][0]["okreslenieOkolicznosci"] =
        json!("UNSUPPORTED");
    let common: CompanyLookupResult =
        decode::decode_for(&serde_json::to_vec(&raw).unwrap(), &id, "RejS")
            .unwrap()
            .into();
    assert_ne!(common.status, Some(crate::CompanyStatus::Active));
}
