//! Bounded local-only evidence, explicitly selected outside ordinary tests.
use super::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, io::Write, path::PathBuf, time::Duration};
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[tokio::test]
#[ignore = "explicit bounded authorized collection only; LV_RUN_DIR manifest required"]
async fn live_registry_batch() {
    let folder = PathBuf::from(std::env::var("LV_RUN_DIR").unwrap());
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(folder.join("manifest.json")).unwrap()).unwrap();
    let ids: Vec<String> = serde_json::from_value(manifest["ids"].clone()).unwrap();
    assert!(!ids.is_empty() && ids.len() <= 100);
    assert_eq!(ids.len(), ids.iter().collect::<HashSet<_>>().len());
    for id in &ids {
        LvBusinessId::parse(id).unwrap();
    }
    // Never silently resume attempted identifiers or bypass a preceding cooldown.
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(folder.join("observations.jsonl"))
        .unwrap();
    let native = LvRegistryClient::new().unwrap();
    let client = crate::Client::with_test_latvian(native.clone());
    for id in ids {
        native.begin_capture();
        let result = client.lookup_company(&id, "LV").await;
        let capture = native.take_capture();
        let raw = capture["body"].as_str();
        let mut row = json!({"id": id, "checkedAt": jiff::Timestamp::now().to_string(), "capture": capture, "bodySha256": raw.map(|b|hash(b.as_bytes()))});
        row["captureSha256"] = json!(hash(&serde_json::to_vec(&row["capture"]).unwrap()));
        let failed = result.is_err();
        match result {
            Ok(value) => row["result"] = serde_json::to_value(value).unwrap(),
            Err(e) => {
                row["error"] = json!(format!("{e:?}"));
                row["lookupError"] = serde_json::to_value(e).unwrap();
            }
        }
        writeln!(output, "{}", serde_json::to_string(&row).unwrap()).unwrap();
        output.sync_all().unwrap();
        println!("{id}: success={}", !failed);
        // A matched but unresolved record is evidence to review, not an outage.
        // Preserve it and finish the authorized selection; stop on transport/parser failures.
        if failed && row["lookupError"]["id"] != "unresolved_data" {
            panic!(
                "Stopped after provider/parser error; preserve original journal before recovery"
            );
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

#[test]
#[ignore = "offline replay only; requires retained LV_RUN_DIR source observations"]
fn replay_registry_batch() {
    let folder = PathBuf::from(std::env::var("LV_RUN_DIR").unwrap());
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(folder.join("manifest.json")).unwrap()).unwrap();
    let selected: Vec<String> = serde_json::from_value(manifest["ids"].clone()).unwrap();
    let source = std::fs::read_to_string(folder.join("observations.jsonl")).unwrap();
    assert!(source.ends_with('\n'), "preserve incomplete journal");
    let expectations: Value =
        serde_json::from_slice(&std::fs::read(folder.join("expectations.json")).unwrap()).unwrap();
    let mut replay = Vec::new();
    for line in source.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let id = LvBusinessId::parse(row["id"].as_str().unwrap()).unwrap();
        let raw = row["capture"]["body"].as_str().unwrap();
        assert_eq!(row["bodySha256"], hash(raw.as_bytes()));
        let native = replay_native(&row, &id).unwrap();
        let common: crate::CompanyLookupResult = native.into();
        let actual = serde_json::to_value(common).unwrap();
        let review = &expectations[id.as_str()];
        assert_eq!(review["bodySha256"], row["bodySha256"]);
        if row["capture"]["lifecycle"].is_object() {
            assert_eq!(review["captureSha256"], row["captureSha256"]);
        }
        for (pointer, expected) in review["fields"].as_object().expect("review required") {
            assert_eq!(
                actual.pointer(pointer),
                Some(expected),
                "{} {pointer}",
                id.as_str()
            );
        }
        let validation_status =
            crate::test_support::company_validation::classify(&actual, &review["fields"]);
        replay.push(json!({"id":id.as_str(),"actual":actual,"validationStatus":validation_status,"unresolvedDetail": if validation_status == "unresolved" { crate::test_support::company_validation::unresolved_detail(&actual) } else { Value::Null }}));
    }
    std::fs::write(
        folder.join("validation-summary.json"),
        serde_json::to_vec_pretty(&crate::test_support::company_validation::summary(
            &replay, &selected,
        ))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(
        folder.join("offline-replay.json"),
        serde_json::to_vec_pretty(&replay).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "offline audit of explicitly selected retained responses; never collects"]
fn audit_retained_registry_responses() {
    crate::test_support::retained_registry::run("LV", |row| {
        let id = LvBusinessId::parse(row["id"].as_str().ok_or("missingIdentity")?)
            .map_err(|e| e.to_string())?;
        let raw = crate::test_support::retained_registry::body(row, "/capture/body", "bodySha256")?;
        if row["capture"]["httpStatus"] != 200 {
            return Err("nonSuccessHttpStatus".into());
        }
        let _ = raw;
        replay_native(row, &id)
            .map(|v| Some(v.into()))
            .map_err(|e| e.to_string())
    });
}

fn replay_native(row: &Value, id: &LvBusinessId) -> Result<LvCompanyLookupResult, String> {
    let raw = crate::test_support::retained_registry::body(row, "/capture/body", "bodySha256")?;
    let mut result = decode::decode(raw.as_bytes(), id).map_err(|e| e.to_string())?;
    if row["capture"]["lifecycle"].is_object() {
        if row["captureSha256"] != hash(&serde_json::to_vec(&row["capture"]).unwrap()) {
            return Err("sourceHashMismatch".into());
        }
        let mut bodies = Vec::new();
        for (name, _, _) in lifecycle::SOURCES {
            let r = &row["capture"]["lifecycle"][name];
            if r["httpStatus"] != 200 {
                return Err("nonSuccessHttpStatus".into());
            }
            bodies.push(r["body"].as_str().ok_or("missingCapturedBody")?);
        }
        result.details.lifecycle = Some(
            lifecycle::decode(
                bodies[0].as_bytes(),
                bodies[1].as_bytes(),
                bodies[2].as_bytes(),
                id,
            )
            .map_err(|e| e.to_string())?,
        );
    }
    Ok(result)
}

#[test]
#[ignore = "offline replay of explicitly selected multi-dataset Latvian captures"]
fn replay_retained_lifecycle_sources() {
    let folder = PathBuf::from(std::env::var("LV_LIFECYCLE_RUN_DIR").unwrap());
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(folder.join("lv-supplement-manifest.json")).unwrap())
            .unwrap();
    let ids: Vec<String> = serde_json::from_value(manifest["ids"].clone()).unwrap();
    assert!(!ids.is_empty() && ids.len() <= 100);
    assert_eq!(
        ids.iter().collect::<std::collections::HashSet<_>>().len(),
        ids.len()
    );
    let journal = std::fs::read_to_string(folder.join("lv-supplement-observations.jsonl")).unwrap();
    assert!(journal.ends_with('\n'));
    let captures: Vec<Value> = journal
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(captures.len(), 4);
    let mut datasets = std::collections::HashMap::new();
    let mut hashes = serde_json::Map::new();
    for (index, (name, resource, field)) in std::iter::once(("base", decode::RESOURCE, "regcode"))
        .chain(lifecycle::SOURCES)
        .enumerate()
    {
        let row = &captures[index];
        assert_eq!(row["source"], name);
        assert_eq!(row["httpStatus"], 200);
        let raw = row["body"].as_str().unwrap();
        assert_eq!(row["bodySha256"], hash(raw.as_bytes()));
        hashes.insert(name.into(), row["bodySha256"].clone());
        let response: Value = serde_json::from_str(raw).unwrap();
        assert_eq!(response["success"], true);
        let result = &response["result"];
        assert_eq!(result["resource_id"], resource);
        assert_eq!(result["filters"], json!({field:ids}));
        let records = result["records"].as_array().unwrap();
        assert_eq!(result["total"], records.len());
        assert!(records.len() <= 500);
        for record in records {
            let echoed = record[field]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| record[field].to_string());
            assert!(ids.contains(&echoed));
        }
        datasets.insert(name, records.clone());
    }
    let reviews: Value = serde_json::from_slice(
        &std::fs::read(folder.join("lv-lifecycle-expectations.json")).unwrap(),
    )
    .unwrap();
    let mut replay = Vec::new();
    for id in &ids {
        let select = |name: &str, field: &str| -> Vec<Value> {
            datasets[name]
                .iter()
                .filter(|r| {
                    r[field].as_str() == Some(id.as_str())
                        || r[field]
                            .as_u64()
                            .zip(id.parse::<u64>().ok())
                            .is_some_and(|(a, b)| a == b)
                })
                .cloned()
                .collect()
        };
        let base = select("base", "regcode");
        assert_eq!(base.len(), 1);
        let id_parsed = LvBusinessId::parse(id).unwrap();
        let mut native =
            decode::record(serde_json::from_value(base[0].clone()).unwrap(), &id_parsed).unwrap();
        native.details.lifecycle = Some(LvLifecycleDetails {
            liquidations: serde_json::from_value(json!(select(
                "liquidation",
                lifecycle::SOURCES[0].2
            )))
            .unwrap(),
            suspensions: serde_json::from_value(json!(select(
                "suspension",
                lifecycle::SOURCES[1].2
            )))
            .unwrap(),
            insolvencies: serde_json::from_value(json!(select(
                "insolvency",
                lifecycle::SOURCES[2].2
            )))
            .unwrap(),
        });
        let common: crate::CompanyLookupResult = native.into();
        let actual = serde_json::to_value(&common).unwrap();
        let review = &reviews[id];
        assert_eq!(review["sourceHashes"], json!(hashes));
        assert_eq!(actual["status"], review["status"], "{id}");
        let validation = crate::test_support::company_validation::classify(
            &actual,
            &json!({"/status":review["status"]}),
        );
        assert_eq!(
            crate::model::company_validation::ensure_resolved(common).is_ok(),
            validation == "verifiedPositive"
        );
        replay.push(
            json!({"id":id,"actual":actual,"validationStatus":validation,"sourceHashes":hashes}),
        );
    }
    std::fs::write(
        folder.join("lv-lifecycle-replay.json"),
        serde_json::to_vec_pretty(&replay).unwrap(),
    )
    .unwrap();
    std::fs::write(
        folder.join("lv-lifecycle-summary.json"),
        serde_json::to_vec_pretty(&crate::test_support::company_validation::summary(
            &replay, &ids,
        ))
        .unwrap(),
    )
    .unwrap();
}
