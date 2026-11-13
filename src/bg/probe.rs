//! Explicit bounded Commercial Register observations, never part of default tests.
use super::{BgBusinessId, BgRegistryClient, commercial};
use crate::{BgCompanyLookupResult, LookupError, RegistryError};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, time::Instant};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
enum Outcome {
    Found,
    NotFoundInCommercialRegister,
    ParserError,
    ProviderError,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Observation {
    schema_version: u8,
    tax_id: String,
    checked_at: String,
    elapsed_ms: u128,
    scheduler_wait_ms: u128,
    transport: super::client::RequestTrace,
    error_kind: Option<&'static str>,
    unavailability_reason: Option<&'static str>,
    outcome: Outcome,
    raw_response: Option<String>,
    record: Option<BgCompanyLookupResult>,
    error: Option<String>,
    service_unavailable: bool,
    resume_not_before: Option<chrono::DateTime<chrono::Utc>>,
}

#[tokio::test]
#[ignore = "explicit Commercial Register collection; REGISTRY_RUN_DIR required"]
async fn live_commercial_batch() {
    let dir = PathBuf::from(std::env::var("REGISTRY_RUN_DIR").expect("REGISTRY_RUN_DIR"));
    let manifest: Value =
        serde_json::from_slice(&fs::read(dir.join("manifest.json")).unwrap()).unwrap();
    let ids = manifest["ids"].as_array().expect("manifest ids");
    assert!(
        !ids.is_empty() && ids.len() <= 100,
        "bounded batches of 1..100"
    );
    let interval = manifest["policy"]["startIntervalSeconds"]
        .as_u64()
        .expect("interval");
    assert!((10..=300).contains(&interval));
    let client = BgRegistryClient::new().unwrap();
    let provider_state =
        PathBuf::from(std::env::var("REGISTRY_PROVIDER_STATE").expect("provider state"));
    // Spacing is persisted across processes; the normal client's admission clock
    // still enforces its own default minimum.
    for id in ids {
        let id = BgBusinessId::parse(id.as_str().unwrap()).unwrap();
        let path = dir.join("observations").join(format!("{id}.json"));
        if path.exists() {
            continue;
        }
        let scheduled = Instant::now();
        let state: Value = serde_json::from_slice(&fs::read(&provider_state).unwrap()).unwrap();
        if state["cooldownUntil"]
            .as_str()
            .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok())
            .is_some_and(|t| t > chrono::Utc::now())
        {
            break;
        }
        if let Some(next) = state["nextStartAt"]
            .as_str()
            .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok())
            && let Ok(wait) = (next - chrono::Utc::now()).to_std()
        {
            tokio::time::sleep(wait).await;
        }
        let request_start = chrono::Utc::now();
        // Conservative reservation survives process termination during a request.
        write_json(
            &provider_state,
            &serde_json::json!({"nextStartAt":request_start + chrono::Duration::seconds(30 + interval as i64),"cooldownUntil":null}),
        );
        let scheduler_wait_ms = scheduled.elapsed().as_millis();
        let started = Instant::now();
        let mut row = Observation {
            schema_version: 2,
            tax_id: id.to_string(),
            checked_at: chrono::Utc::now().to_rfc3339(),
            elapsed_ms: 0,
            scheduler_wait_ms,
            transport: Default::default(),
            error_kind: None,
            unavailability_reason: None,
            outcome: Outcome::ProviderError,
            raw_response: None,
            record: None,
            error: None,
            service_unavailable: false,
            resume_not_before: None,
        };
        match client.observe_commercial(id.as_str()).await {
            Ok(body) => {
                row.raw_response = String::from_utf8(body.clone()).ok();
                if body.is_empty() {
                    row.outcome = Outcome::NotFoundInCommercialRegister;
                } else {
                    match commercial::decode(&body, &id) {
                        Ok(record) => {
                            row.outcome = Outcome::Found;
                            row.record = Some(record);
                        }
                        Err(error) => {
                            row.outcome = Outcome::ParserError;
                            row.error_kind = Some("parserError");
                            row.error = Some(error.to_string());
                        }
                    }
                }
            }
            Err(error) => {
                row.error_kind = Some(error_kind(&error));
                if let LookupError::Registry {
                    error: RegistryError::ServiceUnavailable { reason, .. },
                    ..
                } = &error
                {
                    row.unavailability_reason = Some(match reason {
                        crate::UnavailabilityReason::Timeout => "timeout",
                        crate::UnavailabilityReason::Connection => "connection",
                        crate::UnavailabilityReason::ServerError => "serverError",
                    });
                }
                row.service_unavailable = error.is_service_unavailable();
                let now = chrono::Utc::now();
                let minimum = now + chrono::Duration::seconds(60);
                let retry_after = match &error {
                    crate::LookupError::Registry {
                        error:
                            crate::RegistryError::RateLimited { retry_after }
                            | crate::RegistryError::ServiceUnavailable { retry_after, .. },
                        ..
                    } => retry_after.as_ref(),
                    _ => None,
                };
                row.resume_not_before = Some(match retry_after {
                    Some(crate::RetryAfter::Seconds(seconds)) => (now
                        + chrono::Duration::seconds((*seconds).min(315_360_000) as i64))
                    .max(minimum),
                    Some(crate::RetryAfter::Date(date)) => (*date).max(minimum),
                    None => minimum,
                });
                row.error = Some(error.to_string());
            }
        }
        row.elapsed_ms = started.elapsed().as_millis();
        row.transport = client.observation_trace();
        write_json(
            &provider_state,
            &serde_json::json!({"nextStartAt":request_start + chrono::Duration::seconds(interval as i64),"cooldownUntil":row.resume_not_before}),
        );
        let stop = matches!(row.outcome, Outcome::ProviderError);
        write_json(&path, &serde_json::to_value(&row).unwrap());
        eprintln!(
            "{}: {} ({} ms)",
            id,
            serde_json::to_value(&row.outcome).unwrap(),
            row.elapsed_ms
        );
        // Stop on service/access/rate failures, preserving unattempted IDs for resume.
        if stop {
            break;
        }
    }
}

#[test]
#[ignore = "offline replay of captured Commercial Register observations"]
fn replay_commercial_batch() {
    let dir = PathBuf::from(std::env::var("REGISTRY_RUN_DIR").unwrap());
    let mut rows = Vec::new();
    for entry in fs::read_dir(dir.join("observations")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|s| s != "json") {
            continue;
        }
        let row: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let Some(raw) = row["rawResponse"].as_str().filter(|s| !s.is_empty()) else {
            continue;
        };
        let id = BgBusinessId::parse(row["taxId"].as_str().unwrap()).unwrap();
        let mut replay = serde_json::json!({"observationSha256": source_sha(&fs::read(&path).unwrap()), "sourceSha256":source_sha(raw.as_bytes()), "taxId":id,"rawResponse":raw,"record":null,"outcome":"parserError","error":null});
        match commercial::decode(raw.as_bytes(), &id) {
            Ok(record) => {
                replay["record"] = serde_json::to_value(record).unwrap();
                replay["outcome"] = Value::String("found".into());
            }
            Err(error) => replay["error"] = Value::String(error.to_string()),
        }
        rows.push(replay);
    }
    assert!(!rows.is_empty());
    rows.sort_by(|a, b| a["taxId"].as_str().cmp(&b["taxId"].as_str()));
    let replay = serde_json::json!({"schemaVersion":2,"generatedAt":chrono::Utc::now(),"crateVersion":env!("CARGO_PKG_VERSION"),"parserSha256":parser_fingerprint(),"binarySha256":source_sha(&fs::read(std::env::current_exe().unwrap()).unwrap()),"observations":rows});
    write_json(&dir.join("replay.json"), &replay);
    eprintln!(
        "Replayed {} saved registry responses; review expectations remain separate",
        rows.len()
    );
}

pub(super) fn source_sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn parser_fingerprint() -> String {
    source_sha(
        concat!(
            include_str!("commercial.rs"),
            include_str!("address.rs"),
            include_str!("html.rs"),
            include_str!("legal_form.rs"),
            include_str!("../model/company.rs"),
            include_str!("../model/company_types.rs"),
            include_str!("../locations/normalize.rs"),
            include_str!("../locations/delivery.rs"),
            include_str!("../locations/country.rs"),
            include_str!("../locations/address.rs"),
            include_str!("../../Cargo.toml"),
            include_str!("../../Cargo.lock")
        )
        .as_bytes(),
    )
}
pub(super) fn write_json(path: &std::path::Path, value: &Value) {
    use std::io::Write;
    let temp = path.with_extension("tmp");
    let mut file = fs::File::create(&temp).unwrap();
    file.write_all(&serde_json::to_vec_pretty(value).unwrap())
        .unwrap();
    file.sync_all().unwrap();
    fs::rename(temp, path).unwrap();
}
pub(super) fn error_kind(error: &LookupError) -> &'static str {
    match error {
        LookupError::Registry { error, .. } => match error {
            RegistryError::RateLimited { .. } => "rateLimited",
            RegistryError::ServiceUnavailable { .. } => "serviceUnavailable",
            RegistryError::CoolingDown { .. } => "coolingDown",
            RegistryError::AccessDenied => "accessDenied",
            RegistryError::Http { .. } => "httpError",
            RegistryError::UnexpectedResponse => "unexpectedResponse",
            RegistryError::MismatchedResponse => "mismatchedResponse",
            RegistryError::ResponseTooLarge => "responseTooLarge",
        },
        LookupError::Busy => "busy",
        LookupError::InvalidInput => "invalidInput",
        _ => "lookupError",
    }
}
