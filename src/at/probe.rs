//! Bounded local-only evidence, explicitly selected outside ordinary tests.
use super::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, io::Write, path::PathBuf, time::Duration};
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[tokio::test]
#[ignore = "explicit bounded authorized collection only; AT_RUN_DIR manifest required"]
async fn live_firmenbuch_batch() {
    let folder = PathBuf::from(std::env::var("AT_RUN_DIR").unwrap());
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(folder.join("manifest.json")).unwrap()).unwrap();
    let ids: Vec<String> = serde_json::from_value(manifest["ids"].clone()).unwrap();
    assert!(!ids.is_empty() && ids.len() <= 100);
    assert_eq!(ids.len(), ids.iter().collect::<HashSet<_>>().len());
    for id in &ids {
        AtBusinessId::parse(id).unwrap();
    }
    // Never silently resume attempted identifiers or bypass a preceding cooldown.
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(folder.join("observations.jsonl"))
        .unwrap();
    let native = AtRegistryClient::new().unwrap();
    let client = crate::Client::with_test_austrian(native.clone());
    for id in ids {
        native.begin_capture();
        let result = client.lookup_company(&id, "AT").await;
        let capture = native.take_capture();
        let raw = serde_json::to_vec(&capture).unwrap();
        let mut row = json!({"id": id, "checkedAt": chrono::Utc::now().to_rfc3339(), "capture": capture, "bodySha256": hash(&raw)});
        let failed = result.is_err();
        match result {
            Ok(value) => row["result"] = serde_json::to_value(value).unwrap(),
            Err(e) => row["error"] = json!(format!("{e:?}")),
        }
        writeln!(output, "{}", serde_json::to_string(&row).unwrap()).unwrap();
        output.sync_all().unwrap();
        println!("{id}: success={}", !failed);
        if failed {
            panic!(
                "Stopped after provider/parser error; preserve original journal before recovery"
            );
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

#[test]
#[ignore = "offline replay only; requires retained AT_RUN_DIR source observations"]
fn replay_firmenbuch_batch() {
    let folder = PathBuf::from(std::env::var("AT_RUN_DIR").unwrap());
    let source = std::fs::read_to_string(folder.join("observations.jsonl")).unwrap();
    assert!(source.ends_with('\n'), "preserve incomplete journal");
    let expectations: Value =
        serde_json::from_slice(&std::fs::read(folder.join("expectations.json")).unwrap()).unwrap();
    let mut replay = Vec::new();
    for line in source.lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let id = AtBusinessId::parse(row["id"].as_str().unwrap()).unwrap();
        let captured = &row["capture"];
        assert_eq!(
            row["bodySha256"],
            hash(&serde_json::to_vec(captured).unwrap())
        );
        let responses = captured["responses"].as_array().unwrap();
        let review = &expectations[id.as_str()];
        assert_eq!(review["bodySha256"], row["bodySha256"]);
        if review["outcome"] == "emptyActiveSearch" {
            assert_eq!(responses.len(), 1);
            assert_eq!(responses[0]["httpStatus"], 200);
            let raw = responses[0]["body"].as_str().unwrap();
            let search: Value = serde_json::from_str(raw).unwrap();
            assert_eq!(search["numResults"], 0);
            assert_eq!(search["companies"], json!([]));
            assert_eq!(search["state"], "ACTIVE");
            assert!(matches!(
                decode::select(raw.as_bytes(), &id),
                Err(crate::RegistryFailure::UnexpectedResponse)
            ));
            assert!(row.get("error").is_some() && row.get("result").is_none());
            replay.push(json!({"id":id.as_str(),"outcome":"ambiguousEmptyActiveSearch"}));
            continue;
        }
        assert_eq!(responses.len(), 2);
        assert!(responses.iter().all(|r| r["httpStatus"] == 200));
        let record =
            decode::select(responses[0]["body"].as_str().unwrap().as_bytes(), &id).unwrap();
        let native = decode::detail(
            responses[1]["body"].as_str().unwrap().as_bytes(),
            &id,
            &record,
        )
        .unwrap();
        let common: crate::CompanyLookupResult = native.into();
        let actual = serde_json::to_value(common).unwrap();
        for (pointer, expected) in review["fields"].as_object().expect("review required") {
            assert_eq!(
                actual.pointer(pointer),
                Some(expected),
                "{} {pointer}",
                id.as_str()
            );
        }
        replay.push(json!({"id":id.as_str(),"actual":actual}));
    }
    std::fs::write(
        folder.join("offline-replay.json"),
        serde_json::to_vec_pretty(&replay).unwrap(),
    )
    .unwrap();
}
