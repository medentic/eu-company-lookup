//! Durable batch journal; transport/retry policy remains in the parent module.
use super::*;
use std::collections::HashSet;

struct Lock;
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file("target/vies-probe.lock");
    }
}

fn journal(text: &str, ids: &[crate::VatId]) -> Vec<Observation> {
    let selected: HashSet<_> = ids.iter().map(ToString::to_string).collect();
    assert_eq!(selected.len(), ids.len(), "duplicate selected IDs");
    assert!(
        text.is_empty() || text.ends_with('\n'),
        "partial journal tail: preserve and inspect before recovery"
    );
    let mut seen = HashSet::new();
    text.lines()
        .map(|line| {
            let row = Observation::from_json(line).expect("invalid journal");
            let id = row.vat_id.to_string();
            assert!(selected.contains(&id), "journal ID outside selected batch");
            assert!(seen.insert(id), "duplicate journal ID");
            row
        })
        .collect()
}

fn next_allowed(row: &Observation) -> Result<chrono::DateTime<chrono::Utc>, &'static str> {
    let at = row.checked_at;
    let last = row.last();
    let mut until = at + chrono::Duration::seconds(if last.success() { 5 } else { 60 });
    if let Some(header) = last.retry_after.as_deref() {
        let retry = if let Ok(seconds) = header.parse::<i64>() {
            if seconds < 0 {
                return Err("negative Retry-After");
            }
            at.checked_add_signed(
                chrono::Duration::try_seconds(seconds).ok_or("Retry-After overflow")?,
            )
            .ok_or("Retry-After overflow")?
        } else {
            chrono::DateTime::<chrono::Utc>::from(
                httpdate::parse_http_date(header)
                    .map_err(|_| "invalid Retry-After; manual review required")?,
            )
        };
        until = until.max(retry);
    }
    Ok(until)
}

#[tokio::test]
#[ignore = "explicit live batch; use tools/vies_batch.py with a fixed test binary"]
async fn live_six_second_probe() {
    live_batch(Duration::from_secs(6)).await;
}

#[tokio::test]
#[ignore = "explicit bounded recovery; select the 25-second collector profile"]
async fn live_extended_probe() {
    live_batch(Duration::from_secs(25)).await;
}

async fn live_batch(budget: Duration) {
    let ids: Vec<_> = std::env::var("VIES_PROBE_IDS")
        .unwrap()
        .split(',')
        .map(|s| crate::VatId::parse(s).unwrap())
        .collect();
    assert!(!ids.is_empty() && ids.len() <= 100);
    std::fs::create_dir_all("target").unwrap();
    let mut lock = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open("target/vies-probe.lock")
        .expect("another probe or stale lock; inspect before removing");
    let _guard = Lock;
    writeln!(lock, "{}", std::process::id()).unwrap();
    let path = std::env::var("VIES_PROBE_REPORT").unwrap();
    let resume = std::env::var_os("VIES_PROBE_RESUME").is_some();
    let old = if resume {
        std::fs::read_to_string(&path).unwrap_or_else(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                String::new()
            } else {
                panic!("{e}")
            }
        })
    } else {
        String::new()
    };
    let mut rows = journal(&old, &ids);
    let done: HashSet<_> = rows.iter().map(|r| r.vat_id.to_string()).collect();
    let pending: Vec<_> = ids
        .iter()
        .filter(|id| !done.contains(&id.to_string()))
        .collect();
    let mut options = std::fs::OpenOptions::new();
    options.append(true);
    if resume {
        options.create(true);
    } else {
        options.create_new(true);
    }
    let mut log = options.open(&path).unwrap();
    let http = transport::client(Duration::from_secs(3), None).unwrap();
    for vat in pending {
        // Reserve room for the bounded responses of all three attempts.
        if log.metadata().unwrap().len() > 18 * 1024 * 1024 {
            break; // Incomplete status is emitted below; keep existing evidence intact.
        }
        if let Some(last) = rows.last() {
            let until = next_allowed(last).expect("provider cooldown requires review");
            // A restarted process cannot bypass the previous lookup's cooldown.
            if let Ok(wait) = (until - chrono::Utc::now()).to_std() {
                sleep(wait).await;
            }
        }
        let result = Observation {
            schema_version: SCHEMA_VERSION,
            vat_id: vat.clone(),
            checked_at: chrono::Utc::now(),
            run: experiment_with_budget(|| request(&http, vat), budget).await,
        };
        // Timestamp the completed lookup for cooldown accounting.
        let result = Observation {
            checked_at: chrono::Utc::now(),
            ..result
        };
        writeln!(log, "{}", serde_json::to_string(&result).unwrap()).unwrap();
        log.flush().unwrap();
        log.sync_data().unwrap();
        let last = result.last();
        let deferred = last.retry_after.is_some();
        eprintln!(
            "{}: {}ms; response={}",
            vat,
            result.run.total_ms,
            last.success()
        );
        rows.push(result);
        if deferred {
            break;
        }
    }
    let errors = rows.iter().filter(|r| !r.last().success()).count();
    eprintln!(
        "selected={}, completed={}, pending={}, providerErrors={errors}",
        ids.len(),
        rows.len(),
        ids.len() - rows.len()
    );
    assert_eq!(
        rows.len(),
        ids.len(),
        "INCOMPLETE: provider deferral or log-size budget; inspect report before resume"
    );
}

#[test]
fn resume_rejects_duplicates_foreign_ids_and_partial_tail() {
    let ids = vec![crate::VatId::parse("BG123456789").unwrap()];
    let line = "{\"vatId\":\"BG123456789\",\"checkedAt\":\"2026-11-13T10:00:00Z\",\"totalMs\":1,\"attempts\":[{\"attempt\":1,\"startedAtMs\":0,\"durationMs\":1,\"error\":\"timeout\"}]}\n";
    assert_eq!(journal(line, &ids).len(), 1); // Errors are completed attempts, not retried by resume.
    for invalid in [
        line.repeat(2),
        line.trim_end().into(),
        line.replace("123456789", "987654321"),
    ] {
        assert!(std::panic::catch_unwind(|| journal(&invalid, &ids)).is_err());
    }
    assert!(std::panic::catch_unwind(|| journal("", &[ids[0].clone(), ids[0].clone()])).is_err());
}

#[test]
fn resume_preserves_cooldown_and_retry_after() {
    let row = Observation {
        schema_version: SCHEMA_VERSION,
        vat_id: crate::VatId::parse("BG123456789").unwrap(),
        checked_at: "2026-11-13T10:00:00Z".parse().unwrap(),
        run: Run {
            total_ms: 1,
            attempts: vec![Attempt {
                retry_after: Some("120".into()),
                ..Attempt::failure(AttemptError::BodyReadError)
            }],
        },
    };
    assert_eq!(
        next_allowed(&row).unwrap().to_rfc3339(),
        "2026-11-13T10:02:00+00:00"
    );
    let mut row = row;
    row.run.attempts[0].retry_after = Some("Fri, 13 Nov 2026 10:03:00 GMT".into());
    assert_eq!(
        next_allowed(&row).unwrap().to_rfc3339(),
        "2026-11-13T10:03:00+00:00"
    );
    row.run.attempts[0].retry_after = Some("garbage".into());
    assert!(next_allowed(&row).is_err());
}
