use crate::http_capture::*;
use crate::{ErrorId, RegistryFailure};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn endpoint(response: Vec<u8>, stall: bool) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/path?q=private", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut buf = [0; 4096];
        let _ = socket.read(&mut buf).await.unwrap();
        socket.write_all(&response).await.unwrap();
        if stall {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });
    (url, task)
}
async fn fail(url: &str, raw: bool) -> LookupError {
    scope(
        raw,
        1024,
        std::time::Duration::from_secs(1),
        Provider::IrishCro,
        async {
            let response = reqwest::Client::new()
                .get(url)
                .header("x-repeat", "one")
                .header("x-repeat", "two")
                .send_traced()
                .await
                .map_err(|_| LookupError::from(crate::model::LookupFailure::Transport))?;
            crate::registry_transport::check_status(response.status().as_u16(), None, &[200])
                .map_err(|error| crate::model::LookupFailure::Registry {
                    provider: Provider::IrishCro,
                    error,
                })?;
            Err::<(), _>(
                crate::model::LookupFailure::Registry {
                    provider: Provider::IrishCro,
                    error: RegistryFailure::UnexpectedResponse,
                }
                .into(),
            )
        },
    )
    .await
    .unwrap_err()
}
#[tokio::test]
async fn concurrent_captures_are_isolated_and_repeated_request_headers_survive() {
    let (a, ta) = endpoint(
        b"HTTP/1.1 503 Error\r\nContent-Length: 3\r\n\r\none".to_vec(),
        false,
    )
    .await;
    let (b, tb) = endpoint(
        b"HTTP/1.1 403 Error\r\nContent-Length: 3\r\n\r\ntwo".to_vec(),
        false,
    )
    .await;
    let (ea, eb) = tokio::join!(fail(&a, true), fail(&b, true));
    assert_eq!(ea.id, ErrorId::ServiceUnavailable);
    assert_eq!(eb.id, ErrorId::AccessDenied);
    for (e, url, body) in [(ea, a, "one"), (eb, b, "two")] {
        let debug = e.details.debug.unwrap();
        assert_eq!(debug.exchanges.len(), 1);
        assert_eq!(debug.exchanges[0].request.url, url);
        assert_eq!(
            debug.exchanges[0].response.as_ref().unwrap().body.data,
            body
        );
        assert_eq!(
            debug.exchanges[0]
                .request
                .headers
                .iter()
                .filter(|h| h.name == "x-repeat")
                .count(),
            2
        );
    }
    ta.await.unwrap();
    tb.await.unwrap();
}
#[tokio::test]
async fn timeout_preserves_partial_body_and_authoritative_status() {
    let (url, task) = endpoint(
        b"HTTP/1.1 429 Error\r\nRetry-After: 19\r\nContent-Length: 100\r\n\r\npartial".to_vec(),
        true,
    )
    .await;
    let error = scope(
        true,
        1024,
        std::time::Duration::from_millis(150),
        Provider::IrishCro,
        async {
            let response = reqwest::Client::new().get(url).send_traced().await.unwrap();
            crate::registry_transport::check_status(
                response.status().as_u16(),
                crate::registry_transport::retry_after(&response),
                &[200],
            )
            .map_err(|error| {
                LookupError::from(crate::model::LookupFailure::Registry {
                    provider: Provider::IrishCro,
                    error,
                })
            })
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error.id, ErrorId::RateLimited);
    assert!(!error.is_service_unavailable());
    assert_eq!(
        error.details.retry_after,
        Some(crate::RetryAfter::Seconds(19))
    );
    let debug = error.details.debug.unwrap();
    let response = debug.exchanges[0].response.as_ref().unwrap();
    assert_eq!(response.body.data, "partial");
    assert!(!response.complete);
    assert_eq!(response.incomplete_reason, Some("timeout"));
    task.abort();
}
#[tokio::test]
async fn connection_failure_retains_request_without_a_response() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let error = fail(&url, true).await;
    assert_eq!(error.id, ErrorId::ServiceUnavailable);
    assert!(error.details.upstream_http_status.is_none());
    assert!(error.details.debug.unwrap().exchanges[0].response.is_none());
}

#[test]
fn curl_quoting_and_binary_headers_never_silently_change_values() {
    let request = reqwest::Client::new()
        .post("https://example.invalid/")
        .header("x-raw", "a'b;$HOME")
        .body(vec![0xff, 0, 0x27])
        .build()
        .unwrap();
    let capture = request_capture(&request);
    assert_eq!(capture.body.as_ref().unwrap().encoding, "base64");
    assert!(
        capture
            .curl
            .as_ref()
            .unwrap()
            .contains("'x-raw: a'\"'\"'b;$HOME'")
    );
    let request = reqwest::Client::new()
        .get("https://example.invalid/")
        .header(
            "x-binary",
            reqwest::header::HeaderValue::from_bytes(&[0xff]).unwrap(),
        )
        .build()
        .unwrap();
    let capture = request_capture(&request);
    assert_eq!(capture.headers[0].value.encoding, "base64");
    assert_eq!(capture.headers[0].value.data, "/w==");
    assert!(capture.curl.is_none());
    assert_eq!(
        capture.curl_unavailable_reason,
        Some("header_not_representable_in_shell")
    );
}

#[tokio::test]
async fn multi_request_capture_keeps_order_and_does_not_reuse_previous_status() {
    let (url, task) = endpoint(
        b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nfirst".to_vec(),
        false,
    )
    .await;
    let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let second = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let error = scope(
        true,
        1024,
        std::time::Duration::from_millis(150),
        Provider::IrishCro,
        async {
            let client = reqwest::Client::new();
            let mut first = client.get(&url).send_traced().await.unwrap();
            while first.chunk().await.unwrap().is_some() {}
            drop(first);
            client
                .get(&second)
                .send_traced()
                .await
                .map_err(|_| LookupError::from(crate::model::LookupFailure::Transport))?;
            Ok(())
        },
    )
    .await
    .unwrap_err();
    assert!(error.details.upstream_http_status.is_none());
    let debug = error.details.debug.unwrap();
    assert_eq!(debug.exchanges.len(), 2);
    assert_eq!(debug.exchanges[0].request.url, url);
    assert_eq!(
        debug.exchanges[0].response.as_ref().unwrap().body.data,
        "first"
    );
    assert!(debug.exchanges[1].response.is_none());
    task.await.unwrap();
}
