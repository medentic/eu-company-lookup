//! Read-only source replay audit. Reports differences without modifying reviews.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(crate) fn body<'a>(row: &'a Value, pointer: &str, hash_key: &str) -> Result<&'a str, String> {
    let body = row
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or("missingCapturedBody")?;
    let digest = format!("{:x}", Sha256::digest(body.as_bytes()));
    if row[hash_key] != digest {
        return Err("sourceHashMismatch".into());
    }
    Ok(body)
}

pub(crate) fn run(
    country: &str,
    decode: impl Fn(&Value) -> Result<Option<crate::CompanyLookupResult>, String>,
) {
    let manifest =
        std::env::var("REGISTRY_AUDIT_INPUT").expect("explicit local audit manifest required");
    let runs: Value = serde_json::from_slice(&std::fs::read(manifest).unwrap()).unwrap();
    let output =
        std::env::var("REGISTRY_AUDIT_OUTPUT").expect("separate derived output folder required");
    let mut cases = Vec::new();
    for run in runs
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["country"] == country)
    {
        let path = Path::new(run["path"].as_str().unwrap());
        let source = std::fs::read_to_string(path).unwrap();
        assert!(source.ends_with('\n'), "partial journal must be preserved");
        let review_path = path.with_file_name("expectations.json");
        let reviews: Value = if review_path.exists() {
            serde_json::from_slice(&std::fs::read(review_path).unwrap()).unwrap()
        } else {
            Value::Null
        };
        for line in source.lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            let id = row["id"]
                .as_str()
                .or_else(|| row["taxId"].as_str())
                .expect("identity required");
            let mut result =
                json!({"id":id,"country":country,"sourcePath":path,"sourceHash":row["bodySha256"]});
            match decode(&row) {
                Ok(Some(common)) => {
                    let quality = crate::model::company_validation::ensure_resolved(common.clone());
                    let actual = serde_json::to_value(common).unwrap();
                    let review = &reviews[id];
                    let bound = !review.is_null()
                        && review["bodySha256"] == row["bodySha256"]
                        && (row["captureSha256"].is_null()
                            || review["captureSha256"] == row["captureSha256"]);
                    let mut diffs = Vec::new();
                    if let Some(fields) = review["fields"].as_object() {
                        for (pointer, expected) in fields {
                            if actual.pointer(pointer) != Some(expected) {
                                diffs.push(json!({"pointer":pointer,"expected":expected,"actual":actual.pointer(pointer)}));
                            }
                        }
                    }
                    result["reviewBound"] = json!(bound);
                    result["differences"] = json!(diffs);
                    result["validationStatus"] = json!(if quality.is_err() {
                        "unresolved"
                    } else if bound
                        && diffs.is_empty()
                        && review["fields"]["/status"] == actual["status"]
                    {
                        "verifiedPositive"
                    } else {
                        "needsReview"
                    });
                    result["lookupError"] = quality
                        .err()
                        .map(|e| serde_json::to_value(e).unwrap())
                        .unwrap_or(Value::Null);
                    result["actual"] = actual;
                }
                Ok(None) => result["validationStatus"] = json!("negativeObservation"),
                Err(error) => {
                    result["validationStatus"] = json!("failed");
                    result["error"] = json!(error);
                }
            }
            cases.push(result);
        }
    }
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(
        Path::new(&output).join(format!("{country}.json")),
        serde_json::to_vec_pretty(&cases).unwrap(),
    )
    .unwrap();
    println!("{country}: audited {} retained observations", cases.len());
}
