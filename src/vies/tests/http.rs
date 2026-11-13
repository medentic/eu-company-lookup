use super::super::{Client, ClientConfig, client::parse_retry_after};
use super::soap::{VALID, fault};
use crate::{LimitScope, LookupError, Provider, RetryAfter};
use std::{
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
    time::Instant,
};

struct Server {
    url: String,
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
    async fn new(response: Vec<u8>, delay: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/vies", listener.local_addr().unwrap());
        let calls = Arc::new(AtomicUsize::new(0));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (counter, captured) = (calls.clone(), requests.clone());
        let task = tokio::spawn(async move {
            let mut connections = JoinSet::new();
            loop {
                tokio::select! {
                    accepted = listener.accept() => {
                        let (mut socket, _) = accepted.unwrap();
                        let (response, counter, captured) = (response.clone(), counter.clone(), captured.clone());
                        connections.spawn(async move {
                            let mut data = Vec::new();
                            let mut chunk = [0; 2048];
                            loop {
                                let n = socket.read(&mut chunk).await.unwrap_or(0);
                                if n == 0 { return; }
                                data.extend_from_slice(&chunk[..n]);
                                assert!(data.len() < 16_384);
                                if let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                                    let headers = String::from_utf8_lossy(&data[..end]).to_ascii_lowercase();
                                    let len: usize = headers.lines().find_map(|s| s.strip_prefix("content-length:")).unwrap().trim().parse().unwrap();
                                    if data.len() >= end + 4 + len { break; }
                                }
                            }
                            captured.lock().unwrap().push(String::from_utf8(data).unwrap());
                            counter.fetch_add(1, Ordering::SeqCst);
                            tokio::time::sleep(delay).await;
                            let _ = socket.write_all(&response).await;
                        });
                    },
                    Some(result) = connections.join_next(), if !connections.is_empty() => { result.unwrap(); },
                }
            }
        });
        Self {
            url,
            calls,
            requests,
            task,
        }
    }
    async fn wait_calls(&self, count: usize) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while self.calls.load(Ordering::SeqCst) < count {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }
    fn client(&self) -> Client {
        Client::for_test(self.url.clone(), config())
    }
}
fn config() -> ClientConfig {
    ClientConfig {
        timeout: Duration::from_secs(2),
        connect_timeout: Duration::from_secs(1),
        global_start_interval: Duration::ZERO,
        country_start_interval: Duration::ZERO,
        ..Default::default()
    }
}
fn http(status: u16, body: &str, headers: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status} Test\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n{body}",
        body.len()
    )
    .into_bytes()
}

#[tokio::test]
async fn public_check_sends_exact_request_and_returns_source_observation() {
    let server = Server::new(
        http(200, VALID, "Content-Type: text/xml\r\n"),
        Duration::ZERO,
    )
    .await;
    let before = chrono::Utc::now();
    let result = server.client().lookup_vat("bg 123456789").await.unwrap();
    assert!(result.valid);
    assert_eq!(result.vat_id.as_str(), "BG123456789");
    assert_eq!(result.source, Provider::Vies);
    assert!(result.checked_at >= before && result.checked_at <= chrono::Utc::now());
    let requests = server.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("POST /vies HTTP/1.1"));
    assert!(
        requests[0]
            .to_ascii_lowercase()
            .contains("soapaction: \"\"")
    );
    assert!(requests[0].contains("<countryCode>BG</countryCode><vatNumber>123456789</vatNumber>"));
    assert!(!requests[0].to_ascii_lowercase().contains("accept-language"));
}

#[tokio::test]
async fn boolean_convenience_distinguishes_false_from_failure() {
    let server = Server::new(
        http(200, &VALID.replace("<valid>true", "<valid>false"), ""),
        Duration::ZERO,
    )
    .await;
    assert!(!server.client().is_vat_valid("BG123456789").await.unwrap());
    let server = Server::new(http(500, &fault("MS_UNAVAILABLE"), ""), Duration::ZERO).await;
    assert!(matches!(
        server.client().is_vat_valid("BG123456789").await,
        Err(LookupError::MemberStateUnavailable { .. })
    ));
}

#[tokio::test]
async fn local_failures_send_no_request() {
    let server = Server::new(http(200, VALID, ""), Duration::ZERO).await;
    let client = server.client();
    assert!(matches!(
        client.lookup_vat("bad").await,
        Err(LookupError::InvalidInput)
    ));
    assert!(matches!(
        client.lookup_vat("US123456789").await,
        Err(LookupError::UnsupportedJurisdiction)
    ));
    assert_eq!(client.capabilities().len(), 28);
    assert_eq!(server.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn http_errors_and_redirects_cannot_turn_into_valid_results() {
    for status in [301, 302, 400, 401, 403, 404] {
        let server = Server::new(
            http(
                status,
                VALID,
                "Location: http://127.0.0.1:1/private\r\nRetry-After: 120\r\n",
            ),
            Duration::ZERO,
        )
        .await;
        assert!(matches!(server.client().lookup_vat("BG123456789").await,
            Err(LookupError::Http { status: s, retry_after: Some(RetryAfter::Seconds(120)) }) if s == status));
        assert_eq!(server.calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn http_server_and_gateway_failures_report_verification_unavailable() {
    for status in [408, 500, 502, 503, 504] {
        for body in ["", "<html>maintenance</html>", VALID] {
            let server =
                Server::new(http(status, body, "Retry-After: 120\r\n"), Duration::ZERO).await;
            let client = server.client();
            let error = client.is_vat_valid("BG123456789").await.unwrap_err();
            assert!(error.is_service_unavailable());
            if matches!(status, 408 | 504) {
                assert!(matches!(
                    error,
                    LookupError::ProviderTimeout {
                        retry_after: Some(RetryAfter::Seconds(120))
                    }
                ));
            } else {
                assert!(matches!(
                    error,
                    LookupError::ServiceUnavailable {
                        retry_after: Some(RetryAfter::Seconds(120))
                    }
                ));
            }
            assert!(matches!(
                client.lookup_vat("BG123456789").await,
                Err(LookupError::CoolingDown { .. })
            ));
            assert_eq!(server.calls.load(Ordering::SeqCst), 1);
        }
    }
}

#[tokio::test]
async fn error_body_bounds_cannot_hide_http_service_unavailability() {
    for status in [500, 502, 503, 504] {
        for response in [
            format!("HTTP/1.1 {status} Test\r\nContent-Length: 999999999\r\n\r\n").into_bytes(),
            format!("HTTP/1.1 {status} Test\r\nContent-Length: 100\r\n\r\nx").into_bytes(),
        ] {
            let server = Server::new(response, Duration::ZERO).await;
            assert!(
                server
                    .client()
                    .lookup_vat("BG123456789")
                    .await
                    .unwrap_err()
                    .is_service_unavailable()
            );
        }
    }
}

#[tokio::test]
async fn soap_faults_are_decoded_on_success_and_failure_http_statuses() {
    for status in [200, 500] {
        let server = Server::new(
            http(
                status,
                &fault("GLOBAL_MAX_CONCURRENT_REQ"),
                "Retry-After: 75\r\n",
            ),
            Duration::ZERO,
        )
        .await;
        assert!(matches!(
            server.client().lookup_vat("BG123456789").await,
            Err(LookupError::RateLimited {
                scope: LimitScope::Global,
                retry_after: Some(RetryAfter::Seconds(75))
            })
        ));
    }
}

#[tokio::test]
async fn rate_limit_establishes_shared_global_cooldown_without_retry() {
    let server = Server::new(
        http(429, "private provider message", "Retry-After: 120\r\n"),
        Duration::ZERO,
    )
    .await;
    let client = server.client();
    assert!(matches!(
        client.lookup_vat("BG123456789").await,
        Err(LookupError::RateLimited { .. })
    ));
    assert!(matches!(
        client.clone().lookup_vat("EL123456789").await,
        Err(LookupError::CoolingDown {
            scope: LimitScope::Global,
            retry_after_seconds: 119..=120
        })
    ));
    assert_eq!(server.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn country_cooldown_does_not_block_other_countries() {
    let server = Server::new(
        http(500, &fault("MS_MAX_CONCURRENT_REQ"), ""),
        Duration::ZERO,
    )
    .await;
    let client = server.client();
    assert!(matches!(
        client.lookup_vat("BG123456789").await,
        Err(LookupError::RateLimited {
            scope: LimitScope::MemberState,
            ..
        })
    ));
    assert!(matches!(
        client.clone().lookup_vat("BG123456789").await,
        Err(LookupError::CoolingDown {
            scope: LimitScope::MemberState,
            ..
        })
    ));
    assert!(matches!(
        client.lookup_vat("EL123456789").await,
        Err(LookupError::RateLimited { .. })
    ));
    assert_eq!(server.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn limits_apply_to_known_length_and_streamed_bodies() {
    let responses = [
        b"HTTP/1.1 200 OK\r\nContent-Length: 9999999\r\n\r\n".to_vec(),
        format!(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{}\r\n0\r\n\r\n",
            4096,
            "x".repeat(4096)
        )
        .into_bytes(),
    ];
    for response in responses {
        let server = Server::new(response, Duration::ZERO).await;
        let client = Client::for_test(
            server.url.clone(),
            ClientConfig {
                max_response_bytes: 1024,
                ..config()
            },
        );
        assert!(matches!(
            client.lookup_vat("BG123456789").await,
            Err(LookupError::ResponseTooLarge)
        ));
    }
    let server = Server::new(http(200, VALID, ""), Duration::ZERO).await;
    let client = Client::for_test(
        server.url.clone(),
        ClientConfig {
            max_response_bytes: VALID.len(),
            ..config()
        },
    );
    assert!(client.lookup_vat("BG123456789").await.is_ok());
}

#[tokio::test]
async fn malformed_utf8_xml_and_truncated_body_are_failures() {
    let mut invalid_utf8 = b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\n".to_vec();
    invalid_utf8.push(255);
    for response in [invalid_utf8, http(200, "<html>maintenance</html>", "")] {
        let server = Server::new(response, Duration::ZERO).await;
        assert!(matches!(
            server.client().lookup_vat("BG123456789").await,
            Err(LookupError::MalformedResponse)
        ));
    }
    let server = Server::new(
        b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nx".to_vec(),
        Duration::ZERO,
    )
    .await;
    assert!(matches!(
        server.client().lookup_vat("BG123456789").await,
        Err(LookupError::Transport)
    ));
}

#[tokio::test]
async fn deadline_includes_network_wait() {
    let server = Server::new(http(200, VALID, ""), Duration::from_secs(5)).await;
    let client = Client::for_test(
        server.url.clone(),
        ClientConfig {
            timeout: Duration::from_millis(80),
            connect_timeout: Duration::from_millis(40),
            ..config()
        },
    );
    assert!(matches!(
        client.lookup_vat("BG123456789").await,
        Err(LookupError::DeadlineExceeded)
    ));
    assert_eq!(server.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn unavailable_local_start_slot_returns_busy_without_claiming_an_outage() {
    for country_spacing in [false, true] {
        let server = Server::new(http(200, VALID, ""), Duration::ZERO).await;
        let client = Client::for_test(
            server.url.clone(),
            ClientConfig {
                timeout: Duration::from_millis(400),
                connect_timeout: Duration::from_millis(200),
                global_start_interval: if country_spacing {
                    Duration::ZERO
                } else {
                    Duration::from_secs(5)
                },
                country_start_interval: if country_spacing {
                    Duration::from_secs(5)
                } else {
                    Duration::ZERO
                },
                ..config()
            },
        );
        client.lookup_vat("BG123456789").await.unwrap();
        let next = if country_spacing {
            "BG123456789"
        } else {
            "EL123456789"
        };
        let start = Instant::now();
        assert!(matches!(
            client.clone().lookup_vat(next).await,
            Err(LookupError::Busy)
        ));
        assert!(start.elapsed() < Duration::from_secs(2));
        assert_eq!(server.calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn clones_share_capacity_and_cancellation_releases_slots() {
    let server = Server::new(http(200, VALID, ""), Duration::from_secs(5)).await;
    let client = server.client();
    let clone = client.clone();
    let first = tokio::spawn(async move { clone.lookup_vat("BG123456789").await });
    server.wait_calls(1).await;
    assert!(matches!(
        client.lookup_vat("BG987654321").await,
        Err(LookupError::Busy)
    ));
    let clone = client.clone();
    let second = tokio::spawn(async move { clone.lookup_vat("EL123456789").await });
    server.wait_calls(2).await;
    assert!(matches!(
        client.lookup_vat("RO12").await,
        Err(LookupError::Busy)
    ));
    first.abort();
    second.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    assert!(second.await.unwrap_err().is_cancelled());
    let clone = client.clone();
    let third = tokio::spawn(async move { clone.lookup_vat("BG123456789").await });
    server.wait_calls(3).await;
    third.abort();
    assert!(third.await.unwrap_err().is_cancelled());
}

#[tokio::test]
async fn spacing_is_enforced_on_successive_calls() {
    let server = Server::new(http(200, VALID, ""), Duration::ZERO).await;
    let client = Client::for_test(
        server.url.clone(),
        ClientConfig {
            country_start_interval: Duration::from_millis(100),
            ..config()
        },
    );
    let start = Instant::now();
    client.lookup_vat("BG123456789").await.unwrap();
    client.lookup_vat("BG123456789").await.unwrap();
    assert!(start.elapsed() >= Duration::from_millis(100));
}

#[tokio::test]
async fn unavailable_faults_preserve_retry_after_and_apply_the_correct_scope() {
    for code in ["SERVICE_UNAVAILABLE", "MS_UNAVAILABLE", "TIMEOUT"] {
        let server = Server::new(
            http(503, &fault(code), "Retry-After: 180\r\n"),
            Duration::ZERO,
        )
        .await;
        let client = server.client();
        let error = client.lookup_vat("BG123456789").await.unwrap_err();
        assert!(matches!(
            error,
            LookupError::ServiceUnavailable {
                retry_after: Some(RetryAfter::Seconds(180))
            } | LookupError::MemberStateUnavailable {
                retry_after: Some(RetryAfter::Seconds(180))
            } | LookupError::ProviderTimeout {
                retry_after: Some(RetryAfter::Seconds(180))
            }
        ));
        let scope = if code == "SERVICE_UNAVAILABLE" {
            LimitScope::Global
        } else {
            LimitScope::MemberState
        };
        assert!(matches!(client.lookup_vat("BG123456789").await,
            Err(LookupError::CoolingDown { scope: s, retry_after_seconds: 179..=180 }) if s == scope));
        assert_eq!(server.calls.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn an_inflight_rate_limit_stops_a_call_waiting_for_global_spacing() {
    let server = Server::new(http(429, "", ""), Duration::from_millis(80)).await;
    let client = Client::for_test(
        server.url.clone(),
        ClientConfig {
            global_start_interval: Duration::from_millis(250),
            ..config()
        },
    );
    let clone = client.clone();
    let first = tokio::spawn(async move { clone.lookup_vat("BG123456789").await });
    server.wait_calls(1).await;
    assert!(matches!(
        client.lookup_vat("EL123456789").await,
        Err(LookupError::CoolingDown {
            scope: LimitScope::Global,
            ..
        })
    ));
    assert!(matches!(
        first.await.unwrap(),
        Err(LookupError::RateLimited { .. })
    ));
    assert_eq!(server.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn stalled_body_is_bounded_by_the_total_deadline() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/vies", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0; 1];
        socket.read_exact(&mut request).await.unwrap();
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4096\r\n\r\n<")
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_secs(10)).await;
    });
    let client = Client::for_test(
        url,
        ClientConfig {
            timeout: Duration::from_millis(150),
            connect_timeout: Duration::from_millis(100),
            ..config()
        },
    );
    assert!(matches!(
        client.lookup_vat("BG123456789").await,
        Err(LookupError::DeadlineExceeded)
    ));
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn closed_connection_is_a_sanitized_transport_error() {
    // Accept and read the request, then close without any response. A closed
    // port can instead hit the connection deadline on some operating systems.
    let server = Server::new(Vec::new(), Duration::ZERO).await;
    let client = server.client();
    let error = client.lookup_vat("BG123456789").await.unwrap_err();
    assert!(matches!(error, LookupError::Transport));
    assert!(!format!("{error:?} {error}").contains("123456789"));
    assert_eq!(server.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn retry_after_parses_seconds_and_dates_without_accepting_garbage() {
    assert_eq!(parse_retry_after(" 120 "), Some(RetryAfter::Seconds(120)));
    assert!(matches!(
        parse_retry_after("Sun, 06 Nov 1994 08:49:37 GMT"),
        Some(RetryAfter::Date(_))
    ));
    for value in [
        "",
        "-1",
        "+1",
        "1.5",
        "private arbitrary text",
        "18446744073709551616",
    ] {
        assert!(parse_retry_after(value).is_none());
    }
}

#[tokio::test]
async fn extreme_retry_after_does_not_overflow_cooldown_arithmetic() {
    let server = Server::new(
        http(429, "", "Retry-After: 18446744073709551615\r\n"),
        Duration::ZERO,
    )
    .await;
    let client = server.client();
    assert!(matches!(
        client.lookup_vat("BG123456789").await,
        Err(LookupError::RateLimited {
            retry_after: Some(RetryAfter::Seconds(u64::MAX)),
            ..
        })
    ));
    assert!(matches!(
        client.lookup_vat("BG123456789").await,
        Err(LookupError::CoolingDown { .. })
    ));
}

#[test]
fn invalid_configuration_fails_before_any_lookup() {
    for config in [
        ClientConfig {
            timeout: Duration::ZERO,
            ..Default::default()
        },
        ClientConfig {
            max_concurrent_requests: 0,
            ..Default::default()
        },
        ClientConfig {
            max_concurrent_requests: usize::MAX,
            ..Default::default()
        },
        ClientConfig {
            max_response_bytes: 0,
            ..Default::default()
        },
        ClientConfig {
            failure_cooldown: Duration::ZERO,
            ..Default::default()
        },
        ClientConfig {
            global_start_interval: Duration::MAX,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            Client::with_config(config),
            Err(LookupError::InvalidConfiguration(_))
        ));
    }
}

#[tokio::test]
async fn compact_response_has_exact_keys_and_structured_address() {
    let raw = "ул. ПРИМЕР 1 обл.РУСЕ, гр.РУСЕ 7000";
    let xml = VALID.replace("Примерна улица 1\nСофия 1000", raw);
    let server = Server::new(http(200, &xml, ""), Duration::ZERO).await;
    let result = server.client().lookup_vat("BG123456789").await.unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::json!({
            "vatId": "BG123456789", "valid": true, "source": "Vies",
            "company": " Синтетична & Δοκιμή SRL ",
            "address": { "fullAddressByVies": raw, "address": "ул. ПРИМЕР 1", "address2": null,
                "city": "Русе", "postcode": "7000", "region": "Русе", "country": "България", "countryISO2": "BG" }
        })
    );
}

#[tokio::test]
async fn missing_placeholder_and_negative_addresses_preserve_source_data() {
    for (element, expected) in [
        ("", None),
        ("<address/>", Some("")),
        ("<address> --- </address>", Some(" --- ")),
    ] {
        let xml = VALID
            .replace("<address>Примерна улица 1\nСофия 1000</address>", element)
            .replace("<valid>true", "<valid>false");
        let server = Server::new(http(200, &xml, ""), Duration::ZERO).await;
        let result = server.client().lookup_vat("BG123456789").await.unwrap();
        assert!(!result.valid);
        assert_eq!(result.address.full_address_by_vies.as_deref(), expected);
        assert!(result.address.address.is_none() && result.address.city.is_none());
        assert!(result.company.is_some());
    }
}

#[tokio::test]
async fn invalid_utf8_error_page_preserves_http_failure_classification() {
    let mut reply = b"HTTP/1.1 400 Bad Request\r\nContent-Length: 1\r\nRetry-After: 120\r\nConnection: close\r\n\r\n".to_vec();
    reply.push(255);
    let server = Server::new(reply, Duration::ZERO).await;
    assert!(matches!(
        server.client().lookup_vat("BG123456789").await,
        Err(LookupError::Http {
            status: 400,
            retry_after: Some(RetryAfter::Seconds(120))
        })
    ));
}
