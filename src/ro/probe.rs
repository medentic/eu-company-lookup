//! Explicitly selected local-only observations; never part of the ordinary tests.
use super::*;
use crate::LookupError;
use jiff::{Timestamp, civil::Date};
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
    let date: Date = manifest["requestDate"].as_str().unwrap().parse().unwrap();
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
            let completed = row["completedAt"]
                .as_str()
                .unwrap()
                .parse::<jiff::Timestamp>()
                .unwrap();
            let gap = if row.get("error").is_some() { 60 } else { 5 };
            let until = completed + jiff::SignedDuration::from_secs(gap);
            if let Ok(wait) = (until - Timestamp::now()).try_into() {
                tokio::time::sleep(wait).await;
            }
        }
        client.begin_capture();
        let start = std::time::Instant::now();
        let result = client.lookup_at(id, date).await;
        let capture = client.take_capture();
        let deferred = capture["retryAfter"].as_str().is_some();
        let mut row = json!({"schemaVersion":1,"taxId":id,"requestDate":date,"completedAt":Timestamp::now(),"durationMs":start.elapsed().as_millis(),"transport":capture});
        if let Some(body) = row["transport"]["body"].as_str() {
            row["bodySha256"] = json!(format!("{:x}", Sha256::digest(body.as_bytes())));
        }
        let stop = match result {
            Ok(result) => {
                row["response"] = serde_json::to_value(result).unwrap();
                false
            }
            Err(LookupError {
                kind: crate::model::LookupFailure::RegistryNotFound { .. },
                ..
            }) => {
                row["notFound"] = json!(true);
                false
            }
            Err(error) => {
                row["error"] = json!({"message":error.to_string(),"debug":format!("{error:?}")});
                row["lookupError"] = serde_json::to_value(error).unwrap();
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
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(folder.join("manifest.json")).unwrap()).unwrap();
    let selected: Vec<String> = serde_json::from_value(manifest["ids"].clone()).unwrap();
    let reviews: Value = match std::fs::read(folder.join("expectations.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Value::Null,
        Err(e) => panic!("cannot read reviews: {e}"),
    };
    let mut rows = Vec::new();
    let journal = std::fs::read_to_string(folder.join("observations.jsonl")).unwrap();
    assert!(
        journal.is_empty() || journal.ends_with('\n'),
        "preserve partial journal"
    );
    for line in journal.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        assert_eq!(row["requestDate"], manifest["requestDate"]);
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
            let review = &reviews[row["taxId"].as_str().unwrap()];
            if !review.is_null() {
                assert_eq!(review["bodySha256"], row["bodySha256"]);
            }
            let mut replay =
                json!({"id":row["taxId"],"taxId":row["taxId"],"bodySha256":row["bodySha256"]});
            match result {
                Ok(Some(value)) => {
                    replay["response"] = serde_json::to_value(value).unwrap();
                    if !review.is_null() {
                        for (pointer, expected) in review["fields"]
                            .as_object()
                            .expect("review fields required")
                        {
                            assert_eq!(replay["response"].pointer(pointer), Some(expected));
                        }
                    }
                    replay["validationStatus"] =
                        json!(crate::test_support::company_validation::classify(
                            &replay["response"],
                            &review["fields"]
                        ));
                }
                Ok(None) => {
                    replay["notFound"] = json!(true);
                    replay["validationStatus"] = json!(if review["outcome"] == "notFound" {
                        "verifiedNegative"
                    } else {
                        "negativeObservation"
                    });
                }
                Err(e) => {
                    replay["error"] = json!(e.to_string());
                    replay["validationStatus"] = json!("failed");
                }
            }
            if replay["validationStatus"] == "unresolved" {
                replay["unresolvedDetail"] =
                    crate::test_support::company_validation::unresolved_detail(&replay["response"]);
            }
            rows.push(replay);
        } else {
            rows.push(json!({"id":row["taxId"],"taxId":row["taxId"],"validationStatus":"failed","error":"missing captured body"}));
        }
    }
    std::fs::write(
        folder.join("validation-summary.json"),
        serde_json::to_vec_pretty(&crate::test_support::company_validation::summary(
            &rows, &selected,
        ))
        .unwrap(),
    )
    .unwrap();
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
                if let Some(reviewed) = case["review"]["validationStatus"].as_str() {
                    let common: crate::CompanyLookupResult = result.clone().into();
                    let resolved =
                        crate::model::company_validation::ensure_resolved(common).is_ok();
                    assert_eq!(
                        resolved,
                        reviewed == "verifiedPositive",
                        "{id}: lifecycle review qualification"
                    );
                    assert!(matches!(reviewed, "verifiedPositive" | "unresolved"));
                }
                let mut actual = serde_json::to_value(result).unwrap();
                actual.as_object_mut().unwrap().remove("checkedAt");
                assert_eq!(actual, case["review"]["expected"], "{id}");
            }
            None => assert_eq!(case["review"]["notFound"], true),
        }
    }
}

#[test]
#[ignore = "read-only offline audit of the retained ANAF corpus"]
fn audit_retained_anaf_corpus() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ro/fixtures/anaf/ro.json");
    let cases: Vec<Value> = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut rows = Vec::new();
    for case in cases {
        let raw = case["rawResponse"].as_str().unwrap();
        assert_eq!(
            case["bodySha256"],
            format!("{:x}", Sha256::digest(raw.as_bytes()))
        );
        let id = RoBusinessId::parse(case["taxId"].as_str().unwrap()).unwrap();
        let result = decode::decode(
            raw.as_bytes(),
            &id,
            case["requestDate"].as_str().unwrap().parse().unwrap(),
        )
        .unwrap();
        if let Some(native) = result {
            let common: crate::CompanyLookupResult = native.clone().into();
            let mut actual = serde_json::to_value(native).unwrap();
            actual.as_object_mut().unwrap().remove("checkedAt");
            let expected = &case["review"]["expected"];
            let differences: Vec<_> = expected
                .as_object()
                .unwrap()
                .iter()
                .filter(|(k, v)| actual[*k] != **v)
                .map(|(k, v)| json!({"field":k,"expected":v,"actual":actual[k]}))
                .collect();
            rows.push(json!({"id":id.as_str(),"bodySha256":case["bodySha256"],"differences":differences,"actual":actual,"resolved":crate::model::company_validation::ensure_resolved(common).is_ok()}));
        } else {
            rows.push(json!({"id":id.as_str(),"negative":true,"reviewMatches":case["review"]["notFound"]==true}));
        }
    }
    let dir = std::env::var("REGISTRY_AUDIT_OUTPUT").unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        std::path::Path::new(&dir).join("RO-corpus.json"),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
}
