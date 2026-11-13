use super::*;
use crate::{LookupError, Provider, RegistryError, RetryAfter, UnavailabilityReason};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::{JoinHandle, JoinSet},
};

fn deed() -> Value {
    json!({"uic":"123456789", "fullName":"\"ПРИМЕР\" ЕООД", "legalForm":10, "deedStatus":1,
    "sections":[{"subUICType":1,"subDeeds":[{"subUICType":1,"subDeedIsClosed":false,"groups":[{"fields":[
        {"fieldIdent":"00010","nameCode":"CR_F_1_L","fieldOperation":3,"htmlData":"<p>123456789</p>"},
        {"fieldIdent":"00030","nameCode":"CR_F_3_L","fieldOperation":3,"htmlData":"<p>Еднолично дружество с ограничена отговорност</p>"},
        {"fieldIdent":"00040","nameCode":"CR_F_4_L","fieldOperation":3,"htmlData":"<p>Example &amp; Co</p>"},
        {"fieldIdent":"00050","nameCode":"CR_F_5_L","fieldOperation":3,"htmlData":"<div><p>Държава: БЪЛГАРИЯ<br>Област: Русе, Община: Русе<br>Населено място: гр. Русе, п.к. 7000<br>ул. Примерна № 1</p></div>"}
    ]}]}]}]})
}
fn report(inner: &str) -> String {
    format!(
        "<html><body><div>РЕГИСТЪР БУЛСТАТ - СПРАВКИ</div><div>Справка за субект на БУЛСТАТ</div>{inner}</body></html>"
    )
}
fn search_row(id: &str, name: &str) -> String {
    format!(
        "<tr><td><a href='https://reports.bulstat.bg/bulstat-ireports/preview?__report=https%3A%2F%2Freports.bulstat.bg%2Fbulstat-ireports%2Fv_rpt1.rptdesign&amp;uic={id}'>{id}</a></td><td>{name}</td></tr>"
    )
}
fn search() -> String {
    report(&format!(
        "<table><tr><th>Код по БУЛСТАТ</th><th>Субект</th></tr>{}{}<tr><td>Общо:</td><td>2</td></tr></table>",
        search_row("987654321", "Друго лице"),
        search_row("123456789", "Пример")
    ))
}
fn detail() -> String {
    report("<table id='tab1'><tr><th colspan='4'>Основни обстоятелства</th></tr>
        <tr><td>Код по БУЛСТАТ</td><td>123456789</td><td>История</td><td>Документи</td></tr>
        <tr><td>Нефизическо лице</td><td>Пример, Example, държава: БЪЛГАРИЯ, правна форма: Сдружение</td><td></td><td></td></tr>
        <tr><td colspan='4'><table><tr><td></td><td>седалище и адрес на управление: БЪЛГАРИЯ, пощенски код 1000, гр.София, ул. Примерна №1</td><td></td><td></td></tr></table></td></tr>
        <tr><td>Състояние</td><td>развиващ дейност</td><td></td><td></td></tr></table>
        <div>Няма намерен резултат!</div>")
}

struct Server {
    base: String,
    calls: Arc<AtomicUsize>,
    requests: Arc<Mutex<Vec<String>>>,
    task: JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    async fn new(responses: Vec<Vec<u8>>, delay: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/", listener.local_addr().unwrap());
        let calls = Arc::new(AtomicUsize::new(0));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let queue = Arc::new(Mutex::new(VecDeque::from(responses)));
        let (counter, captured) = (calls.clone(), requests.clone());
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    connection = listener.accept() => {
                        let (mut socket, _) = connection.unwrap();
                        let (counter, captured, queue) = (counter.clone(), captured.clone(), queue.clone());
                        connections.spawn(async move {
                            let mut request = Vec::new(); let mut buf = [0;2048];
                            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                                let n = socket.read(&mut buf).await.unwrap_or(0); if n == 0 { return; }
                                request.extend_from_slice(&buf[..n]); assert!(request.len() < 16384);
                            }
                            captured.lock().unwrap().push(String::from_utf8(request).unwrap());
                            counter.fetch_add(1, Ordering::SeqCst);
                            let response = queue.lock().unwrap().pop_front().unwrap_or_else(|| http(500,"", ""));
                            tokio::time::sleep(delay).await;
                            let _ = socket.write_all(&response).await;
                        });
                    },
                    Some(result) = connections.join_next(), if !connections.is_empty() => { result.unwrap(); }
                }
            }
        });
        Self {
            base,
            calls,
            requests,
            task,
        }
    }
    fn client(&self) -> BgRegistryClient {
        BgRegistryClient::for_test(
            format!("{}deeds/", self.base),
            format!("{}preview", self.base),
            config(),
        )
    }
    async fn wait(&self) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while self.calls.load(Ordering::SeqCst) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
}
fn config() -> BgRegistryConfig {
    BgRegistryConfig {
        timeout: Duration::from_secs(2),
        connect_timeout: Duration::from_secs(1),
        start_interval: Duration::ZERO,
        ..Default::default()
    }
}
fn http(status: u16, body: &str, content_type: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status} Test\r\nConnection: close\r\nContent-Length: {}\r\n{}\r\n{body}",
        body.len(),
        if content_type.is_empty() {
            String::new()
        } else {
            format!("Content-Type: {content_type}\r\n")
        }
    )
    .into_bytes()
}

#[test]
fn identifier_patterns_overlap_and_do_not_convert_vat_or_personal_ids() {
    for good in ["123456789", "0000000010001", " 000 000 001 "] {
        let id = BgBusinessId::parse(good).unwrap();
        assert!(id.as_str().starts_with('0') || id.as_str() == "123456789");
        assert_eq!(
            serde_json::from_str::<BgBusinessId>(&serde_json::to_string(&id).unwrap()).unwrap(),
            id
        );
    }
    for bad in [
        "BG123456789",
        "1234567890",
        "12345678",
        "12345678901234",
        "１２３４５６７８９",
        "123-456789",
        "",
        "12345678\u{a0}9",
    ] {
        assert!(BgBusinessId::parse(bad).is_err());
    }
    assert!(BgBusinessId::parse(&" ".repeat(65)).is_err());
    assert!(serde_json::from_str::<BgBusinessId>("\"bad\"").is_err());
}

#[tokio::test]
async fn commercial_hit_preserves_identity_and_never_requests_bulstat() {
    let server = Server::new(
        vec![http(200, &deed().to_string(), "application/json")],
        Duration::ZERO,
    )
    .await;
    let record = server.client().lookup("123 456 789").await.unwrap();
    assert_eq!(record.source, Provider::BulgarianCommercialRegister);
    assert_eq!(record.status.as_deref(), Some("active"));
    assert_eq!(record.company_name, "\"ПРИМЕР\" ЕООД");
    assert_eq!(record.legal_form.as_deref(), Some("ЕООД"));
    assert_eq!(record.company_english_name.as_deref(), Some("Example & Co"));
    assert_eq!(record.address.city.as_deref(), Some("Русе"));
    assert_eq!(record.address.country_iso2.as_deref(), Some("BG"));
    assert_eq!(record.address.country.as_deref(), Some("България"));
    assert_eq!(server.calls.load(Ordering::SeqCst), 1);
    let requests = server.requests.lock().unwrap();
    assert!(requests[0].starts_with("GET /deeds/123456789?entryDate="));
    assert!(requests[0].contains("loadFieldsFromAllLegalForms=false"));
    assert!(requests[0].contains("accept-language: bg"));
    assert!(!requests[0].contains("cookie:"));
}

#[tokio::test]
async fn definitive_commercial_miss_falls_back_to_exact_bulstat_record() {
    let server = Server::new(
        vec![
            http(200, "", ""),
            http(200, &search(), "text/html"),
            http(200, &detail(), "text/html"),
        ],
        Duration::ZERO,
    )
    .await;
    let record = server.client().lookup("123456789").await.unwrap();
    assert_eq!(record.source, Provider::Bulstat);
    assert_eq!(record.company_name, "Пример");
    assert_eq!(record.address.city.as_deref(), Some("София"));
    assert_eq!(record.address.country.as_deref(), Some("България"));
    assert_eq!(record.status.as_deref(), Some("развиващ дейност"));
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert!(requests[1].contains("rpt1.rptdesign"));
    assert!(requests[2].contains("v_rpt1.rptdesign"));
}

#[tokio::test]
async fn not_found_requires_two_definitive_misses() {
    let server = Server::new(
        vec![
            http(200, "", ""),
            http(
                200,
                &report("<div>Няма намерен резултат!</div>"),
                "text/html",
            ),
        ],
        Duration::ZERO,
    )
    .await;
    assert!(matches!(
        server.client().lookup("123456789").await,
        Err(LookupError::NotFound)
    ));
    assert_eq!(server.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn invalid_input_skips_both_sources() {
    let server = Server::new(vec![], Duration::ZERO).await;
    for value in ["BG123456789", "1234567890", "garbage"] {
        assert!(matches!(
            server.client().lookup(value).await,
            Err(LookupError::InvalidInput)
        ));
    }
    assert_eq!(server.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn server_failures_are_specific_unavailability_and_never_fall_back() {
    for status in [408, 500, 502, 503, 504] {
        let response = format!(
            "HTTP/1.1 {status} Test\r\nRetry-After: 180\r\nContent-Length: 999999999\r\n\r\n"
        )
        .into_bytes();
        let server = Server::new(vec![response], Duration::ZERO).await;
        let client = server.client();
        let error = client.lookup("123456789").await.unwrap_err();
        assert!(error.is_service_unavailable());
        assert!(matches!(
            error,
            LookupError::Registry {
                provider: Provider::BulgarianCommercialRegister,
                error: RegistryError::ServiceUnavailable {
                    retry_after: Some(RetryAfter::Seconds(180)),
                    ..
                }
            }
        ));
        assert!(matches!(
            client.clone().lookup("123456789").await,
            Err(LookupError::Registry {
                error: RegistryError::CoolingDown { .. },
                ..
            })
        ));
        assert_eq!(server.calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn bulstat_outage_after_commercial_miss_is_not_not_found() {
    let server = Server::new(
        vec![http(200, "", ""), http(503, "unavailable", "text/html")],
        Duration::ZERO,
    )
    .await;
    let error = server.client().lookup("123456789").await.unwrap_err();
    assert!(error.is_service_unavailable());
    assert!(matches!(
        error,
        LookupError::Registry {
            provider: Provider::Bulstat,
            error: RegistryError::ServiceUnavailable { .. }
        }
    ));
}

#[tokio::test]
async fn wrong_content_access_errors_and_mismatch_never_fall_back() {
    let wrong = deed().to_string().replace("123456789", "987654321");
    let responses = vec![
        http(200, "<html>login</html>", "text/html"),
        http(200, "", "text/html"),
        http(200, "null", "application/json"),
        http(200, " ", "application/json"),
        http(200, "{}", "application/json"),
        http(200, &wrong, "application/json"),
        http(401, "", ""),
        http(403, "", ""),
        http(404, "", ""),
        http(302, "", ""),
        http(204, "", ""),
    ];
    for response in responses {
        let server = Server::new(vec![response], Duration::ZERO).await;
        assert!(matches!(
            server.client().lookup("123456789").await,
            Err(LookupError::Registry { .. })
        ));
        assert_eq!(server.calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn network_timeout_and_closed_connection_remain_unknown_and_sanitized() {
    for (response, delay) in [
        (
            http(200, &deed().to_string(), "application/json"),
            Duration::from_secs(3),
        ),
        (Vec::new(), Duration::ZERO),
        (
            b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nx".to_vec(),
            Duration::ZERO,
        ),
    ] {
        let server = Server::new(vec![response], delay).await;
        let client = BgRegistryClient::for_test(
            format!("{}deeds/", server.base),
            format!("{}preview", server.base),
            BgRegistryConfig {
                timeout: Duration::from_millis(100),
                connect_timeout: Duration::from_millis(50),
                ..config()
            },
        );
        let error = client.lookup("123456789").await.unwrap_err();
        assert!(error.is_service_unavailable());
        assert!(!format!("{error:?} {error}").contains("123456789"));
        assert_eq!(server.calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn clones_share_capacity_and_cancel_releases_it() {
    let server = Server::new(vec![http(200, "", "")], Duration::from_secs(3)).await;
    let client = server.client();
    let clone = client.clone();
    let running = tokio::spawn(async move { clone.lookup("123456789").await });
    server.wait().await;
    assert!(matches!(
        client.lookup("123456789").await,
        Err(LookupError::Busy)
    ));
    running.abort();
    assert!(running.await.unwrap_err().is_cancelled());
    let clone = client.clone();
    let running = tokio::spawn(async move { clone.lookup("123456789").await });
    tokio::time::timeout(Duration::from_secs(1), async {
        while server.calls.load(Ordering::SeqCst) < 2 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    running.abort();
    let _ = running.await;
}

#[tokio::test]
async fn rate_limits_and_local_spacing_are_not_negative_results() {
    let server = Server::new(
        vec![
            b"HTTP/1.1 429 Test\r\nRetry-After: 18446744073709551615\r\nContent-Length: 0\r\n\r\n"
                .to_vec(),
        ],
        Duration::ZERO,
    )
    .await;
    let client = server.client();
    assert!(matches!(
        client.lookup("123456789").await,
        Err(LookupError::Registry {
            error: RegistryError::RateLimited { .. },
            ..
        })
    ));
    assert!(matches!(
        client.lookup("123456789").await,
        Err(LookupError::Registry {
            error: RegistryError::CoolingDown { .. },
            ..
        })
    ));
    let server = Server::new(
        vec![http(200, &deed().to_string(), "application/json")],
        Duration::ZERO,
    )
    .await;
    let client = BgRegistryClient::for_test(
        format!("{}deeds/", server.base),
        format!("{}preview", server.base),
        BgRegistryConfig {
            start_interval: Duration::from_secs(10),
            ..config()
        },
    );
    client.lookup("123456789").await.unwrap();
    assert!(matches!(
        client.lookup("123456789").await,
        Err(LookupError::Busy)
    ));
    assert_eq!(server.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn response_size_is_bounded_before_parsing() {
    let server = Server::new(
        vec![http(200, &deed().to_string(), "application/json")],
        Duration::ZERO,
    )
    .await;
    let client = BgRegistryClient::for_test(
        format!("{}deeds/", server.base),
        format!("{}preview", server.base),
        BgRegistryConfig {
            max_response_bytes: 16,
            ..config()
        },
    );
    assert!(matches!(
        client.lookup("123456789").await,
        Err(LookupError::Registry {
            error: RegistryError::ResponseTooLarge,
            ..
        })
    ));
}

#[test]
fn source_fields_are_selected_by_identity_and_ambiguous_fields_fail() {
    let id = BgBusinessId::parse("123456789").unwrap();
    let mut d = deed();
    let mut branch = d["sections"][0].clone();
    branch["subUICType"] = json!(2);
    branch["subDeeds"][0]["groups"][0]["fields"][3]["htmlData"] =
        json!("<p>Wrong branch address</p>");
    d["sections"].as_array_mut().unwrap().insert(0, branch);
    let record = commercial::decode(&serde_json::to_vec(&d).unwrap(), &id).unwrap();
    assert_eq!(record.address.city.as_deref(), Some("Русе"));
    let mut d = deed();
    d["sections"][0]["subDeeds"][0]["groups"][0]["fields"][3]["fieldOperation"] = json!(2);
    assert!(
        commercial::decode(&serde_json::to_vec(&d).unwrap(), &id)
            .unwrap()
            .address
            .full_address_by_registry
            .is_none()
    );
    let mut d = deed();
    let duplicate = d["sections"][0]["subDeeds"][0]["groups"][0]["fields"][3].clone();
    d["sections"][0]["subDeeds"][0]["groups"][0]["fields"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(commercial::decode(&serde_json::to_vec(&d).unwrap(), &id).is_err());
    let mut d = deed();
    d["sections"][0]["subDeeds"][0]["subDeedIsClosed"] = json!(true);
    assert_eq!(
        commercial::decode(&serde_json::to_vec(&d).unwrap(), &id)
            .unwrap()
            .status
            .as_deref(),
        Some("closed")
    );
}

#[test]
fn bulstat_missing_markup_links_or_conflicting_identity_never_mean_absence() {
    let id = BgBusinessId::parse("123456789").unwrap();
    assert!(bulstat::search(b"", &id).is_err());
    assert!(bulstat::search(b"<html>maintenance</html>", &id).is_err());
    assert!(
        bulstat::search(
            report("<table><tr><td>unknown result</td></tr></table>").as_bytes(),
            &id
        )
        .is_err()
    );
    assert!(
        bulstat::search(
            search()
                .replace("https://reports.bulstat.bg/", "https://evil.test/")
                .as_bytes(),
            &id
        )
        .is_err()
    );
    assert!(matches!(
        bulstat::detail(
            detail().replace("123456789", "987654321").as_bytes(),
            &id,
            "Пример".into()
        ),
        Err(RegistryError::MismatchedResponse)
    ));
    assert!(bulstat::detail(detail().as_bytes(), &id, "Different".into()).is_err());
}

#[test]
fn addresses_preserve_uncertain_source_and_markup_boundaries() {
    assert_eq!(
        html::text("<div>A &amp; <b>B</b><br>C</div><p>D</p><script>bad</script>"),
        "A & B\nC\nD"
    );
    let raw = "Държава: БЪЛГАРИЯ\nНаселено място: с. Пример, п.к. 0123\nрайон Пример\nул. Тест 1";
    let a = address::commercial(Some(raw.into()));
    assert_eq!(a.city.as_deref(), Some("Пример"));
    assert_eq!(a.postcode.as_deref(), Some("0123"));
    assert_eq!(a.address.as_deref(), Some("ул. Тест 1"));
    assert!(a.address2.is_none());
    assert_eq!(a.full_address_by_registry.as_deref(), Some(raw));
    assert_eq!(a.country.as_deref(), Some("България"));
    let a = address::commercial(Some(format!("{raw}\nНаселено място: гр. Друго, п.к. 1000")));
    assert!(a.city.is_none() && a.postcode.is_none());
    let a = address::commercial(Some("Държава: GERMANY\nUnrecognized address".into()));
    assert_eq!(a.country_iso2.as_deref(), Some("DE"));
    assert_eq!(a.country.as_deref(), Some("Deutschland"));
    let unknown = address::commercial(Some(
        "Държава: UNKNOWN COUNTRY\nUnrecognized address".into(),
    ));
    assert!(unknown.country_iso2.is_none());
    assert_eq!(unknown.country.as_deref(), Some("UNKNOWN COUNTRY"));
}

#[test]
fn sofia_date_boundary_handles_dst_and_utc_day_rollover() {
    for (utc, expected) in [
        ("2026-01-01T23:00:00Z", "2026-01-02T23:59:59.999+02:00"),
        ("2026-07-01T22:00:00Z", "2026-07-02T23:59:59.999+03:00"),
        ("2026-03-29T00:00:00Z", "2026-03-29T23:59:59.999+03:00"),
        ("2026-10-25T00:00:00Z", "2026-10-25T23:59:59.999+02:00"),
    ] {
        assert_eq!(client::end_of_day(utc.parse().unwrap()), expected);
    }
    assert!(
        LookupError::Registry {
            provider: Provider::Bulstat,
            error: RegistryError::ServiceUnavailable {
                reason: UnavailabilityReason::Timeout,
                retry_after: None
            }
        }
        .is_service_unavailable()
    );
    assert!(!LookupError::NotFound.is_service_unavailable());
}

#[tokio::test]
#[ignore = "explicit opt-in live registry observation"]
async fn live_bg_registry() {
    let id = std::env::var("BG_REGISTRY_LIVE_ID").expect("set BG_REGISTRY_LIVE_ID explicitly");
    let result = BgRegistryClient::new().unwrap().lookup(&id).await.unwrap();
    assert_eq!(result.tax_id, BgBusinessId::parse(&id).unwrap());
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}

#[test]
fn enriched_record_keeps_dates_roles_money_and_public_links_distinct() {
    let mut d = deed();
    d["hasInstructions"] = json!(true);
    let fields = d["sections"][0]["subDeeds"][0]["groups"][0]["fields"]
        .as_array_mut()
        .unwrap();
    fields[0]["fieldEntryDate"] = json!("2023-10-24T12:26:34");
    for (ident, code, content) in [
        ("00020", "CR_F_2_L", "Пример"),
        ("00060", "CR_F_6_L", "Пълно описание &amp; дейност"),
        ("00070", "CR_F_7_L", "Иван Примеров, Държава: БЪЛГАРИЯ"),
        ("00230", "CR_F_23_L", "Иван Примеров, Държава: БЪЛГАРИЯ"),
        ("00310", "CR_F_31_L", "61.36 €"),
        ("00320", "CR_F_32_L", "1,234 EUR"),
    ] {
        fields.push(json!({"fieldIdent":ident,"nameCode":code,"fieldOperation":3,"fieldEntryDate":"2026-02-15T05:31:27","recordMinActionDate":"1999-01-01T00:00:00","htmlData":format!("<p class='field-text'>{content}</p>")}));
    }
    d["sections"].as_array_mut().unwrap().push(json!({"subUICType":13,"subDeeds":[{"subUICType":13,"subDeedIsClosed":false,"groups":[{"fields":[{"fieldIdent":"1001AJ","nameCode":"CR_F_1001_L","fieldOperation":3,"fieldEntryDate":"2026-02-15T05:31:27","htmlData":"<p><a href='DocumentAccess/example'>Учредителен акт</a><br>Дата на обявяване: 24.10.2023 г. 12:26:34<br><a href='https://evil.example/CR/DocumentAccess/x'>Bad</a><a href='javascript:alert(1)'>Bad</a></p>"}]}]}]}));
    let id = BgBusinessId::parse("123456789").unwrap();
    let record = commercial::decode(&serde_json::to_vec(&d).unwrap(), &id).unwrap();
    let v = serde_json::to_value(record).unwrap();
    assert_eq!(
        v["registryUrl"],
        "https://portal.registryagency.bg/CR/Reports/ActiveConditionTabResult?uic=123456789"
    );
    assert!(v.get("company").is_none() && v.get("foreignName").is_none());
    assert_eq!(v["companyName"], "\"ПРИМЕР\" ЕООД");
    assert_eq!(v["companyEnglishName"], "Example & Co");
    let r = &v["registryDetails"];
    for removed in ["register", "deedStatus", "legalFormCode", "recordClosed"] {
        assert!(r.get(removed).is_none());
    }
    assert_eq!(r["identifierEntryDate"], "2023-10-24");
    assert_eq!(r["identifierEntryYear"], 2023);
    assert_eq!(r["registeredName"], "Пример");
    assert_eq!(r["managers"][0]["name"], "Иван Примеров");
    assert_eq!(r["owners"][0]["role"], "soleCapitalOwner");
    assert_eq!(r["capital"]["registered"]["amount"], "61.36");
    assert_eq!(r["capital"]["registered"]["currency"], "EUR");
    assert!(r["capital"]["paidIn"]["amount"].is_null());
    assert_eq!(r["capital"]["paidIn"]["sourceText"], "1,234 EUR");
    assert_eq!(r["businessActivity"], "Пълно описание & дейност");
    assert_eq!(r["documents"].as_array().unwrap().len(), 1);
    assert_eq!(r["documents"][0]["announcedAtLocal"], "2023-10-24T12:26:34");
    assert_eq!(
        r["documents"][0]["url"],
        "https://portal.registryagency.bg/CR/DocumentAccess/example"
    );
    assert_eq!(r["hasInstructions"], true);
    d["sections"][0]["subDeeds"][0]["groups"][0]["fields"][0]["fieldEntryDate"] =
        json!("not-a-date");
    let v =
        serde_json::to_value(commercial::decode(&serde_json::to_vec(&d).unwrap(), &id).unwrap())
            .unwrap();
    assert!(v["registryDetails"]["identifierEntryYear"].is_null());
    let v =
        serde_json::to_value(bulstat::detail(detail().as_bytes(), &id, "Пример".into()).unwrap())
            .unwrap();
    let url = reqwest::Url::parse(v["registryUrl"].as_str().unwrap()).unwrap();
    assert_eq!(url.host_str(), Some("reports.bulstat.bg"));
    assert!(
        url.query_pairs()
            .any(|(k, v)| k == "uic" && v == "123456789")
    );
}

#[test]
fn registry_delivery_split_omits_district_and_preserves_original_text() {
    let cr = "Държава: БЪЛГАРИЯ\nНаселено място: гр. София, п.к. 1510\nр-н Подуяне\nж.к. Хаджи Димитър, бл. 107, ет. 1, ап. 56";
    let bs = "седалище и адрес на управление: БЪЛГАРИЯ, пощенски код 1510, гр.София, р-н Подуяне, ж.к. Хаджи Димитър, бл. 107, ет. 1, ап. 56";
    for (raw, parsed) in [
        (cr, address::commercial(Some(cr.into()))),
        (bs, address::bulstat(Some(bs.into()))),
    ] {
        assert_eq!(parsed.address.as_deref(), Some("жк. Хаджи Димитър"));
        assert_eq!(parsed.address2.as_deref(), Some("бл. 107, ет. 1, ап. 56"));
        assert!(
            serde_json::to_value(&parsed)
                .unwrap()
                .get("district")
                .is_none()
        );
        assert_eq!(parsed.full_address_by_registry.as_deref(), Some(raw));
    }
    let parsed = address::commercial(Some(
        "Държава: БЪЛГАРИЯ\nбл. 107\nДопълнителни указания".into(),
    ));
    assert!(parsed.address.is_none());
    assert_eq!(
        parsed.address2.as_deref(),
        Some("бл. 107\nДопълнителни указания")
    );
}

#[test]
fn document_dates_are_bound_to_individual_blocks_or_left_unknown() {
    let id = BgBusinessId::parse("123456789").unwrap();
    for (html, dates) in [
        (
            "<div><p><a href='DocumentAccess/a'>A</a><br>Дата на обявяване: 01.02.2020 г. 10:00:00</p><p><a href='DocumentAccess/b'>B</a><br>Дата на обявяване: 03.04.2021 г. 11:00:00</p></div>",
            vec![json!("2020-02-01T10:00:00"), json!("2021-04-03T11:00:00")],
        ),
        (
            "<p><a href='DocumentAccess/a'>A</a><a href='DocumentAccess/b'>B</a><br>Дата на обявяване: 01.02.2020 г. 10:00:00</p>",
            vec![Value::Null, Value::Null],
        ),
        (
            "<div><p><a href='DocumentAccess/a'>A</a></p><p><a href='DocumentAccess/b'>B</a><br>Дата на обявяване: 03.04.2021 г. 11:00:00</p></div>",
            vec![Value::Null, json!("2021-04-03T11:00:00")],
        ),
    ] {
        let mut d = deed();
        d["sections"].as_array_mut().unwrap().push(json!({"subUICType":13,"subDeeds":[{"subUICType":13,"subDeedIsClosed":false,"groups":[{"fields":[{"fieldIdent":"1001AJ","nameCode":"CR_F_1001_L","fieldOperation":3,"htmlData":html}]}]}]}));
        let record = commercial::decode(&serde_json::to_vec(&d).unwrap(), &id).unwrap();
        let v = serde_json::to_value(record).unwrap();
        let docs = v["registryDetails"]["documents"].as_array().unwrap();
        assert_eq!(
            docs.iter()
                .map(|d| d["announcedAtLocal"].clone())
                .collect::<Vec<_>>(),
            dates
        );
    }
}

#[test]
fn registry_uses_shared_normalization_and_country_metadata() {
    let raw = "Държава: БЪЛГАРИЯ\nНаселено място: гр. ОБНОВА -, п.к. 7000\nул. Тест, бл..107, вх.-, ет..2";
    let parsed = address::commercial(Some(raw.into()));
    assert_eq!(parsed.city.as_deref(), Some("Обнова"));
    assert_eq!(parsed.address.as_deref(), Some("ул. Тест"));
    assert_eq!(parsed.address2.as_deref(), Some("бл. 107 ет. 2"));
    assert_eq!(parsed.full_address_by_registry.as_deref(), Some(raw));
    let parsed = address::commercial(Some(
        "Държава: БЪЛГАРИЯ\nбул./ул. КНЯЖЕСКА № 13, бл. ЮТА, вх. 2, ет. 2".into(),
    ));
    assert_eq!(parsed.address.as_deref(), Some("бул./ул. КНЯЖЕСКА № 13"));
    assert_eq!(parsed.address2.as_deref(), Some("бл. ЮТА, вх. 2, ет. 2"));
}

#[test]
fn registry_json_contract_is_stable_and_entry_year_is_derived() {
    let id = BgBusinessId::parse("123456789").unwrap();
    let mut record = commercial::decode(&serde_json::to_vec(&deed()).unwrap(), &id).unwrap();
    let crate::RegistryDetails::BulgarianCommercialRegister(details) = &mut record.registry_details
    else {
        panic!("wrong source")
    };
    details.identifier_entry_date = chrono::NaiveDate::from_ymd_opt(2024, 2, 29);
    let v = serde_json::to_value(&record).unwrap();
    fn keys(value: &Value, expected: &[&str]) {
        let actual: std::collections::BTreeSet<_> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(actual, expected.iter().copied().collect());
    }
    keys(
        &v,
        &[
            "taxId",
            "source",
            "registryUrl",
            "companyName",
            "companyEnglishName",
            "legalForm",
            "address",
            "status",
            "registryDetails",
        ],
    );
    keys(
        &v["address"],
        &[
            "fullAddressByRegistry",
            "address",
            "address2",
            "city",
            "postcode",
            "region",
            "municipality",
            "country",
            "countryISO2",
        ],
    );
    keys(
        &v["registryDetails"],
        &[
            "registeredName",
            "identifierEntryDate",
            "identifierEntryYear",
            "managers",
            "owners",
            "capital",
            "businessActivity",
            "documents",
            "hasInstructions",
        ],
    );
    assert_eq!(v["registryDetails"]["identifierEntryDate"], "2024-02-29");
    assert_eq!(v["registryDetails"]["identifierEntryYear"], 2024);
    let crate::RegistryDetails::BulgarianCommercialRegister(details) = &mut record.registry_details
    else {
        unreachable!()
    };
    details.identifier_entry_date = None;
    let v = serde_json::to_value(record).unwrap();
    assert!(v["registryDetails"]["identifierEntryDate"].is_null());
    assert!(v["registryDetails"]["identifierEntryYear"].is_null());
    let v =
        serde_json::to_value(bulstat::detail(detail().as_bytes(), &id, "Пример".into()).unwrap())
            .unwrap();
    keys(&v["registryDetails"], &["subjectText"]);
}

#[tokio::test]
async fn commercial_probe_never_falls_back_to_bulstat() {
    let server = Server::new(vec![http(200, "", "application/json")], Duration::ZERO).await;
    assert!(
        server
            .client()
            .observe_commercial("123456789")
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(server.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn migrated_uic_field_can_include_labelled_court_file_without_changing_identity() {
    let mut d = deed();
    let id = BgBusinessId::parse("123456789").unwrap();
    d["sections"][0]["subDeeds"][0]["groups"][0]["fields"][0]["htmlData"] =
        json!("<p>123456789<br>Фирмено дело: 123/1999 120</p>");
    assert!(commercial::decode(&serde_json::to_vec(&d).unwrap(), &id).is_ok());
    d["sections"][0]["subDeeds"][0]["groups"][0]["fields"][0]["htmlData"] =
        json!("<p>123456789<br>987654321</p>");
    assert!(commercial::decode(&serde_json::to_vec(&d).unwrap(), &id).is_err());
}

#[test]
fn commercial_address_excludes_labelled_contacts_but_preserves_source() {
    let raw = "Държава: БЪЛГАРИЯ\nНаселено място: гр. Пример, п.к. 1000\nбул./ул. ул. Примерна № 9, ет. 2 Телефон: 000000000\nАдрес на електронна поща: example@example.invalid";
    let a = address::commercial(Some(raw.into()));
    assert_eq!(a.address.as_deref(), Some("ул. Примерна № 9"));
    assert_eq!(a.address2.as_deref(), Some("ет. 2"));
    assert_eq!(a.full_address_by_registry.as_deref(), Some(raw));
}

#[test]
fn manager_without_country_and_separate_document_sections_are_supported() {
    let mut d = deed();
    d["sections"][0]["subDeeds"][0]["subUIC"] = json!("main");
    d["sections"][0]["subDeeds"][0]["groups"][0]["fields"].as_array_mut().unwrap().push(json!({"fieldIdent":"00070","nameCode":"CR_F_7_L","fieldOperation":3,"htmlData":"<p class='field-text'>ИВАН ПРИМЕРОВ</p>"}));
    for (sub, ident, title) in [
        ("main", "10019B", "Accounts"),
        ("branch", "1001AJ", "Wrong branch"),
        ("main", "1001AJ", "Constitution"),
    ] {
        d["sections"].as_array_mut().unwrap().push(json!({"subUICType":13,"subDeeds":[{"subUIC":sub,"subUICType":13,"subDeedIsClosed":false,"groups":[{"fields":[{"fieldIdent":ident,"nameCode":"CR_F_1001_L","fieldOperation":3,"htmlData":format!("<p><a href='DocumentAccess/example'>{title}</a><br>Дата на обявяване: 01.02.2020 г. 10:00:00</p>")}]}]}]}));
    }
    let record = commercial::decode(
        &serde_json::to_vec(&d).unwrap(),
        &BgBusinessId::parse("123456789").unwrap(),
    )
    .unwrap();
    let v = serde_json::to_value(record).unwrap();
    assert_eq!(v["registryDetails"]["managers"][0]["name"], "ИВАН ПРИМЕРОВ");
    assert!(v["registryDetails"]["managers"][0]["country"].is_null());
    assert_eq!(
        v["registryDetails"]["documents"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        v["registryDetails"]["documents"][0]["title"],
        "Constitution"
    );
}

#[test]
#[ignore = "offline regression against local-only reviewed Commercial Register observations"]
fn reviewed_commercial_corpus() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bg/fixtures/commercial");
    assert!(
        root.is_dir(),
        "local-only Commercial Register corpus is not installed"
    );
    let mut checked = 0;
    for file in std::fs::read_dir(root).unwrap() {
        let fixture: Value =
            serde_json::from_slice(&std::fs::read(file.unwrap().path()).unwrap()).unwrap();
        assert_eq!(fixture["review"]["schemaVersion"], 1);
        let id = BgBusinessId::parse(fixture["taxId"].as_str().unwrap()).unwrap();
        let raw = fixture["rawResponse"].as_str().unwrap();
        assert_eq!(
            super::probe::source_sha(raw.as_bytes()),
            fixture["review"]["sourceSha256"].as_str().unwrap(),
            "fixture source changed for {id}"
        );
        let actual =
            serde_json::to_value(commercial::decode(raw.as_bytes(), &id).unwrap()).unwrap();
        for (path, expected) in fixture["review"]["expected"].as_object().unwrap() {
            let mut field = &actual;
            for segment in path.split('.') {
                field = field
                    .get(segment)
                    .unwrap_or_else(|| panic!("missing key {id}: {path}"));
            }
            assert_eq!(field, expected, "{id}: {path}");
        }
        assert!(actual["address"].get("district").is_none());
        checked += 1;
    }
    assert!(checked >= 9);
}

#[tokio::test]
async fn probe_metrics_separate_wait_from_http_and_capture_rate_metadata() {
    let server = Server::new(
        vec![
            http(200, "", "application/json"),
            http(429, "", "text/plain\r\nRetry-After: 90"),
        ],
        Duration::from_millis(20),
    )
    .await;
    let mut cfg = config();
    cfg.start_interval = Duration::from_millis(150);
    let client = BgRegistryClient::for_test(
        format!("{}deeds/", server.base),
        format!("{}preview", server.base),
        cfg,
    );
    client.observe_commercial("123456789").await.unwrap();
    assert_eq!(client.observation_trace().http_status, Some(200));
    assert!(client.observe_commercial("987654321").await.is_err());
    let trace = client.observation_trace();
    assert_eq!(trace.http_status, Some(429));
    assert!(trace.admission_wait_ms >= 80);
    assert!(trace.http_ms.unwrap() >= 15);
    assert!(matches!(trace.retry_after, Some(RetryAfter::Seconds(90))));
}

#[test]
fn closed_joint_stock_records_and_full_branch_identifiers_keep_contract() {
    for (id, legal, abbreviation) in [
        ("123456789", "Акционерно дружество", "АД"),
        ("1234567890123", "Еднолично акционерно дружество", "ЕАД"),
    ] {
        let mut d = deed();
        d["uic"] = json!(id);
        d["sections"][0]["subDeeds"][0]["subDeedIsClosed"] = json!(true);
        d["sections"][0]["subDeeds"][0]["groups"][0]["fields"][0]["htmlData"] =
            json!(format!("<p>{id}</p>"));
        d["sections"][0]["subDeeds"][0]["groups"][0]["fields"][1]["htmlData"] =
            json!(format!("<p>{legal}</p>"));
        let parsed = commercial::decode(
            &serde_json::to_vec(&d).unwrap(),
            &BgBusinessId::parse(id).unwrap(),
        )
        .unwrap();
        assert_eq!(parsed.status.as_deref(), Some("closed"));
        assert_eq!(parsed.legal_form.as_deref(), Some(abbreviation));
        assert_eq!(parsed.tax_id.as_str(), id);
        if id.len() == 13 {
            assert!(
                commercial::decode(
                    &serde_json::to_vec(&d).unwrap(),
                    &BgBusinessId::parse(&id[..9]).unwrap()
                )
                .is_err()
            );
        }
    }
    assert_eq!(
        legal_form::abbreviate("Едноличен търговец"),
        crate::LegalForm::Et
    );
    assert_eq!(
        super::probe::source_sha(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn deleted_company_overrides_open_subdeed_and_removes_portal_status_markup() {
    let mut d = deed();
    d["deedStatus"] = json!(4);
    d["fullName"] =
        json!("\"Пример\" ЕАД<span class=\"text-dark\">&nbsp;-&nbsp;Заличен търговец/ЮЛНЦ</span>");
    d["sections"][0]["subDeeds"][0]["subDeedIsClosed"] = json!(false);
    let record = commercial::decode(
        &serde_json::to_vec(&d).unwrap(),
        &BgBusinessId::parse("123456789").unwrap(),
    )
    .unwrap();
    assert_eq!(record.status, Some(crate::CompanyStatus::Closed));
    assert_eq!(record.company_name, "\"Пример\" ЕАД");
    d["deedStatus"] = json!(999);
    d["fullName"] = json!("Unknown status example");
    let record = commercial::decode(
        &serde_json::to_vec(&d).unwrap(),
        &BgBusinessId::parse("123456789").unwrap(),
    )
    .unwrap();
    assert!(record.status.is_none());
}

#[test]
fn commercial_duplicate_street_label_accepts_missing_marker_space() {
    for (delivery, expected) in [
        ("бул./ул. ул.ЕЛЕДЖИК № 3А", "ул. ЕЛЕДЖИК № 3А"),
        ("бул./ул. бул.ПРИМЕР № 2", "бул. ПРИМЕР № 2"),
        ("бул./ул. ПРИМЕР № 2", "бул./ул. ПРИМЕР № 2"),
        ("бул./ул. жк ЕЛЕНОВО III", "жк. ЕЛЕНОВО III"),
        ("бул./ул. ж.к. ЕЛЕНОВО III", "жк. ЕЛЕНОВО III"),
        (
            "ж.к. ВИТОША, бул./ул. ул. МОГИЛАТА № 17",
            "жк. ВИТОША, ул. МОГИЛАТА № 17",
        ),
    ] {
        let raw = format!("Държава: БЪЛГАРИЯ\n{delivery}");
        let parsed = super::address::commercial(Some(raw.clone()));
        assert_eq!(parsed.address.as_deref(), Some(expected));
        assert_eq!(parsed.full_address_by_registry, Some(raw));
    }
}
