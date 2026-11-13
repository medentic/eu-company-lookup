//! Explicitly selected local-only observations; never part of the ordinary tests.
use super::*;
use chrono::{NaiveDate, Utc};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, io::Write, path::PathBuf, time::Duration};

#[tokio::test]
#[ignore = "authorized bounded ANAF collection only; use tools/ro_batch.py"]
async fn live_anaf_batch() {
    let folder = PathBuf::from(std::env::var("RO_RUN_DIR").unwrap());
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(folder.join("manifest.json")).unwrap()).unwrap();
    let ids: Vec<String> = serde_json::from_value(manifest["ids"].clone()).unwrap();
    assert!(!ids.is_empty() && ids.len() <= 100);
    assert_eq!(ids.iter().collect::<HashSet<_>>().len(), ids.len());
    let date: NaiveDate = manifest["requestDate"].as_str().unwrap().parse().unwrap();
    let journal = folder.join("observations.jsonl");
    let prior = std::fs::read_to_string(&journal).unwrap_or_else(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            String::new()
        } else {
            panic!("cannot read existing journal: {e}")
        }
    });
    assert!(
        prior.is_empty() || prior.ends_with('\n'),
        "partial journal must be preserved"
    );
    let mut seen = HashSet::new();
    let mut last: Option<Value> = None;
    for line in prior.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let id = row["taxId"].as_str().unwrap().to_owned();
        assert!(ids.contains(&id) && seen.insert(id));
        last = Some(row);
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(journal)
        .unwrap();
    let client = RoRegistryClient::new().unwrap();
    for id in &ids {
        if seen.contains(id) {
            continue;
        }
        assert!(
            file.metadata().unwrap().len() < 18 * 1024 * 1024,
            "journal budget exceeded"
        );
        if let Some(row) = &last {
            let completed =
                chrono::DateTime::parse_from_rfc3339(row["completedAt"].as_str().unwrap()).unwrap();
            let gap = if row.get("error").is_some() { 60 } else { 5 };
            let until = completed + chrono::Duration::seconds(gap);
            if let Ok(wait) = (until.with_timezone(&Utc) - Utc::now()).to_std() {
                tokio::time::sleep(wait).await;
            }
        }
        client.begin_capture();
        let start = std::time::Instant::now();
        let result = client.lookup_at(id, date).await;
        let capture = client.take_capture();
        let deferred = capture["retryAfter"].as_str().is_some();
        let mut row = json!({"schemaVersion":1,"taxId":id,"requestDate":date,"completedAt":Utc::now(),"durationMs":start.elapsed().as_millis(),"transport":capture});
        if let Some(body) = row["transport"]["body"].as_str() {
            row["bodySha256"] = json!(format!("{:x}", Sha256::digest(body.as_bytes())));
        }
        let stop = match result {
            Ok(result) => {
                row["response"] = serde_json::to_value(result).unwrap();
                false
            }
            Err(crate::LookupError::RegistryNotFound { .. }) => {
                row["notFound"] = json!(true);
                false
            }
            Err(error) => {
                row["error"] = json!({"message":error.to_string(),"debug":format!("{error:?}")});
                true
            }
        };
        writeln!(file, "{}", row).unwrap();
        file.flush().unwrap();
        file.sync_data().unwrap();
        seen.insert(id.clone());
        eprintln!("completed={}/{} error={stop}", seen.len(), ids.len());
        last = Some(row);
        assert!(
            !stop && !deferred,
            "provider deferral or parser error recorded; inspect before continuing"
        );
    }
    assert_eq!(
        seen.len(),
        ids.len(),
        "run stopped on provider or parser error; inspect before resuming"
    );
    // Avoid immediate next-batch request even when the wrapper changes processes.
    tokio::time::sleep(Duration::from_secs(5)).await;
}

#[test]
#[ignore = "offline replay of local ANAF observations"]
fn replay_anaf_batch() {
    let folder = PathBuf::from(std::env::var("RO_RUN_DIR").unwrap());
    let mut rows = Vec::new();
    for line in std::fs::read_to_string(folder.join("observations.jsonl"))
        .unwrap()
        .lines()
    {
        let row: Value = serde_json::from_str(line).unwrap();
        if let Some(body) = row["transport"]["body"].as_str() {
            assert_eq!(
                row["bodySha256"],
                format!("{:x}", Sha256::digest(body.as_bytes()))
            );
            let result = decode::decode(
                body.as_bytes(),
                &RoBusinessId::parse(row["taxId"].as_str().unwrap()).unwrap(),
                row["requestDate"].as_str().unwrap().parse().unwrap(),
            );
            let mut replay = json!({"taxId":row["taxId"],"bodySha256":row["bodySha256"]});
            match result {
                Ok(Some(value)) => {
                    replay["response"] = serde_json::to_value(value).unwrap();
                }
                Ok(None) => {
                    replay["notFound"] = json!(true);
                }
                Err(e) => {
                    replay["error"] = json!(e.to_string());
                }
            }
            rows.push(replay);
        }
    }
    std::fs::write(
        folder.join("offline-replay.json"),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "offline regression against the local reviewed ANAF corpus"]
fn reviewed_anaf_corpus() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ro/fixtures/anaf/ro.json");
    let cases: Vec<Value> = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert!(!cases.is_empty());
    let mut seen = HashSet::new();
    for case in cases {
        let id = RoBusinessId::parse(case["taxId"].as_str().unwrap()).unwrap();
        assert!(seen.insert(id.clone()), "duplicate corpus identifier");
        assert_eq!(case["review"]["status"], "reviewed");
        assert_eq!(
            case["review"]["expectedSha256"],
            format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&case["review"]["expected"]).unwrap())
            )
        );
        let raw = case["rawResponse"].as_str().unwrap();
        assert_eq!(
            case["bodySha256"],
            format!("{:x}", Sha256::digest(raw.as_bytes()))
        );
        let result = decode::decode(
            raw.as_bytes(),
            &id,
            case["requestDate"].as_str().unwrap().parse().unwrap(),
        )
        .unwrap();
        match result {
            Some(result) => {
                let mut actual = serde_json::to_value(result).unwrap();
                actual.as_object_mut().unwrap().remove("checkedAt");
                assert_eq!(actual, case["review"]["expected"], "{id}");
            }
            None => assert_eq!(case["review"]["notFound"], true),
        }
    }
}
