//! Opt-in transport experiment; does not alter the production client's retry policy.
use super::super::{address, soap, transport};
use std::{future::Future, io::Write, time::Duration};
use tokio::time::{Instant, sleep, timeout_at};
mod records;
use records::{
    Attempt, AttemptError, Observation, ObservedCompany, Outcome, Run, SCHEMA_VERSION, Source,
};

async fn experiment<F, Fut>(call: F) -> Run
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Attempt>,
{
    experiment_with_budget(call, Duration::from_secs(6)).await
}

async fn experiment_with_budget<F, Fut>(mut call: F, budget: Duration) -> Run
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Attempt>,
{
    let start = Instant::now();
    let deadline = start + budget;
    let mut attempts = Vec::new();
    for index in 0..3 {
        if index == 2 {
            if Instant::now() + Duration::from_secs(1) >= deadline {
                break;
            }
            sleep(Duration::from_secs(1)).await;
        }
        if Instant::now() >= deadline {
            break;
        }
        let began = Instant::now();
        let mut row = timeout_at(deadline, call())
            .await
            .unwrap_or_else(|_| Attempt::failure(AttemptError::TotalDeadlineExceeded));
        row.attempt = index + 1;
        row.started_at_ms = began.duration_since(start).as_millis() as u64;
        row.duration_ms = began.elapsed().as_millis() as u64;
        // Any Retry-After header defers to the provider rather than an immediate retry.
        let retry = row.retryable();
        attempts.push(row);
        if !retry {
            break;
        }
    }
    Run {
        total_ms: start.elapsed().as_millis() as u64,
        attempts,
    }
}

async fn request(http: &reqwest::Client, vat: &crate::VatId) -> Attempt {
    request_to(http, vat, transport::ENDPOINT).await
}

async fn request_to(http: &reqwest::Client, vat: &crate::VatId, endpoint: &str) -> Attempt {
    let response = match transport::exchange(http, endpoint, vat, 65_536).await {
        Ok(response) => response,
        Err(transport::Failure::Timeout) => {
            return Attempt::failure(AttemptError::TransportTimeout);
        }
        Err(_) => return Attempt::failure(AttemptError::TransportError),
    };
    let mut row = Attempt::new(Outcome::Skipped { body_skipped: true });
    row.http_status = Some(response.status.as_u16());
    row.retry_after = response.retry_after;
    row.outcome = match response.body {
        transport::Body::Skipped => Outcome::Skipped { body_skipped: true },
        transport::Body::Failed(error) => Outcome::Failure {
            error: match error {
                transport::Failure::Timeout => AttemptError::TransportTimeout,
                transport::Failure::Network => AttemptError::BodyReadError,
                transport::Failure::TooLarge => AttemptError::BodyTooLarge,
                transport::Failure::InvalidUtf8 => AttemptError::InvalidUtf8,
            },
        },
        transport::Body::Text(raw) => {
            let decoded = soap::decode(&raw, vat);
            row.raw_body = Some(raw);
            match decoded {
                Ok(decoded) if response.status.is_success() => Outcome::Success {
                    response: Box::new(ObservedCompany {
                        vat_id: vat.clone(),
                        valid: decoded.valid,
                        source: Source::Vies,
                        company: decoded.company,
                        address: address::split(vat.prefix(), decoded.address).into(),
                    }),
                },
                Ok(_) => Outcome::Failure {
                    error: AttemptError::HttpError,
                },
                Err(error) => Outcome::Failure {
                    error: error.into(),
                },
            }
        }
    };
    row
}

mod batch;

#[tokio::test(start_paused = true)]
async fn retries_502_immediately_then_after_one_second() {
    let mut n = 0;
    let result = experiment(|| {
        n += 1;
        let status = if n < 3 { 502 } else { 200 };
        async move {
            sleep(Duration::from_millis(100)).await;
            Attempt::status(status)
        }
    })
    .await;
    assert_eq!(result.attempts.len(), 3);
    assert_eq!(result.attempts[1].started_at_ms, 100);
    assert_eq!(result.attempts[2].started_at_ms, 1200);
    assert_eq!(result.total_ms, 1300);
}
#[tokio::test(start_paused = true)]
async fn total_budget_cancels_slow_attempt() {
    let result = experiment(|| async {
        sleep(Duration::from_secs(20)).await;
        Attempt::status(502)
    })
    .await;
    assert_eq!(result.total_ms, 6000);
    assert_eq!(result.attempts.len(), 1);
    assert!(matches!(
        result.attempts[0].outcome,
        Outcome::Failure {
            error: AttemptError::TotalDeadlineExceeded
        }
    ));
}
#[tokio::test(start_paused = true)]
async fn no_retry_for_other_errors_or_provider_retry_after() {
    for row in [
        Attempt::status(429),
        Attempt::status(503),
        Attempt {
            retry_after: Some("60".into()),
            ..Attempt::status(502)
        },
        Attempt::failure(AttemptError::TransportError),
        Attempt::status(200),
    ] {
        let result = experiment(|| std::future::ready(row.clone())).await;
        assert_eq!(result.attempts.len(), 1);
    }
}
#[tokio::test(start_paused = true)]
async fn no_final_attempt_when_pause_would_exhaust_budget() {
    let result = experiment(|| async {
        sleep(Duration::from_millis(2600)).await;
        Attempt::status(502)
    })
    .await;
    assert_eq!(result.attempts.len(), 2);
    assert_eq!(result.total_ms, 5200);
}

#[tokio::test]
async fn preserves_retry_after_when_body_cannot_be_used() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    for (body, length, expected) in [
        (vec![255], 1, AttemptError::InvalidUtf8),
        (vec![b'x'; 65_537], 65_537, AttemptError::BodyTooLarge),
        (vec![b'x'], 30, AttemptError::BodyReadError),
    ] {
        let server = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", server.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (mut socket, _) = server.accept().await.unwrap();
            let mut buf = [0; 4096];
            let _ = socket.read(&mut buf).await.unwrap();
            socket.write_all(format!("HTTP/1.1 503 Service Unavailable\r\nContent-Length: {length}\r\nRetry-After: 120\r\nConnection: close\r\n\r\n").as_bytes()).await.unwrap();
            socket.write_all(&body).await.unwrap();
            socket.shutdown().await.unwrap();
        });
        let http = reqwest::Client::builder().no_proxy().build().unwrap();
        let row = request_to(
            &http,
            &crate::VatId::parse("BG123456789").unwrap(),
            &endpoint,
        )
        .await;
        task.await.unwrap();
        assert_eq!(row.http_status, Some(503));
        assert_eq!(row.retry_after.as_deref(), Some("120"));
        assert!(matches!(row.outcome, Outcome::Failure { error } if error == expected));
    }
}

#[tokio::test(start_paused = true)]
async fn extended_profile_is_bounded_and_accepts_a_slow_response() {
    let slow = experiment_with_budget(
        || async {
            sleep(Duration::from_secs(20)).await;
            Attempt::status(200)
        },
        Duration::from_secs(25),
    )
    .await;
    assert_eq!(slow.total_ms, 20_000);
    assert_eq!(slow.attempts.len(), 1);
    assert_eq!(slow.attempts[0].http_status, Some(200));
    let expired = experiment_with_budget(
        || async {
            sleep(Duration::from_secs(30)).await;
            Attempt::status(200)
        },
        Duration::from_secs(25),
    )
    .await;
    assert_eq!(expired.total_ms, 25_000);
    assert!(matches!(
        expired.attempts[0].outcome,
        Outcome::Failure {
            error: AttemptError::TotalDeadlineExceeded
        }
    ));
}
