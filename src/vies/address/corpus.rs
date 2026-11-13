//! Saved-data regression and explicitly requested live verification.
use crate::vies::reporting::{Facts, Status, classify};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::HashSet, path::PathBuf};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Case {
    vat_id: String,
    valid: bool,
    vies_output: Source,
    expected: Expected,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    company: Option<String>,
    address: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    address: Value,
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
enum ReviewStatus {
    Reviewed,
    Ambiguous,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Review {
    vat_id: String,
    status: ReviewStatus,
    note: String,
    reviewed_at: chrono::NaiveDate,
    review_version: u32,
    normalization_version: String,
    expected_sha256: String,
}

fn cases() -> Vec<Case> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/vies/fixtures/countries");
    assert!(root.is_dir(), "local-only VIES corpus is not installed");
    let filter = std::env::var("VIES_COUNTRY").ok().map(|s| s.to_lowercase());
    let mut paths: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|p| p.unwrap().path())
        .filter(|p| p.extension().is_some_and(|s| s == "json"))
        .filter(|p| {
            filter
                .as_ref()
                .is_none_or(|f| p.file_stem().unwrap().to_str() == Some(f))
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no country fixture matched");
    let mut seen = HashSet::new();
    let mut all = vec![];
    for path in paths {
        let country = path.file_stem().unwrap().to_str().unwrap();
        let batch: Vec<Case> =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).expect("invalid corpus JSON");
        assert!(!batch.is_empty(), "empty country corpus");
        for case in batch {
            let vat = crate::VatId::parse(&case.vat_id).expect("invalid VAT syntax");
            assert_eq!(
                super::country(vat.prefix()).unwrap().0.to_lowercase(),
                country
            );
            assert!(seen.insert(case.vat_id.clone()), "duplicate VAT ID");
            let object = case
                .expected
                .address
                .as_object()
                .expect("expected address object");
            let keys = [
                "address",
                "address2",
                "city",
                "postcode",
                "region",
                "country",
                "countryISO2",
            ];
            assert_eq!(
                object.len(),
                keys.len(),
                "exact expected address fields required"
            );
            for key in keys {
                assert!(
                    object
                        .get(key)
                        .is_some_and(|v| v.is_null() || v.is_string()),
                    "missing/invalid {key}"
                );
            }
            all.push(case);
        }
    }
    if let Ok(limit) = std::env::var("VIES_LIMIT") {
        all.truncate(limit.parse().expect("positive VIES_LIMIT"));
    }
    assert!(!all.is_empty(), "no cases selected");
    all
}

fn differences(expected: &Value, actual: &Value) -> Vec<Value> {
    expected
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, value)| actual.get(*key) != Some(*value))
        .map(|(key, value)| json!({"field": key, "expected": value, "actual": actual.get(key)}))
        .collect()
}

fn report(mut rows: Vec<Value>, mode: &str) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/vies/fixtures/countries/reviews/bg.json");
    if path.exists() {
        let reviews: Vec<Review> = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        for row in &mut rows {
            if let Some(review) = reviews
                .iter()
                .find(|r| row["vatId"].as_str() == Some(r.vat_id.as_str()))
            {
                row["reviewNote"] = json!(review.note);
                row["reviewVersion"] = json!(review.review_version);
                row["reviewedAt"] = json!(review.reviewed_at);
                row["normalizationVersion"] = json!(review.normalization_version);
                row["expectedSha256"] = json!(review.expected_sha256);
                if row["status"] == "passed" && review.status == ReviewStatus::Ambiguous {
                    row["status"] = json!(classify(&Facts {
                        observed: true,
                        response: true,
                        expectation: true,
                        ambiguous: true,
                        ..Facts::default()
                    }));
                }
            }
        }
    }
    let total = rows.len();
    let ambiguous = rows.iter().filter(|r| r["status"] == "ambiguous").count();
    let passed = rows.iter().filter(|r| r["status"] == "passed").count();
    let report = json!({"mode": mode, "checkedAt": chrono::Utc::now(), "total": total,
        "passed": passed, "ambiguous": ambiguous, "parserMatched": passed + ambiguous, "notPassed": total - passed,
        "passPercentage": if total == 0 { 0.0 } else { passed as f64 * 100.0 / total as f64 },
        "cases": rows});
    if let Ok(path) = std::env::var("VIES_REPORT") {
        std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }
    eprintln!("{}", serde_json::to_string_pretty(&report).unwrap());
    assert_eq!(passed + ambiguous, total, "corpus failures; inspect report");
}

#[test]
#[ignore = "offline regression against local-only reviewed VIES observations"]
fn saved_corpus() {
    let rows = cases().into_iter().map(|case| {
        let vat = crate::VatId::parse(&case.vat_id).unwrap();
        let parsed = super::split(vat.prefix(), case.vies_output.address.clone());
        assert_eq!(parsed.full_address_by_vies, case.vies_output.address);
        let actual = serde_json::to_value(parsed).unwrap();
        let differences = differences(&case.expected.address, &actual);
        json!({"vatId": case.vat_id, "status": classify(&Facts { observed: true, response: true, expectation: true, parser_mismatch: !differences.is_empty(), ..Facts::default() }), "differences": differences, "actual": actual})
    }).collect();
    report(rows, "saved");
}

#[tokio::test]
#[ignore = "production VIES calls; explicitly select country and request live check"]
async fn live_corpus() {
    let client = crate::Client::with_config(crate::ClientConfig {
        timeout: std::time::Duration::from_secs(25),
        ..Default::default()
    })
    .unwrap();
    let mut rows = vec![];
    for case in cases() {
        let row = match client.lookup_vat(&case.vat_id).await {
            Err(error) => {
                json!({"vatId": case.vat_id, "status": Status::ProviderError, "error": error.to_string()})
            }
            Ok(result) => {
                let actual = serde_json::to_value(&result.address).unwrap();
                let diff = differences(&case.expected.address, &actual);
                let status = classify(&Facts {
                    observed: true,
                    response: true,
                    expectation: true,
                    validity_changed: result.valid != case.valid,
                    source_changed: result.company != case.vies_output.company
                        || result.address.full_address_by_vies != case.vies_output.address,
                    parser_mismatch: !diff.is_empty(),
                    ambiguous: false,
                });
                json!({"vatId": case.vat_id, "status": status, "differences": diff, "response": result})
            }
        };
        // Optional append-only progress journal survives interruption. Never edits fixtures.
        if let Ok(path) = std::env::var("VIES_PROGRESS") {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .unwrap();
            writeln!(
                file,
                "{}",
                json!({"checkedAt": chrono::Utc::now(), "case": row})
            )
            .unwrap();
        }
        rows.push(row);
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    report(rows, "live");
}

#[test]
fn report_detects_wrong_fields_and_iso_mappings() {
    assert_eq!(
        differences(&json!({"city": "Русе"}), &json!({"city": "София"})).len(),
        1
    );
    for (prefix, iso) in [("BG", "BG"), ("EL", "GR"), ("XI", "GB")] {
        assert_eq!(
            super::split(prefix, None).country_iso2.as_deref(),
            Some(iso)
        );
    }
}

#[test]
#[ignore = "offline replay of local probe journals; set VIES_REPLAY_DIR"]
fn replay_probe_journals() {
    let root = PathBuf::from(std::env::var("VIES_REPLAY_DIR").unwrap());
    let mut paths: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("batch-")
                && p.extension().is_some_and(|e| e == "jsonl")
        })
        .collect();
    if root.join("observations.jsonl").exists() {
        paths.push(root.join("observations.jsonl"));
    }
    paths.sort();
    let mut rows = vec![];
    for path in paths {
        for line in std::fs::read_to_string(path).unwrap().lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            let last = row["attempts"].as_array().unwrap().last().unwrap();
            let Some(response) = last.get("response") else {
                continue;
            };
            let vat = crate::VatId::parse(response["vatId"].as_str().unwrap()).unwrap();
            let raw = response["address"]["fullAddressByVies"]
                .as_str()
                .map(str::to_owned);
            let parsed = super::split(vat.prefix(), raw.clone());
            assert_eq!(parsed.full_address_by_vies, raw);
            let actual = serde_json::to_value(parsed).unwrap();
            rows.push(json!({"vatId":vat,"differencesFromObservation":differences(&response["address"], &actual),"actual":actual}));
        }
    }
    assert!(!rows.is_empty());
    std::fs::write(
        root.join("offline-replay.json"),
        serde_json::to_string_pretty(&rows).unwrap(),
    )
    .unwrap();
    eprintln!(
        "Replayed {} saved responses; observation differences are not approved expectations",
        rows.len()
    );
}
