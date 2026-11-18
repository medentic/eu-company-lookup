//! Bounded local-only evidence, explicitly selected outside ordinary tests.
use super::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, io::Write, path::PathBuf, time::Duration};
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[tokio::test]
#[ignore = "explicit bounded authorized collection only; IS_RUN_DIR manifest required"]
async fn live_registry_batch() {
    let folder = PathBuf::from(std::env::var("IS_RUN_DIR").unwrap());
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(folder.join("manifest.json")).unwrap()).unwrap();
    let ids: Vec<String> = serde_json::from_value(manifest["ids"].clone()).unwrap();
    assert!(!ids.is_empty() && ids.len() <= 100);
    assert_eq!(ids.len(), ids.iter().collect::<HashSet<_>>().len());
    for id in &ids {
        IsBusinessId::parse(id).unwrap();
    }
    // Never silently resume attempted identifiers or bypass a preceding cooldown.
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(folder.join("observations.jsonl"))
        .unwrap();
    let native = IsRegistryClient::new().unwrap();
    let client = crate::Client::with_test_icelandic(native.clone());
    for id in ids {
        native.begin_capture();
        let result = client.lookup_company(&id, "IS").await;
        let capture = native.take_capture();
        let raw = capture["body"].as_str();
        let mut row = json!({"id": id, "checkedAt": jiff::Timestamp::now().to_string(), "capture": capture, "bodySha256": raw.map(|b|hash(b.as_bytes()))});
        if row["capture"].get("overviewResponses").is_some() {
            row["captureSha256"] = json!(hash(&serde_json::to_vec(&row["capture"]).unwrap()));
        }
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
        if failed {
            panic!(
                "Stopped after provider/parser error; preserve original journal before recovery"
            );
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

#[test]
#[ignore = "offline replay only; requires retained IS_RUN_DIR source observations"]
fn replay_registry_batch() {
    let folder = PathBuf::from(std::env::var("IS_RUN_DIR").unwrap());
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
        let id = IsBusinessId::parse(row["id"].as_str().unwrap()).unwrap();
        let raw = row["capture"]["body"].as_str().unwrap();
        assert_eq!(row["bodySha256"], hash(raw.as_bytes()));
        let native = replay_native(&row, &id).unwrap();
        let common: crate::CompanyLookupResult = native.into();
        let actual = serde_json::to_value(common).unwrap();
        let review = &expectations[id.as_str()];
        assert_eq!(review["bodySha256"], row["bodySha256"]);
        if !row["captureSha256"].is_null() {
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
    crate::test_support::retained_registry::run("IS", |row| {
        let id = IsBusinessId::parse(row["id"].as_str().ok_or("missingIdentity")?)
            .map_err(|e| e.to_string())?;
        crate::test_support::retained_registry::body(row, "/capture/body", "bodySha256")?;
        if row["capture"]["httpStatus"] != 200 {
            return Err("nonSuccessHttpStatus".into());
        }
        replay_native(row, &id)
            .map(|v| Some(v.into()))
            .map_err(|e| e.to_string())
    });
}

fn replay_native(row: &Value, id: &IsBusinessId) -> Result<IsCompanyLookupResult, String> {
    let raw = crate::test_support::retained_registry::body(row, "/capture/body", "bodySha256")?;
    let mut native = decode::decode(raw.as_bytes(), id).map_err(|e| e.to_string())?;
    if let Some(responses) = row["capture"]["overviewResponses"].as_array() {
        use base64::Engine;
        if row["captureSha256"] != hash(&serde_json::to_vec(&row["capture"]).unwrap()) {
            return Err("overviewCaptureHashMismatch".into());
        }
        if responses.len() != 5 || responses.iter().any(|r| r["httpStatus"] != 200) {
            return Err("incompleteOrFailedOverview".into());
        }
        if !overview::available(raw.as_bytes(), id).map_err(|e| e.to_string())? {
            return Err("missingOverviewOffer".into());
        }
        for (index, response) in responses.iter().enumerate() {
            let (content_type, encoding) = if index == 4 {
                ("application/pdf", "base64")
            } else {
                ("text/html", "utf8")
            };
            if response["encoding"] != encoding
                || response["contentType"]
                    .as_str()
                    .and_then(|s| s.split(';').next())
                    .map(str::trim)
                    != Some(content_type)
            {
                return Err("unexpectedOverviewRepresentation".into());
            }
        }
        let text = |index: usize| -> Result<&[u8], String> {
            responses[index]["body"]
                .as_str()
                .map(str::as_bytes)
                .ok_or("missingOverviewBody".into())
        };
        let kid = overview::cart(text(0)?).map_err(|e| e.to_string())?;
        overview::page_fields(text(1)?, id, false).map_err(|e| e.to_string())?;
        overview::confirmation(text(2)?, &kid).map_err(|e| e.to_string())?;
        overview::page_fields(text(3)?, id, true).map_err(|e| e.to_string())?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(text(4)?)
            .map_err(|e| e.to_string())?;
        native.details.registration_overview =
            Some(overview::pdf(&bytes, id).map_err(|e| e.to_string())?);
    }
    Ok(native)
}
