//! Local-only bounded observations through the public Bulgarian lookup path.
use super::{
    BgBusinessId, BgRegistryClient, bulstat,
    probe::{source_sha, write_json},
};
use crate::{LookupError, Provider, RegistryError, RetryAfter};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, time::Instant};

fn read(path: impl AsRef<std::path::Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[tokio::test]
#[ignore = "explicit bounded live fallback collection; use tools/bulstat_batch.py"]
async fn live_bulstat_batch() {
    let dir = PathBuf::from(std::env::var("BULSTAT_RUN_DIR").expect("run directory"));
    let state_path = PathBuf::from(std::env::var("REGISTRY_PROVIDER_STATE").expect("shared state"));
    let manifest = read(dir.join("manifest.json"));
    let ids = manifest["ids"].as_array().unwrap();
    assert!(!ids.is_empty() && ids.len() <= 10);
    let client = BgRegistryClient::new().unwrap();
    for value in ids {
        let id = BgBusinessId::parse(value.as_str().unwrap()).unwrap();
        let path = dir.join("observations").join(format!("{id}.json"));
        if path.exists() {
            continue;
        }
        let state = read(&state_path);
        for key in ["nextStartAt", "cooldownUntil"] {
            if let Some(until) = state[key].as_str() {
                let until = until.parse::<chrono::DateTime<chrono::Utc>>().unwrap();
                if let Ok(wait) = (until - chrono::Utc::now()).to_std() {
                    tokio::time::sleep(wait).await;
                }
            }
        }
        let start = chrono::Utc::now();
        // Reserve the complete deadline plus spacing, including after interruption.
        write_json(
            &state_path,
            &json!({"nextStartAt":start+chrono::Duration::seconds(60),"cooldownUntil":null}),
        );
        client.begin_capture();
        let timer = Instant::now();
        let result = client.lookup(id.as_str()).await;
        let elapsed = timer.elapsed().as_millis();
        let requests = client.take_capture();
        let (outcome, record, error) = match &result {
            Ok(record) => (
                if record.source == Provider::Bulstat {
                    "bulstatFound"
                } else {
                    "commercialFound"
                },
                serde_json::to_value(record).unwrap(),
                Value::Null,
            ),
            Err(LookupError::NotFound) => ("notFound", Value::Null, Value::Null),
            Err(error) => (
                super::probe::error_kind(error),
                Value::Null,
                json!(error.to_string()),
            ),
        };
        let failed = result.is_err() && !matches!(result, Err(LookupError::NotFound));
        let cooldown = if failed {
            let minimum = chrono::Utc::now() + chrono::Duration::seconds(60);
            let retry = match &result {
                Err(LookupError::Registry {
                    error:
                        RegistryError::RateLimited { retry_after }
                        | RegistryError::ServiceUnavailable { retry_after, .. },
                    ..
                }) => retry_after.as_ref(),
                _ => None,
            };
            Some(match retry {
                Some(RetryAfter::Seconds(n)) => minimum.max(
                    chrono::Utc::now() + chrono::Duration::seconds((*n).min(315_360_000) as i64),
                ),
                Some(RetryAfter::Date(date)) => minimum.max(*date),
                None => minimum,
            })
        } else {
            None
        };
        write_json(
            &path,
            &json!({"schemaVersion":1,"taxId":id,"checkedAt":start,"elapsedMs":elapsed,"outcome":outcome,"record":record,"error":error,"requests":requests,"resumeNotBefore":cooldown}),
        );
        write_json(
            &state_path,
            &json!({"nextStartAt":chrono::Utc::now()+chrono::Duration::seconds(30),"cooldownUntil":cooldown}),
        );
        eprintln!("{id}: {outcome} ({elapsed} ms)");
        if failed {
            break;
        }
    }
}

fn replay(row: &Value) -> Result<Value, String> {
    let id = BgBusinessId::parse(row["taxId"].as_str().ok_or("missing ID")?)
        .map_err(|e| e.to_string())?;
    let requests = row["requests"].as_array().ok_or("missing requests")?;
    if requests.len() != 3 {
        return Err("expected CR miss, BULSTAT search and detail".into());
    }
    let mut bodies = Vec::new();
    for (i, request) in requests.iter().enumerate() {
        let provider = if i == 0 {
            "BulgarianCommercialRegister"
        } else {
            "Bulstat"
        };
        if request["provider"] != provider
            || request["trace"]["httpStatus"] != 200
            || !request["error"].is_null()
        {
            return Err("unexpected provider/status/error".into());
        }
        let body: Vec<u8> =
            serde_json::from_value(request["body"].clone()).map_err(|e| e.to_string())?;
        let content_type = request["contentType"]
            .as_str()
            .ok_or("missing content type")?;
        if i == 0 {
            if !body.is_empty() || !matches!(content_type, "" | "application/json") {
                return Err("not a definitive CR miss".into());
            }
        } else if content_type != "text/html" {
            return Err("not HTML".into());
        }
        bodies.push(body);
    }
    let name = bulstat::search(&bodies[1], &id)
        .map_err(|e| e.to_string())?
        .ok_or("empty search")?;
    let record = bulstat::detail(&bodies[2], &id, name).map_err(|e| e.to_string())?;
    serde_json::to_value(record).map_err(|e| e.to_string())
}

#[test]
#[ignore = "offline replay of a local BULSTAT batch; no network"]
fn replay_bulstat_batch() {
    let dir = PathBuf::from(std::env::var("BULSTAT_RUN_DIR").unwrap());
    let manifest = read(dir.join("manifest.json"));
    let reviews = dir.join("review.json");
    let reviews = if reviews.exists() {
        read(reviews)
    } else {
        json!({})
    };
    let mut rows = Vec::new();
    for id in manifest["ids"].as_array().unwrap() {
        let path = dir
            .join("observations")
            .join(format!("{}.json", id.as_str().unwrap()));
        if !path.exists() {
            continue;
        }
        let row = read(&path);
        let result = replay(&row);
        let digest = source_sha(&fs::read(path).unwrap());
        let review = &reviews[id.as_str().unwrap()];
        let matches_review = if review.is_null() {
            None
        } else {
            assert_eq!(review["observationSha256"], digest, "review source changed");
            let expected = review["expected"].as_object().expect("review expectations");
            assert!(!expected.is_empty());
            Some(result.as_ref().is_ok_and(|record| {
                expected.iter().all(|(path, value)| {
                    let found = path.split('.').try_fold(record, |r, key| r.get(key));
                    found == Some(value)
                })
            }))
        };
        rows.push(json!({"taxId":id,"observationSha256":digest,"record":result.as_ref().ok(),"error":result.as_ref().err(),"matchesLive":result.as_ref().is_ok_and(|r| *r == row["record"]),"matchesReview":matches_review}));
    }
    assert!(!rows.is_empty());
    write_json(
        &dir.join("replay.json"),
        &json!({"schemaVersion":1,"generatedAt":chrono::Utc::now(),"binarySha256":source_sha(&fs::read(std::env::current_exe().unwrap()).unwrap()),"rows":rows}),
    );
    assert!(
        rows.iter().all(|r| r["error"].is_null()
            && r["matchesReview"]
                .as_bool()
                .unwrap_or(r["matchesLive"] == true)),
        "replay failures; see replay.json"
    );
}

#[test]
fn replay_requires_the_complete_successful_fallback_chain() {
    for row in [
        json!({"taxId":"000000001","requests":[]}),
        json!({"taxId":"000000001","requests":[{}, {}, {}]}),
    ] {
        assert!(replay(&row).is_err());
    }
}

#[test]
fn replay_preserves_branch_identity_and_rejects_detail_mismatch() {
    let id = "0000000010001";
    let header = "<div>РЕГИСТЪР БУЛСТАТ - СПРАВКИ</div><div>Справка за субект на БУЛСТАТ</div>";
    let search = format!(
        "{header}<table><tr><td><a href='https://reports.bulstat.bg/bulstat-ireports/preview?__report=https%3A%2F%2Freports.bulstat.bg%2Fbulstat-ireports%2Fv_rpt1.rptdesign&amp;uic={id}'>{id}</a></td><td>Пример</td></tr></table>"
    );
    let detail = format!(
        "{header}<table id='tab1'><tr><td>Код по БУЛСТАТ</td><td>{id}</td><td></td><td></td></tr><tr><td>Нефизическо лице</td><td>Пример, правна форма: Висше училище</td><td></td><td></td></tr></table>"
    );
    let request = |provider: &str, body: &str, content_type: &str| json!({"provider":provider,"body":body.as_bytes(),"contentType":content_type,"trace":{"httpStatus":200},"error":null});
    let mut row = json!({"taxId":id,"requests":[request("BulgarianCommercialRegister","","application/json"),request("Bulstat",&search,"text/html"),request("Bulstat",&detail,"text/html")]});
    let record = replay(&row).unwrap();
    assert_eq!(record["taxId"], id);
    assert_eq!(record["source"], "Bulstat");
    row["requests"][2]["body"] = json!(detail.replace(id, "0000000010002").as_bytes());
    assert!(replay(&row).is_err());
    row["requests"][0]["body"] = json!(b"null".to_vec());
    assert!(replay(&row).is_err());
}
